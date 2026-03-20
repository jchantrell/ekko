use anyhow::Result;

use super::client;

pub async fn run() -> Result<()> {
    let mut client = client::connect().await?;

    let resp = client.queue_status().await?;

    if resp.groups.is_empty() {
        println!("Queue is empty.");
        return Ok(());
    }

    for group in &resp.groups {
        let status = match &group.processing {
            Some(name) => format!("processing: {name}"),
            None => "idle".into(),
        };
        println!("{} — {status}, {} pending", group.group_id, group.pending);
    }

    Ok(())
}
