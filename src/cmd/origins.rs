use anyhow::Result;

use crate::graphiti::ListOriginsRequest;
use crate::groups::GroupsDb;
use crate::OriginsCommand;

pub async fn run(
    command: Option<OriginsCommand>,
    include_stats: bool,
    filter: Option<String>,
) -> Result<()> {
    if let Some(OriginsCommand::Set {
        origin,
        name,
        description,
    }) = command
    {
        let db = GroupsDb::open()?;
        db.upsert(&origin, name.as_deref(), description.as_deref())?;
        println!("Origin '{origin}' updated.");
        return Ok(());
    }

    let mut client = super::client::connect().await?;
    let graph_origins = client
        .list_origins(ListOriginsRequest { include_stats })
        .await?;

    let local_groups = GroupsDb::open()
        .and_then(|db| db.list())
        .unwrap_or_default();

    let local_map: std::collections::HashMap<&str, &crate::groups::GroupMeta> = local_groups
        .iter()
        .map(|g| (g.group_id.as_str(), g))
        .collect();

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for oi in &graph_origins.groups {
        if let Some(ref f) = filter
            && !oi.origin.contains(f.as_str())
        {
            continue;
        }
        seen.insert(oi.origin.clone());

        let meta = local_map.get(oi.origin.as_str());
        let display_name = meta.and_then(|m| m.name.as_deref());
        let desc = meta.and_then(|m| m.description.as_deref());

        print!("{}", oi.origin);
        if let Some(name) = display_name
            && name != oi.origin
        {
            print!(" ({name})");
        }
        if let Some(d) = desc {
            print!(" — {d}");
        }
        if include_stats {
            print!(
                " [entities: {}, episodes: {}",
                oi.entity_count.unwrap_or(0),
                oi.episode_count.unwrap_or(0),
            );
            if let Some(ref last) = oi.last_activity {
                print!(", last: {last}");
            }
            print!("]");
        }
        println!();
    }

    for lg in &local_groups {
        if seen.contains(&lg.group_id) {
            continue;
        }
        if let Some(ref f) = filter
            && !lg.group_id.contains(f.as_str())
        {
            continue;
        }
        print!("{}", lg.group_id);
        if let Some(ref name) = lg.name
            && name != &lg.group_id
        {
            print!(" ({name})");
        }
        if let Some(ref d) = lg.description {
            print!(" — {d}");
        }
        println!(" (no graph data)");
    }

    Ok(())
}
