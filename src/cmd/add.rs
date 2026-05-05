use anyhow::Result;
use chrono::Utc;

use crate::graphiti::AddMemoryRequest;
use crate::project;

use super::client;

pub async fn run(text: String, origin: Option<String>, name: Option<String>, source: Option<String>) -> Result<()> {
    let mut client = client::connect().await?;

    let origin = origin
        .or_else(|| project::detect_origin(&std::env::current_dir().ok()?));
    let name = name.unwrap_or_else(|| Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string());

    let req = AddMemoryRequest {
        name,
        episode_body: text,
        origin,
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
