use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::config::Config;

const SHIM_FILES: &[(&str, &str)] = &[
    ("__main__.py", include_str!("../../shim/__main__.py")),
    ("ingestion.py", include_str!("../../shim/ingestion.py")),
    ("driver.py", include_str!("../../shim/driver.py")),
    ("formatting.py", include_str!("../../shim/formatting.py")),
    ("handlers.py", include_str!("../../shim/handlers.py")),
];

pub async fn run() -> Result<()> {
    let reinit = Config::exists();

    if !reinit {
        let runtime = container_runtime().ok_or_else(|| {
            anyhow::anyhow!(
                "No container runtime found.\n\
                 Install Docker: https://docs.docker.com/get-docker/\n\
                 Install Podman: https://podman.io/docs/installation"
            )
        })?;

        init_falkordb(runtime)?;
    }

    init_python_venv()?;
    write_shim()?;

    if !reinit {
        let config = Config::default();
        config.save().context("failed to save config")?;
        println!("\nConfig written to {}", Config::config_path()?.display());
    }

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

fn init_falkordb(runtime: &str) -> Result<()> {
    println!("Setting up FalkorDB via {runtime}...");

    let data_dir = Config::data_dir()?;
    std::fs::create_dir_all(&data_dir)?;

    let compose_path = data_dir.join("docker-compose.yml");
    std::fs::write(&compose_path, compose_content())
        .context("failed to write docker-compose.yml")?;

    println!("  Pulling FalkorDB image...");
    let output = Command::new(runtime)
        .current_dir(&data_dir)
        .args(["compose", "pull"])
        .output()
        .with_context(|| format!("failed to run {runtime} compose pull"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{runtime} compose pull failed: {stderr}");
    }

    println!("  Starting FalkorDB...");
    let output = Command::new(runtime)
        .current_dir(&data_dir)
        .args(["compose", "up", "-d"])
        .output()
        .with_context(|| format!("failed to run {runtime} compose up"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{runtime} compose up failed: {stderr}");
    }

    println!("  FalkorDB started.");
    Ok(())
}

fn init_python_venv() -> Result<()> {
    let venv_dir = Config::venv_dir()?;

    if venv_dir.exists() {
        println!("  Python venv already exists at {}", venv_dir.display());
    } else {
        let python = find_python().ok_or_else(|| {
            anyhow::anyhow!(
                "Python 3 not found.\n\
                 Install Python 3.10+: https://www.python.org/downloads/"
            )
        })?;

        println!("  Creating Python venv at {}...", venv_dir.display());
        let output = Command::new(&python)
            .args(["-m", "venv", &venv_dir.to_string_lossy()])
            .output()
            .with_context(|| format!("failed to create venv with {python}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("venv creation failed: {stderr}");
        }
    }

    let pip = venv_dir.join("bin").join("pip");
    println!("  Installing graphiti-core[falkordb]...");
    let output = Command::new(&pip)
        .args(["install", "--quiet", "graphiti-core[falkordb]"])
        .output()
        .context("failed to run pip install")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("pip install failed: {stderr}");
    }

    // Verify installation
    let python_bin = Config::python_path()?;
    let output = Command::new(&python_bin)
        .args(["-c", "from importlib.metadata import version; print(version('graphiti-core'))"])
        .output()
        .context("failed to verify graphiti-core")?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("  graphiti-core {version} installed.");
    } else {
        bail!("graphiti-core installation verification failed");
    }

    Ok(())
}

fn write_shim() -> Result<()> {
    let shim_dir = Config::shim_dir()?;
    std::fs::create_dir_all(&shim_dir)?;

    for (name, content) in SHIM_FILES {
        let path = shim_dir.join(name);
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    println!("  Shim written to {}", shim_dir.display());

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
                Ok(mut child) => match child.wait() {
                    Ok(status) if status.success() => {
                        println!("  Ollama model ready: {model}");
                    }
                    Ok(_) => println!("  WARNING: Failed to pull {model}"),
                    Err(e) => println!("  WARNING: Failed to pull {model}: {e}"),
                },
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

fn find_python() -> Option<String> {
    for cmd in ["python3", "python"] {
        if which(cmd) {
            return Some(cmd.to_string());
        }
    }
    None
}

fn compose_content() -> String {
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

volumes:
  falkordb_data:
"#
    .to_string()
}
