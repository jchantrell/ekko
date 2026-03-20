use std::io::{self, Write};

use anyhow::{bail, Result};

use crate::graphiti::ClearGraphRequest;
use crate::project;

use super::client;

pub async fn run(group: Option<String>, yes: bool) -> Result<()> {
    let group = match group {
        Some(g) => g,
        None => {
            let cwd = std::env::current_dir()?;
            project::detect_group_id(&cwd)
                .ok_or_else(|| anyhow::anyhow!("could not detect project from cwd — use --group"))?
        }
    };
    if !yes {
        print!("This will permanently delete all memories for group '{group}'. Continue? [y/N] ");
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
            group_ids: Some(vec![group]),
        })
        .await?;

    println!("{}", resp.message);
    Ok(())
}
