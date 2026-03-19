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
    ensure_ollama_models()?;

    let config = Config::default();
    config.save().context("failed to save config")?;

    println!("\nConfig written to {}", Config::config_path()?.display());
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
    std::fs::write(&compose_path, compose_content())
        .context("failed to write docker-compose.yml")?;

    let graphiti_config_path = data_dir.join("graphiti-config.yaml");
    std::fs::write(&graphiti_config_path, graphiti_config_content())
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

fn ensure_ollama_models() -> Result<()> {
    if !which("ollama") {
        println!(
            "\n  WARNING: Ollama not found. Install it: https://ollama.com/install.sh\n  \
             Graphiti needs an LLM and embedding model to function."
        );
        return Ok(());
    }

    let models = ["nomic-embed-text", "llama3.2:3b"];
    for model in models {
        println!("  Ensuring Ollama model: {model}");
        let output = Command::new("ollama")
            .args(["pull", model])
            .output()
            .with_context(|| format!("failed to pull {model}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("  WARNING: Failed to pull {model}: {stderr}");
        }
    }

    Ok(())
}

fn which(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn graphiti_config_content() -> &'static str {
    r#"llm:
  provider: openai
  model: llama3.2:3b
  providers:
    openai:
      api_key: ollama
      api_url: http://host.docker.internal:11434/v1

embedder:
  provider: openai
  model: nomic-embed-text
  dimensions: 768
  providers:
    openai:
      api_key: ollama
      api_url: http://host.docker.internal:11434/v1

database:
  provider: falkordb
  falkordb:
    uri: redis://falkordb:6379
    database: default_db
"#
}

fn compose_content() -> &'static str {
    r#"services:
  falkordb:
    image: falkordb/falkordb:latest
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
    image: zepai/knowledge-graph-mcp:standalone
    ports:
      - "8000:8000"
    environment:
      - OPENAI_API_KEY=ollama
      - FALKORDB_URI=redis://falkordb:6379
      - FALKORDB_DATABASE=default_db
      - MODEL_NAME=llama3.2:3b
      - SEMAPHORE_LIMIT=10
    volumes:
      - ./graphiti-config.yaml:/app/mcp/config/config.yaml:ro
    depends_on:
      falkordb:
        condition: service_healthy
    extra_hosts:
      - "host.docker.internal:host-gateway"

volumes:
  falkordb_data:
"#
}
