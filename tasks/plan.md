# Plan: Remove group_id partitioning, keep as origin metadata

## Goal

Transform `group_id` from a **partitioning mechanism** (filters all queries) into **origin metadata** (stored on write, displayed on read, never filters search). All searches hit the full graph.

## Rename: group_id → origin

To make the semantic shift clear in code and UI, rename `group_id` to `origin` everywhere. This prevents confusion where users/agents think they're still scoping queries.

## Dependency Graph

```
types.rs (foundation — all other code imports these)
    ↓
shim/handlers.py (bridge — translates types to graphiti calls)
    ↓
graphiti client (src/graphiti/direct.rs, client.rs — serializes types to JSONL)
    ↓
MCP server (src/mcp/server.rs — exposes tools to agents)
CLI commands (src/cmd/*.rs — exposes to humans)
    ↓
session pipeline (src/session/ — writes origin on ingest)
```

## Key Decisions

1. **Rename `group_id` → `origin`** in all user-facing surfaces (CLI flags, MCP params, output). Internal plumbing follows.
2. **Search queries pass no origin filter** — `group_ids` field removed from search/episodes requests.
3. **Write path still sets origin** — auto-detected from cwd, passed to graphiti's `group_id` field (graphiti stores it on nodes).
4. **`clear` retains origin filter** — you can still wipe "all memories from project X".
5. **`groups` tool → `origins` tool** — read-only discovery of known origins.
6. **`set_group` → `set_origin`** — label an origin with name/description.
7. **Queue stays per-origin** — no functional change, just rename in display.
8. **`sync --group` → `sync --origin`** — filters which sessions to index (write-side), not search results.
9. **Display origin in `entities` output** — already does this, keep it.
10. **`project.rs` stays** — still needed to auto-detect origin from cwd.
11. **`groups.rs` (SQLite) stays** — stores origin display names. Rename internally.

## What Gets Deleted

- `group_ids` field from `SearchNodesRequest`, `SearchFactsRequest`, `GetEpisodesRequest`
- `group_ids()` and `resolve_group_id()` helpers in MCP server (replaced by simpler `resolve_origin()` for write path only)
- The `group_id` parameter from `recall`, `entities`, `episodes` MCP tools (search is always global)
- The `--group` flag from `ask`, `nodes`, `episodes` CLI commands (search is always global)

## What Gets Kept (renamed)

- `group_id` field on `AddMemoryRequest` → `origin` (still passed to graphiti as `group_id` in JSONL since graphiti uses that field name)
- `group_id` on `Node` and `Episode` response types → `origin` (display metadata)
- `--group` on `add` CLI → `--origin` (controls what origin is written)
- `group_id` param on `remember` MCP tool → `origin` (controls what origin is written)
- `clear` command keeps origin-based filtering
- `groups`/`set_group` commands/tools renamed to `origins`/`set_origin`
- `QueueGroup.group_id` → `QueueGroup.origin`
- `GroupInfo.group_id` → `OriginInfo.origin`
- Session pipeline still detects and writes origin

## Slicing Strategy

Vertical slices, each independently compilable and testable:

1. **Types + shim** — Foundation change. Rename types, remove search filters, update shim.
2. **MCP server** — Remove search-side group params, rename write-side to origin.
3. **CLI commands** — Remove search-side `--group`, rename write-side to `--origin`.
4. **Session pipeline** — Rename group_id to origin in parsers/db.
5. **Cleanup** — Rename `groups.rs` → metadata store, update README/docs.
