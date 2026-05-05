use anyhow::Result;

use crate::graphiti::ListGroupsRequest;
use crate::groups::GroupsDb;
use crate::GroupsCommand;

pub async fn run(
    command: Option<GroupsCommand>,
    include_stats: bool,
    filter: Option<String>,
) -> Result<()> {
    if let Some(GroupsCommand::Set {
        group_id,
        name,
        description,
    }) = command
    {
        let db = GroupsDb::open()?;
        db.upsert(&group_id, name.as_deref(), description.as_deref())?;
        println!("Group '{group_id}' updated.");
        return Ok(());
    }

    let mut client = super::client::connect().await?;
    let graph_groups = client
        .list_groups(ListGroupsRequest { include_stats })
        .await?;

    let local_groups = GroupsDb::open()
        .and_then(|db| db.list())
        .unwrap_or_default();

    let local_map: std::collections::HashMap<&str, &crate::groups::GroupMeta> = local_groups
        .iter()
        .map(|g| (g.group_id.as_str(), g))
        .collect();

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for gi in &graph_groups.groups {
        if let Some(ref f) = filter {
            if !gi.group_id.contains(f.as_str()) {
                continue;
            }
        }
        seen.insert(gi.group_id.clone());

        let meta = local_map.get(gi.group_id.as_str());
        let display_name = meta.and_then(|m| m.name.as_deref());
        let desc = meta.and_then(|m| m.description.as_deref());

        print!("{}", gi.group_id);
        if let Some(name) = display_name {
            if name != gi.group_id {
                print!(" ({name})");
            }
        }
        if let Some(d) = desc {
            print!(" — {d}");
        }
        if include_stats {
            print!(
                " [entities: {}, episodes: {}",
                gi.entity_count.unwrap_or(0),
                gi.episode_count.unwrap_or(0),
            );
            if let Some(ref last) = gi.last_activity {
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
        if let Some(ref f) = filter {
            if !lg.group_id.contains(f.as_str()) {
                continue;
            }
        }
        print!("{}", lg.group_id);
        if let Some(ref name) = lg.name {
            if name != &lg.group_id {
                print!(" ({name})");
            }
        }
        if let Some(ref d) = lg.description {
            print!(" — {d}");
        }
        println!(" (no graph data)");
    }

    Ok(())
}
