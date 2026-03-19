use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::config::Config;

pub async fn run() -> Result<()> {
    if Config::exists() {
        println!("ekko is already initialized. Run `ekko doctor` to check health.");
        return Ok(());
    }

    let runtime = container_runtime().ok_or_else(|| {
        anyhow::anyhow!(
            "No container runtime found.\n\
             Install Docker: https://docs.docker.com/get-docker/\n\
             Install Podman: https://podman.io/docs/installation"
        )
    })?;

    init_container(runtime).await?;

    let config = Config::default();
    config.save().context("failed to save config")?;
    println!("\nConfig written to {}", Config::config_path()?.display());

    ensure_ollama_models();

    println!("\nRun `ekko doctor` to verify everything is working.");
    Ok(())
}

/// Returns the container runtime command ("docker" or "podman"), if available.
fn container_runtime() -> Option<&'static str> {
    if which("docker") {
        Some("docker")
    } else if which("podman") {
        Some("podman")
    } else {
        None
    }
}

async fn init_container(runtime: &str) -> Result<()> {
    println!("Setting up Graphiti via {runtime}...");

    let data_dir = Config::data_dir()?;
    std::fs::create_dir_all(&data_dir)?;

    let compose_path = data_dir.join("docker-compose.yml");
    std::fs::write(&compose_path, compose_content(runtime))
        .context("failed to write docker-compose.yml")?;

    let graphiti_config_path = data_dir.join("graphiti-config.yaml");
    std::fs::write(&graphiti_config_path, graphiti_config_content(runtime))
        .context("failed to write graphiti-config.yaml")?;

    println!("  Pulling images...");
    let output = Command::new(runtime)
        .current_dir(&data_dir)
        .args(["compose", "pull"])
        .output()
        .with_context(|| format!("failed to run {runtime} compose pull"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{runtime} compose pull failed: {stderr}");
    }

    println!("  Starting services...");
    let output = Command::new(runtime)
        .current_dir(&data_dir)
        .args(["compose", "up", "-d"])
        .output()
        .with_context(|| format!("failed to run {runtime} compose up"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{runtime} compose up failed: {stderr}");
    }

    println!("  Graphiti + FalkorDB started.");
    Ok(())
}

fn ensure_ollama_models() {
    if !which("ollama") {
        println!(
            "\n  WARNING: Ollama not found. Install it: https://ollama.com/install.sh\n  \
             Graphiti needs an LLM and embedding model to function."
        );
        return;
    }

    let models = ["nomic-embed-text", "llama3.2:3b"];
    for model in models {
        if has_ollama_model(model) {
            println!("  Ollama model ready: {model}");
        } else {
            println!("  Pulling Ollama model: {model} (this may take a while)...");
            match Command::new("ollama").args(["pull", model]).spawn() {
                Ok(mut child) => {
                    // Wait for the pull to complete
                    match child.wait() {
                        Ok(status) if status.success() => {
                            println!("  Ollama model ready: {model}");
                        }
                        Ok(_) => println!("  WARNING: Failed to pull {model}"),
                        Err(e) => println!("  WARNING: Failed to pull {model}: {e}"),
                    }
                }
                Err(e) => println!("  WARNING: Failed to pull {model}: {e}"),
            }
        }
    }
}

fn has_ollama_model(model: &str) -> bool {
    let output = Command::new("ollama").args(["list"]).output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let base_name = model.split(':').next().unwrap_or(model);
            stdout.lines().any(|line| line.contains(base_name))
        }
        Err(_) => false,
    }
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn graphiti_config_content(runtime: &str) -> String {
    let ollama_host = if runtime == "podman" {
        "host.containers.internal"
    } else {
        "host.docker.internal"
    };

    format!(
        r#"llm:
  provider: openai
  model: llama3.2:3b
  providers:
    openai:
      api_key: ollama
      api_url: http://{ollama_host}:11434/v1

embedder:
  provider: openai
  model: nomic-embed-text
  dimensions: 768
  providers:
    openai:
      api_key: ollama
      api_url: http://{ollama_host}:11434/v1

database:
  provider: falkordb
  falkordb:
    uri: redis://falkordb:6379
    database: default_db
"#
    )
}

fn compose_content(runtime: &str) -> String {
    let ollama_host = if runtime == "podman" {
        "host.containers.internal"
    } else {
        "host.docker.internal"
    };

    // For docker, we need extra_hosts to resolve the alias.
    // Podman resolves host.containers.internal automatically.
    let extra_hosts = if runtime == "docker" {
        "\n    extra_hosts:\n      - \"host.docker.internal:host-gateway\""
    } else {
        ""
    };

    format!(
        r#"services:
  falkordb:
    image: docker.io/falkordb/falkordb:latest
    ports:
      - "6379:6379"
      - "3000:3000"
    volumes:
      - falkordb_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "-p", "6379", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 10s

  graphiti:
    image: docker.io/zepai/knowledge-graph-mcp:standalone
    ports:
      - "8000:8000"
    environment:
      - OPENAI_API_KEY=ollama
      - OPENAI_BASE_URL=http://{ollama_host}:11434/v1
      - FALKORDB_URI=redis://falkordb:6379
      - FALKORDB_DATABASE=default_db
      - MODEL_NAME=llama3.2:3b
      - EMBEDDING_MODEL=nomic-embed-text
      - SEMAPHORE_LIMIT=10
    volumes:
      - ./graphiti-config.yaml:/app/mcp/config/config.yaml:ro
    depends_on:
      falkordb:
        condition: service_healthy{extra_hosts}

volumes:
  falkordb_data:
"#
    )
}
