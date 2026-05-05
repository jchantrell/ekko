# Tasks: Remove group_id partitioning

## Phase 1: Types + Shim (foundation)

- [ ] **1.1** Remove `group_ids` from `SearchNodesRequest`, `SearchFactsRequest`, `GetEpisodesRequest`
  - File: `src/graphiti/types.rs`
  - Keep `group_id` on `AddMemoryRequest` (rename field to still serialize as `group_id` for graphiti compat)
  - Keep `group_ids` on `ClearGraphRequest` (clear-by-origin stays)
  - Rename `group_id` → `origin` on `Node`, `Episode`, `QueueGroup`, `GroupInfo` response types
  - Rename `ListGroupsRequest` → `ListOriginsRequest`, `ListGroupsResponse` → `ListOriginsResponse`, `GroupInfo` → `OriginInfo`
  - **Verify**: `cargo check` passes

- [ ] **1.2** Update shim `handlers.py` — remove `group_ids` from search/episodes handlers
  - `do_search_facts`: remove `group_ids` param, pass empty list `[]` to graphiti (searches all)
  - `do_search_nodes`: same
  - `do_get_episodes`: remove group_ids filter, fetch all recent episodes
  - `do_list_groups` → `do_list_origins`: no functional change (still queries distinct group_ids from neo4j)
  - Keep `group_id` on `do_add_memory` (origin is written)
  - Keep `group_ids` on `do_clear_graph` (clear-by-origin stays)
  - **Verify**: python syntax check passes

- [ ] **1.3** Update graphiti client methods to match new request types
  - File: `src/graphiti/client.rs`, `src/graphiti/direct.rs`
  - Rename `list_groups` → `list_origins`
  - **Verify**: `cargo check` passes

### Checkpoint: `cargo build` succeeds, shim syntax valid

## Phase 2: MCP Server

- [ ] **2.1** Update MCP tool params — remove search-side group_id, rename write-side to origin
  - `RecallParams`: remove `group_id` field entirely
  - `EntitiesParams`: remove `group_id` field entirely
  - `EpisodesParams`: remove `group_id` field entirely
  - `RememberParams`: rename `group_id` → `origin`
  - `GroupsParams` → keep as-is (tool renamed)
  - `SetGroupParams` → `SetOriginParams`: rename `group_id` → `origin`
  - **Verify**: `cargo check` passes

- [ ] **2.2** Update MCP tool implementations
  - `recall()`: remove `group_ids` from search requests (pass `None`)
  - `entities()`: same
  - `episodes()`: same
  - `remember()`: use `params.origin` instead of `params.group_id`
  - Remove `group_ids()` helper method
  - Simplify `resolve_group_id()` → `resolve_origin()` (only used by remember/clear)
  - `groups()` tool → rename to `origins`, update description
  - `set_group()` tool → rename to `set_origin`, update description
  - `status()`: rename "Project scope" display to "Origin"
  - Update `get_info()` instructions text
  - Rename `EkkoServer.group_id` → `EkkoServer.origin`
  - **Verify**: `cargo build` passes

### Checkpoint: `cargo build` succeeds, MCP tools have correct signatures

## Phase 3: CLI Commands

- [ ] **3.1** Update `main.rs` CLI arg definitions
  - `Add`: rename `--group` → `--origin`, update help text
  - `Ask`: remove `--group` entirely
  - `Nodes`: remove `--group` entirely
  - `Episodes`: remove `--group` entirely
  - `Clear`: rename `group` positional → `origin`
  - `Groups` command → `Origins` command
  - `GroupsCommand::Set` → rename `group_id` → `origin`
  - `Sync`: rename `--group` → `--origin`
  - **Verify**: `cargo check` passes

- [ ] **3.2** Update CLI command implementations
  - `cmd/add.rs`: use `origin` param, detect from cwd for write
  - `cmd/ask.rs`: remove group_ids entirely from search requests
  - `cmd/nodes.rs`: same
  - `cmd/episodes.rs`: same
  - `cmd/clear.rs`: rename to use `origin` terminology
  - `cmd/groups.rs` → `cmd/origins.rs`: rename file, update function
  - `cmd/sync.rs`: rename `group_filter` → `origin_filter`
  - `cmd/queue.rs`: update display to say "origin" instead of "group"
  - **Verify**: `cargo build` passes, `ekko --help` shows correct flags

### Checkpoint: Full build passes, CLI works

## Phase 4: Session Pipeline

- [ ] **4.1** Rename in session types and parsers
  - `session/normalize.rs`: `ParsedSession.group_id` → `ParsedSession.origin`
  - `session/normalize.rs`: `SyncOptions.group_filter` → `SyncOptions.origin_filter`
  - `session/parsers/claude_code.rs`: rename group_id variable → origin
  - `session/parsers/opencode.rs`: same
  - `session/summarize.rs`: rename in prompt context
  - **Verify**: `cargo check` passes

- [ ] **4.2** Update session index DB
  - `session/db.rs`: rename column in schema (or keep for compat, just rename in code)
  - `session/mod.rs`: rename all `group_id` → `origin` in ingest logic
  - **Verify**: `cargo build` passes

### Checkpoint: Full build, session sync still works

## Phase 5: Cleanup + Docs

- [ ] **5.1** Rename `groups.rs` internals
  - `GroupMeta.group_id` → `OriginMeta.origin`
  - `GroupsDb` → `OriginsDb`
  - Keep SQLite schema as-is (migration not worth it for local metadata)
  - **Verify**: `cargo build` passes

- [ ] **5.2** Update `project.rs`
  - Rename `detect_group_id()` → `detect_origin()`
  - **Verify**: `cargo build` passes

- [ ] **5.3** Update README.md and MCP tool descriptions
  - Replace all "group" language with "origin"
  - Update CLI examples
  - Update AGENTS.md example section
  - **Verify**: docs are consistent

- [ ] **5.4** Final verification
  - `cargo clippy` — no new warnings from our changes
  - `cargo build --release` — release build succeeds
  - Manual test: `ekko add "test"`, `ekko ask "test"` (searches full graph)
