use std::io::{self, Write};

use anyhow::{bail, Result};

use crate::graphiti::ClearGraphRequest;
use crate::project;

use super::client;

pub async fn run(group: String, yes: bool) -> Result<()> {
    let group = project::sanitize_group_id(group);
    if !yes {
        print!("This will permanently delete all memories for group '{group}'. Continue? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            bail!("Aborted.");
        }
    }

    let client = client::connect().await?;

    let resp = client
        .clear_graph(ClearGraphRequest {
            group_ids: Some(vec![group]),
        })
        .await?;

    println!("{}", resp.message);
    Ok(())
}
