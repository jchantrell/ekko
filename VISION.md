# ekko

Persistent memory for AI agents, powered by Graphiti's temporal knowledge graph.

Rust binary. MCP server. CLI. Session indexer. Daemon. One install.

```
curl -fsSL https://ekko.dev/install.sh | sh
ekko init
```

---

## The Problem

AI coding agents are goldfish. Every session starts from zero. The agent re-discovers your preferences, re-learns your codebase patterns, re-makes mistakes it already learned from.

We've experimented with building memory from scratch (hivemind, CASS, swarm-mail, pdf-brain). The lesson: the hard problems — entity extraction, temporal fact management, contradiction detection, hybrid search with reranking — are already solved by [Graphiti](https://github.com/getzep/graphiti). What's missing is the infrastructure to make it usable as an always-on memory system for coding agents.

## What ekko Is

ekko is the **framework around Graphiti** that turns it into a complete agent memory system:

1. **MCP server** — Exposes Graphiti's capabilities as tools for Claude Code, Cursor, OpenCode, and any MCP host
2. **Session indexer** — Automatically ingests agent conversation history into the knowledge graph
3. **CLI** — Inspect, search, and maintain your memory from the terminal
4. **Daemon** — Background process that multiplexes a single Graphiti connection across all consumers, with auto-start, shim respawn, and periodic sync
5. **Lifecycle manager** — Handles Neo4j setup, Python venv, Ollama models, health checks, and graceful degradation

ekko does NOT duplicate what Graphiti already does. No custom entity extraction. No custom dedup logic. No custom graph storage. Graphiti is the brain. ekko is the nervous system that connects it to the world.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      ekko (Rust binary)                         │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────────┐ │
│  │  MCP Server   │  │     CLI      │  │   Session Indexer     │ │
│  │  (STDIO/rmcp) │  │   (clap)     │  │                       │ │
│  │               │  │              │  │  Claude Code parser   │ │
│  │  7 tools:     │  │  ekko add    │  │  OpenCode parser      │ │
│  │  remember     │  │  ekko ask    │  │                       │ │
│  │  recall       │  │  ekko show   │  │  Parse → Summarize →  │ │
│  │  forget       │  │  ekko sync   │  │  Feed to Graphiti     │ │
│  │  entities     │  │  ekko daemon │  │                       │ │
│  │  episodes     │  │  ekko queue  │  │                       │ │
│  │  queue        │  │  ekko ...    │  │                       │ │
│  │  status       │  │              │  │                       │ │
│  └──────┬────────┘  └──────┬───────┘  └──────────┬────────────┘ │
│         └──────────────────┼─────────────────────┘              │
│                            │                                    │
│  ┌─────────────────────────┴──────────────────────────────────┐ │
│  │              Connection Broker (cmd/client.rs)              │ │
│  │  1. Try daemon socket → 2. Auto-start daemon → 3. Direct  │ │
│  └─────────────┬──────────────────────────┬───────────────────┘ │
│                │                          │                     │
│  ┌─────────────┴──────┐  ┌───────────────┴─────────────────┐  │
│  │   DaemonClient     │  │       DirectClient              │  │
│  │   (Unix socket)    │  │   (owns Python child process)   │  │
│  └─────────────┬──────┘  └───────────────┬─────────────────┘  │
│                │                          │                     │
│  ┌─────────────┴──────────────────────────┘                   │ │
│  │           Daemon Server (daemon/server.rs)                 │ │
│  │  • Owns single Python shim process                        │ │
│  │  • Multiplexes requests over Unix socket                  │ │
│  │  • Auto-respawns dead shim                                │ │
│  │  • Runs periodic background sync (30min)                  │ │
│  │  • Non-blocking lightweight ops (queue/health/status)     │ │
│  └─────────────┬──────────────────────────────────────────────┘ │
└────────────────┼────────────────────────────────────────────────┘
                 │ JSONL over stdin/stdout
