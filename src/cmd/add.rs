use anyhow::Result;
use chrono::Utc;

use crate::graphiti::AddMemoryRequest;
use crate::project;

use super::client;

pub async fn run(text: String, group: Option<String>, name: Option<String>, source: Option<String>) -> Result<()> {
    let mut client = client::connect().await?;

    let group_id = group
        .map(project::sanitize_group_id)
        .or_else(|| project::detect_group_id(&std::env::current_dir().ok()?));
    let name = name.unwrap_or_else(|| Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string());

    let req = AddMemoryRequest {
        name,
        episode_body: text,
        group_id,
        source: source.unwrap_or_else(|| "cli".into()),
        source_description: "ekko cli".into(),
        uuid: None,
        sync: true,
        reference_time: None,
    };

    let resp = client.add_memory(req).await?;
    println!("{}", resp.message);
    Ok(())
}
