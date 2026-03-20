use anyhow::Result;
use std::process::Command;

use crate::config::Config;

pub async fn run() -> Result<()> {
    println!("ekko doctor\n");

    let mut all_ok = true;

    // 1. Config
    check("Config file", Config::exists(), &mut all_ok);

    let config = Config::load()?;

    // 2. Ollama
    let ollama_ok = check_command("ollama", &["--version"]);
    check("Ollama installed", ollama_ok, &mut all_ok);

    if ollama_ok {
        let ollama_running = check_ollama_running().await;
        check("Ollama running", ollama_running, &mut all_ok);

        if ollama_running {
            let has_embed = check_ollama_model(&config.graphiti.embedder.model);
            check(
                &format!("Ollama model: {}", config.graphiti.embedder.model),
                has_embed,
                &mut all_ok,
            );

            let has_llm = check_ollama_model(&config.graphiti.llm.model);
            check(
                &format!("Ollama model: {}", config.graphiti.llm.model),
                has_llm,
                &mut all_ok,
            );
        }
    }

    // 3. Python venv
    let python_path = Config::python_path()?;
    let venv_ok = python_path.exists();
    check(
        &format!("Python venv: {}", python_path.display()),
        venv_ok,
        &mut all_ok,
    );

    if venv_ok {
        let graphiti_ok = check_graphiti_core(&python_path);
        check("graphiti-core installed", graphiti_ok, &mut all_ok);
    }

    // 4. Shim
    let shim_path = Config::shim_path()?;
    check(
        &format!("Shim script: {}", shim_path.display()),
        shim_path.exists(),
        &mut all_ok,
    );

    // 5. Graph DB
    match config.graphiti.database.provider.as_str() {
        "falkordb" => {
            let falkor_ok = check_tcp("localhost", 6379).await;
            check("FalkorDB reachable (port 6379)", falkor_ok, &mut all_ok);
        }
        "neo4j" => {
            let neo4j_ok = check_tcp("localhost", 7687).await;
            check("Neo4j reachable (port 7687)", neo4j_ok, &mut all_ok);
        }
        other => {
            println!("  ? Unknown database provider: {other}");
        }
    }

    // 6. End-to-end shim check (only if all prerequisites pass)
    if all_ok {
        match crate::graphiti::Client::new(&config).await {
            Ok(mut client) => {
                let healthy = client.health().await.unwrap_or(false);
                check("Shim connection", healthy, &mut all_ok);
                let _ = client.shutdown().await;
            }
            Err(e) => {
                check(&format!("Shim connection: {e}"), false, &mut all_ok);
            }
        }
    }

    println!();
    if all_ok {
        println!("All checks passed.");
    } else {
        println!("Some checks failed. Run `ekko init` to set up missing components.");
    }

    Ok(())
}

fn check(label: &str, ok: bool, all_ok: &mut bool) {
    if ok {
        println!("  + {label}");
    } else {
        println!("  x {label}");
        *all_ok = false;
    }
}

fn check_command(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn check_ollama_running() -> bool {
    reqwest::get("http://localhost:11434/api/tags")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn check_ollama_model(model: &str) -> bool {
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

fn check_graphiti_core(python: &std::path::Path) -> bool {
    Command::new(python)
        .args(["-c", "from importlib.metadata import version; print(version('graphiti-core'))"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn check_tcp(host: &str, port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("{host}:{port}"))
        .await
        .is_ok()
}