┌────────────────┴────────────────────────────────────────────────┐
│              Python Shim (shim/*.py)                             │
│  • Imports graphiti-core directly (no HTTP server)              │
│  • QueueService for async episode processing                    │
│  • JSONL request/response protocol                              │
└────────────────┬────────────────────────────────────────────────┘
                 │ Bolt protocol
┌────────────────┴────────────────────────────────────────────────┐
│              Neo4j 5 Community (Docker/Podman)                   │
│              + Ollama (LLM + embeddings)                        │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Decision: Embedded Shim over HTTP

The original plan was to talk to Graphiti's MCP server over HTTP. The actual implementation is different and better: ekko embeds a Python shim that imports `graphiti-core` directly and communicates via JSONL over stdin/stdout. No HTTP server needed. The daemon multiplexes this single shim process across all CLI/MCP consumers via a Unix socket.

Benefits:
- No separate Python server to manage
- Single process owns the graphiti-core client
- Daemon handles connection multiplexing, auto-start, and shim respawn
- Lightweight ops (queue/health/status) use `try_lock` to avoid blocking behind heavy writes

### What Graphiti Handles (we don't rebuild this)

| Capability | How Graphiti Does It |
|---|---|
| **Entity extraction** | LLM extracts entities from text, deduplicates against existing graph |
| **Fact storage** | Relationships (edges) between entities, each carrying a `fact` string |
| **Temporal lifecycle** | Every fact has `created_at`, `valid_at`, `invalid_at`, `expired_at` |
| **Contradiction detection** | LLM compares new facts against existing ones, auto-invalidates contradicted facts |
| **Hybrid search** | Vector similarity + BM25 fulltext + graph traversal, combined via reranking |
| **Community detection** | Label propagation clusters related entities, LLM generates community summaries |
| **Episode provenance** | Raw input preserved as episodes, linked to extracted entities/facts |
| **Group isolation** | `group_id` on every node/edge for project-level scoping |

### What ekko Adds

| Capability | Why Graphiti Doesn't Do This |
|---|---|
| **MCP server for coding agents** | Graphiti has an MCP server, but its tools are generic. ekko's tools are designed for agent workflows (remember/recall/forget semantics, project-aware context). |
| **Session indexing** | Graphiti doesn't know where Claude Code or OpenCode store their sessions. ekko discovers, parses, summarizes, and feeds them as episodes. |
| **CLI** | Graphiti has no CLI. ekko provides terminal-based inspection, search, and maintenance. |
| **Daemon** | Multiplexes a single graphiti-core connection across all consumers. Auto-starts on first use, respawns crashed shims, runs periodic background sync. |
| **Lifecycle management** | `ekko init` sets up Neo4j via container runtime (docker/podman), creates a Python venv with graphiti-core, writes the shim, pulls Ollama models. `ekko doctor` diagnoses issues. |
| **Agent-specific context** | Detects current project from cwd, scopes queries to relevant `group_id`, formats results for agent consumption. |
| **Graceful degradation** | If Graphiti is down, ekko reports status clearly rather than crashing. If Ollama is down, ekko tells the agent. |

---

## Graphiti Integration

### API Surface

ekko talks to graphiti-core via an embedded Python shim over JSONL stdin/stdout. The shim exposes these operations:

| Method | Parameters | Returns | ekko Uses For |
|---|---|---|---|
| `add_memory` | `name`, `episode_body`, `group_id`, `source`, `source_description`, `sync` | Success/error | Storing memories, ingesting sessions |
| `search_nodes` | `query`, `group_ids`, `max_nodes`, `entity_types` | List of entities with summaries | Entity lookup, graph exploration |
| `search_facts` | `query`, `group_ids`, `max_facts`, `center_node_uuid` | List of facts with temporal metadata | Memory recall, context assembly |
| `get_episodes` | `group_ids`, `max_episodes` | List of episodes | Session history |
| `get_edge` | `uuid` | Single fact with full metadata | Detailed inspection |
| `delete_edge` | `uuid` | Success/error | Forgetting specific facts |
| `delete_episode` | `uuid` | Success/error | Removing ingested sessions |
| `clear_graph` | `group_ids` | Success/error | Resetting project memory |
| `health` | — | Boolean | Connection check |
| `status` | — | Health status | Health checks |
| `queue_status` | — | Processing/pending per group | Queue monitoring |

### Graphiti Client (Rust)

A typed client with two backends — `DaemonClient` (Unix socket) and `DirectClient` (owns Python child process):

```rust
pub enum Client {
    Direct(DirectClient),
    Daemon(DaemonClient),
}

impl Client {
    pub async fn add_memory(&mut self, req: AddMemoryRequest) -> Result<AddMemoryResponse>;
    pub async fn search_nodes(&mut self, req: SearchNodesRequest) -> Result<SearchNodesResponse>;
    pub async fn search_facts(&mut self, req: SearchFactsRequest) -> Result<SearchFactsResponse>;
    pub async fn get_episodes(&mut self, req: GetEpisodesRequest) -> Result<GetEpisodesResponse>;
    pub async fn get_edge(&mut self, uuid: &str) -> Result<GetEdgeResponse>;
    pub async fn delete_edge(&mut self, uuid: &str) -> Result<DeleteResponse>;
    pub async fn delete_episode(&mut self, uuid: &str) -> Result<DeleteResponse>;
    pub async fn clear_graph(&mut self, group_ids: &[String]) -> Result<DeleteResponse>;
    pub async fn health(&mut self) -> Result<bool>;
    pub async fn status(&mut self) -> Result<StatusResponse>;
    pub async fn queue_status(&mut self) -> Result<QueueStatusResponse>;
}
```

### Configuration

`~/.config/ekko/config.toml`:

```toml
[graphiti.llm]
provider = "openai"          # OpenAI-compatible (works with Ollama)
model = "qwen3:8b"
api_url = "http://localhost:11434/v1"
api_key = "ollama"

[graphiti.embedder]
provider = "openai"
model = "qwen3-embedding:8b"
dimensions = 4096
api_url = "http://localhost:11434/v1"
api_key = "ollama"

[graphiti.database]
provider = "neo4j"
uri = "bolt://localhost:7687"
user = "neo4j"
password = "ekko-memory"
```

`ekko init` generates this config and starts the required services.

---

## Session Indexing

The highest-value feature ekko adds. Agents produce rich conversation history that currently evaporates. ekko captures it.

### Agent Session Discovery

| Agent | Session Location | Format | Status |
|---|---|---|---|
| Claude Code | `~/.claude/projects/*/sessions/` | JSONL (messages with tool calls) | Implemented |
| OpenCode | `~/.local/share/opencode/sessions/` | SQLite (messages with parts) | Implemented |
| Cursor | TBD | TBD | Planned |
| Aider | `~/.aider.chat.history.md` | Markdown | Planned |
| Gemini CLI | `~/.gemini/` | JSON | Planned |

### Indexing Pipeline

```
Session file discovered
        │
        ▼
Parse agent-specific format → normalized turns
        │
        ▼
Summarize via Ollama (or extractive fallback if --no-llm)
  - 2-3 sentence summary
  - Key decisions made
  - Files changed
  - Outcome
        │
        ▼
Feed to Graphiti as episode
  - group_id = project name (from cwd or session metadata)
  - source = "text"
  - source_description = "claude-code session {id}, {date}"
        │
        ▼
Graphiti extracts entities + facts automatically
  - Technologies mentioned
  - Patterns applied
  - Decisions and their rationale
  - Bugs encountered and fixes
```

### Incremental Indexing

ekko tracks which sessions have been indexed in a local SQLite database (`~/.local/share/ekko/index.db`):

```sql
CREATE TABLE indexed_sessions (
    agent        TEXT NOT NULL,
    session_id   TEXT NOT NULL,
    source_path  TEXT NOT NULL,
    fingerprint  TEXT NOT NULL,
    group_id     TEXT NOT NULL,
    turn_count   INTEGER NOT NULL,
    indexed_at   TEXT NOT NULL,
    PRIMARY KEY (agent, session_id)
);
```

`ekko sync` checks for new/modified sessions and indexes only what's changed. Scoped to the current project by default; use `--all` for everything.

---

## MCP Server

ekko's MCP server is the primary integration point for AI agents. It wraps Graphiti's capabilities with agent-friendly semantics.

### Tools

| Tool | Description | Maps To |
|---|---|---|
| `remember` | Store a memory. Accepts freeform text + optional group_id. | `add_memory` |
| `recall` | Search for relevant memories. Returns facts + entities. | `search_facts` + `search_nodes` |
| `forget` | Delete a specific fact by UUID. | `delete_edge` |
| `entities` | Search for entities in the knowledge graph. | `search_nodes` |
| `episodes` | List recent episodes/sessions. | `get_episodes` |
| `queue` | Show the memory processing queue. | `queue_status` |
| `status` | Health check — is Graphiti reachable? | `health` + `status` |

### Tool Design Principles

- **Project-aware by default**: If the agent is working in `~/Workspace/ekko`, queries are automatically scoped to `group_id = "ekko"`. No need to specify project every time.
- **Thin wrappers**: Tools add minimal logic on top of Graphiti. No re-ranking, no custom scoring. Let Graphiti do its job.
- **Structured results**: Return facts with temporal metadata (`valid_at`, `invalid_at`) so the agent can reason about currency.
- **Fail clearly**: If Graphiti is down, return a clear error message, not a crash.

### STDIO Transport

The MCP server uses STDIO transport (stdin/stdout JSON-RPC). The host (Claude Code, OpenCode, etc.) spawns `ekko serve` as a child process.

```json
{
  "mcpServers": {
    "ekko": {
      "command": "ekko",
      "args": ["serve"]
    }
  }
}
```

Startup target: <100ms (Rust binary, no Python in the critical path — the daemon handles the shim separately).

---

## CLI

```bash
# Setup
ekko init                        # Set up Neo4j + Python venv + shim + Ollama models
ekko doctor                      # Health check (Neo4j, Ollama, shim)

# Memory
ekko add "Joel prefers bun"                    # Store a memory
ekko add "Switched to pnpm" --group ekko       # Store with project scope
ekko ask "what package manager"                # Search facts
ekko ask "what do we know about libSQL" --nodes # Include entity results
ekko show <uuid>                               # Inspect a specific fact
ekko rm fact <uuid>                            # Delete a fact
ekko rm episode <uuid>                         # Delete an episode

# Graph exploration
ekko nodes                       # List entities
ekko nodes --type technology     # Filter by type
ekko nodes "libSQL"              # Search entities by name

# Sessions
ekko sync                        # Index new sessions for current project
ekko sync --all                  # Index all projects
ekko sync --agent claude-code    # Index only Claude Code sessions
ekko sync --dry-run              # Show what would be indexed
ekko sync --no-llm               # Skip LLM summarization
ekko sync --full                 # Re-index (ignore fingerprints)
ekko episodes                    # List recent episodes
ekko episodes --group ekko       # Episodes for a project

# Daemon
ekko daemon start                # Start the daemon (foreground)
ekko daemon stop                 # Stop the running daemon
ekko daemon status               # Show daemon status

# Monitoring
ekko status                      # Graphiti connection status
ekko queue                       # Memory processing queue

# Maintenance
ekko clear                       # Wipe current project's memory
ekko clear --yes                 # Skip confirmation
ekko update                      # Self-update from GitHub Releases
ekko update --check              # Check for updates without installing
```

---

## Setup & Installation

### Prerequisites

- **Ollama** — For local LLM and embeddings. `curl -fsSL https://ollama.com/install.sh | sh`
- **Docker or Podman** — For running Neo4j
- **Python 3.10+** — For the graphiti-core shim

### Install ekko

```bash
# Download the binary
curl -fsSL https://ekko.dev/install.sh | sh

# Or build from source
cargo install --path .
```

### Initialize

```bash
ekko init
```

This:
1. Detects container runtime (docker or podman)
2. Pulls Neo4j 5 Community image and starts it via compose
3. Creates a Python venv at `~/.local/share/ekko/venv/`
4. Installs `graphiti-core[neo4j]` into the venv
5. Writes the Python shim to `~/.local/share/ekko/shim/`
6. Generates `~/.config/ekko/config.toml` (if not present)
7. Pulls Ollama models (`qwen3:8b`, `qwen3-embedding:8b`)

```
Neo4j available at bolt://localhost:7687 (web UI at http://localhost:7474)
```

---

## Technology Stack

| Component | Technology | Why |
|---|---|---|
| **ekko binary** | Rust (edition 2024) | Single static binary, fast startup, cross-platform |
| **MCP server** | `rmcp` crate | Official Rust MCP SDK, `#[tool]` proc macros |
| **CLI** | `clap` 4.x (derive) | Subcommands, derive macros |
| **Async runtime** | `tokio` (full) | Standard async runtime for Rust |
| **HTTP client** | `reqwest` | Only used for Ollama summarization |
| **Session index DB** | `rusqlite` (bundled) | Track which sessions have been indexed |
| **Self-update** | `self_update` | GitHub Releases integration |
| **Schema generation** | `schemars` | MCP tool parameter schemas |
| **Serialization** | `serde` + `serde_json` + `toml` | Config + API types |
| **Python shim** | graphiti-core[neo4j] | Direct library import, JSONL bridge |
| **Graph database** | Neo4j 5 Community | Graphiti's storage backend |
| **LLM + Embeddings** | Ollama (`qwen3:8b`, `qwen3-embedding:8b`) | Entity extraction, embeddings, summarization |
| **CI/CD** | GitHub Actions + release-please | Cross-platform binary builds |

### Why Rust + Python Shim

- **Rust** — Single static binary, <100ms MCP startup, cross-platform.
- **Python shim** — graphiti-core is ~15,000 lines of sophisticated Python. Porting it would take months. The JSONL boundary is clean and the daemon amortizes the Python startup cost.
- **No HTTP server** — The shim imports graphiti-core directly. No Graphiti MCP server to deploy or manage.

---

## Data Flow

### Storing a Memory

```
Agent calls remember("Joel prefers bun over npm")
    │
    ▼
ekko MCP server receives tool call
    │
    ▼
Detect project from cwd → group_id = "ekko"
    │
    ▼
Connection broker → daemon socket (or direct fallback)
    │
    ▼
JSONL request to Python shim:
  {"method": "add_memory", "params": {
    "name": "agent-memory-20260323-141500",
    "episode_body": "Joel prefers bun over npm",
    "group_id": "ekko",
    "source": "text",
    "source_description": "claude-code session"
  }}
    │
    ▼
Shim calls graphiti-core (async, 5-15s):
  1. LLM extracts entities: Joel, bun, npm
  2. LLM extracts facts: Joel→prefers→bun, Joel→avoids→npm
  3. Dedup against existing graph
  4. Check for contradictions (invalidate old facts)
  5. Store in Neo4j
    │
    ▼
ekko returns success to agent immediately
(graphiti-core processes via internal queue)
```

### Recalling Memories

```
Agent calls recall("what package manager should I use")
    │
    ▼
ekko MCP server receives tool call
    │
    ▼
Detect project from cwd → group_id = "ekko"
    │
    ▼
Sequential requests to shim (via daemon):
  1. search_facts(query, group_ids=["ekko"])
  2. search_nodes(query, group_ids=["ekko"])
    │
    ▼
Format results for agent:
  Facts:
    - "Joel prefers bun over npm" (valid, created 2026-03-18)
    - "Joel prefers pnpm for monorepos" (valid, created 2026-03-15)
    - "Joel used yarn" (invalid since 2026-01-01)
  Entities:
    - bun (technology): "JavaScript runtime and package manager"
    - npm (technology): "Node.js default package manager"
    │
    ▼
Return to agent
```

### Session Indexing

```
ekko sync
    │
    ▼
Scan known agent session directories:
  ~/.claude/projects/*/sessions/*.jsonl
  ~/.local/share/opencode/sessions/*.db
    │
    ▼
For each new/modified session:
  1. Parse agent-specific format → normalized turns
  2. Detect project from session metadata or file path
  3. Summarize via Ollama (or extractive fallback):
     "Designed ekko's architecture. Chose Rust + Graphiti.
      Decided against reimplementing graph logic."
  4. Feed to Graphiti as episode via shim:
     add_memory(
       name: "claude-code-session-{id}",
       episode_body: summary,
       group_id: project,
       source: "text",
       source_description: "claude-code session {id}, 2026-03-18"
     )
  5. Mark session as indexed in local SQLite DB
```

---

## What We Learned From Prior Experiments

| System | What Worked | What Didn't | What ekko Steals |
|---|---|---|---|
| **hivemind** | Schema had the right shape (decay, access tracking, temporal fields) | Nothing exercised the tracking. Memories accumulated but never ranked. | Lesson: don't build features you won't exercise. Start with Graphiti's proven capabilities. |
| **CASS** | Multi-agent session discovery. Knew where every agent stores history. | No semantic search. Raw chunks, no summarization. Rust binary hard to extend. | Session discovery patterns. Agent location map. |
| **swarm-mail Mem0** | LLM-driven ADD/UPDATE/DELETE is elegant dedup. | Required cloud LLM. | Graphiti does this natively with local Ollama. |
| **pdf-brain** | Ollama embeddings work at scale (484K embeddings). | PGlite + HNSW = 52GB for 907 docs. | Lesson: let the graph DB handle indexing, don't roll your own. |
| **Graphiti research** | Temporal fact lifecycle is the right model. Hybrid search with reranking. | Every write costs 3-8 LLM calls. Python-only. | Use it as-is. Don't reimplement. Accept the write cost — it's the price of intelligence. |

---

## Design Principles

1. **Don't rebuild what Graphiti does** — Entity extraction, fact dedup, temporal invalidation, hybrid search. These are solved. Use them.

2. **ekko is the integration layer** — It connects Graphiti to the agent ecosystem. MCP tools, session indexing, CLI, lifecycle management.

3. **Project-aware by default** — Detect the current project, scope queries automatically. The agent shouldn't have to think about `group_id`.

4. **Fail clearly** — If Graphiti is down, say so. If Ollama is missing, say so. Never silently degrade in a way that confuses the agent.

5. **Fast where it matters** — ekko's MCP server starts in <100ms. Graphiti's write latency (5-15s) is acceptable because it processes async. Search is <1s.

6. **Observe everything** — `ekko status`, `ekko doctor`, `ekko queue`, `ekko episodes`. You can always see what's in the graph and whether the system is healthy.

---

## Resolved Questions

These were open questions from the initial design. All have been answered through implementation.

1. **FalkorDB vs Neo4j** — Neo4j. Migrated at v0.10.0. More mature, better tooling, Graphiti's primary target. FalkorDB support was dropped.

2. **Ollama model selection** — `qwen3:8b` for LLM, `qwen3-embedding:8b` (4096 dimensions) for embeddings. Both work reliably with Graphiti.

3. **Session summarization** — Summarize first, then feed to Graphiti. Reduces noise and token cost. Extractive fallback available via `--no-llm`.

4. **Write latency** — Fire-and-forget. The `remember` tool returns immediately. The `queue` tool lets agents (and users) monitor processing status.

5. **Kuzu as embedded alternative** — Not pursued. Neo4j via container is simple enough and well-supported by Graphiti.

6. **HTTP vs embedded shim** — Embedded Python shim over JSONL. No HTTP server to manage. Daemon multiplexes the single shim process.

---

## Milestones

### v0.1 — Graphiti Integration ✅
- [x] Rust project scaffold (cargo, clap, tokio)
- [x] Graphiti client (typed JSONL wrapper via Python shim)
- [x] `ekko init` (Neo4j via container, Python venv, shim, Ollama models)
- [x] `ekko doctor` (health checks for all services)
- [x] `ekko status` (Graphiti connection status)
- [x] Config file management (`~/.config/ekko/config.toml`)
- [x] `ekko update` (self-update from GitHub Releases)
- [x] CI/CD (release-please + cross-platform binary builds)

### v0.2 — MCP Server ✅
- [x] MCP server via STDIO (`ekko serve`)
- [x] `remember` tool (store memory)
- [x] `recall` tool (search facts + nodes)
- [x] `forget` tool (delete edge)
- [x] `entities` tool (search nodes)
- [x] `episodes` tool (list episodes)
- [x] `status` tool (health check)
- [x] `queue` tool (processing queue visibility)
- [x] Project detection from cwd → automatic group_id

### v0.3 — CLI ✅
- [x] `ekko add` (store memory)
- [x] `ekko ask` (search)
- [x] `ekko show` (inspect fact)
- [x] `ekko rm fact|episode` (delete)
- [x] `ekko nodes` (list/search entities)
- [x] `ekko episodes` (list episodes)
- [x] `ekko clear` (wipe project memory)
- [x] `ekko queue` (processing queue)

### v0.4 — Session Indexing (partial)
- [x] Claude Code parser (JSONL)
- [x] OpenCode parser (SQLite)
- [ ] Cursor parser
- [ ] Aider parser
- [ ] Gemini CLI parser
- [x] Session summarization via Ollama (with extractive fallback)
- [x] `ekko sync` (incremental indexing, project-scoped by default)
- [x] `ekko sync --all` / `--agent` / `--since` / `--dry-run` / `--no-llm` / `--limit`
- [x] Index tracking database (rusqlite)

### v0.5 — Daemon ✅
- [x] Daemon server with Unix socket multiplexer
- [x] Auto-start daemon on first use
- [x] flock-based race prevention for concurrent starts
- [x] Auto-respawn crashed shim processes
- [x] Periodic background sync (30min interval)
- [x] Non-blocking lightweight ops (queue/health/status via `try_lock`)
- [x] `ekko daemon start|stop|status`

### v0.6 — Distribution ✅
- [x] Cross-platform release binaries (GitHub Actions: linux/mac/windows, amd64/arm64)
- [x] Installation script (`install.sh`)
- [x] Documentation (README.md)

### Next — Polish
- [ ] `ekko export` / `ekko import` (JSON backup/restore)
- [ ] Shell completions (bash, zsh, fish — clap supports natively)
- [ ] Remaining session parsers (Cursor, Aider, Gemini CLI)
- [ ] Test suite (unit, integration, e2e — currently zero tests)
- [ ] `_global` group_id support in CLI commands
- [ ] Windows daemon support (named pipes instead of Unix sockets)

### Future — Discover & Fill Gaps
- [ ] Relevance decay (time-weighted fact scoring)
- [ ] Working memory blocks (structured context for agents)
- [ ] Context assembly (token-budgeted prompt blocks)
- [ ] Cross-machine sync (distributed knowledge graphs)

---

## Name

**ekko** — from the Greek *echo* (reflection, reverberation). Memories that echo back when you need them.
