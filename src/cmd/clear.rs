use std::io::{self, Write};

use anyhow::{bail, Result};

use crate::graphiti::ClearGraphRequest;
use crate::project;

use super::client;

pub async fn run(origin: Option<String>, yes: bool) -> Result<()> {
    let origin = match origin {
        Some(o) => o,
        None => {
            let cwd = std::env::current_dir()?;
            project::detect_origin(&cwd)
                .ok_or_else(|| anyhow::anyhow!("could not detect project from cwd — use --origin"))?
        }
    };
    if !yes {
        print!("This will permanently delete all memories for origin '{origin}'. Continue? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            bail!("Aborted.");
        }
    }

    let mut client = client::connect().await?;

    let resp = client
        .clear_graph(ClearGraphRequest {
            origins: Some(vec![origin]),
        })
        .await?;

    println!("{}", resp.message);
    Ok(())
}
