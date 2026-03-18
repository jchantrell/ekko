# ekko

Persistent memory for AI agents, powered by Graphiti's temporal knowledge graph.

Rust binary. MCP server. CLI. Session indexer. One install.

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
4. **Lifecycle manager** — Handles Graphiti setup, health checks, and graceful degradation

ekko does NOT duplicate what Graphiti already does. No custom entity extraction. No custom dedup logic. No custom graph storage. Graphiti is the brain. ekko is the nervous system that connects it to the world.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      ekko (Rust binary)                      │
│                                                              │
│  ┌────────────┐  ┌────────────┐  ┌─────────────────────────┐│
│  │ MCP Server │  │    CLI     │  │    Session Indexer      ││
│  │  (STDIO)   │  │   (clap)  │  │                         ││
│  │            │  │            │  │  Claude Code sessions   ││
│  │  Tools:    │  │  ekko add  │  │  OpenCode sessions     ││
│  │  remember  │  │  ekko ask  │  │  Cursor sessions       ││
│  │  recall    │  │  ekko show │  │  Aider history         ││
│  │  forget    │  │  ekko init │  │                         ││
│  │  status    │  │  ekko sync │  │  Parse → Summarize →   ││
│  │            │  │  ekko ...  │  │  Feed to Graphiti      ││
│  └─────┬──────┘  └─────┬──────┘  └────────────┬────────────┘│
│        └───────────────┼──────────────────────┘             │
│                        │                                     │
│  ┌─────────────────────┴───────────────────────────────────┐│
│  │                  Graphiti Client                         ││
│  │                                                          ││
│  │  add_memory()       search_nodes()                      ││
│  │  search_facts()     get_episodes()                      ││
│  │  delete_edge()      clear_graph()                       ││
│  │  get_status()                                           ││
│  │                                                          ││
│  │  HTTP client (reqwest) → localhost:8000                  ││
│  └─────────────────────┬───────────────────────────────────┘│
└────────────────────────┼────────────────────────────────────┘
                         │ HTTP (JSON-RPC / MCP over HTTP)
┌────────────────────────┴────────────────────────────────────┐
│                Graphiti MCP Server (Python)                   │
│                                                              │
│  Temporal knowledge graph engine:                            │
│  • Entity extraction (LLM-driven)                            │
│  • Fact deduplication & contradiction detection              │
│  • Temporal validity (valid_at / invalid_at / expired_at)    │
│  • Hybrid search (vector + BM25 + graph traversal)          │
│  • Reranking (RRF, MMR, cross-encoder, node distance)       │
│  • Community detection (label propagation)                   │
│  • Episode provenance                                        │
│  • Group isolation (project scoping via group_id)            │
│                                                              │
│  Graph DB: Neo4j or FalkorDB                                │
│  LLM/Embeddings: Ollama (local) or OpenAI                   │
└──────────────────────────────────────────────────────────────┘
```

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
| **Session indexing** | Graphiti doesn't know where Claude Code or Cursor store their sessions. ekko discovers, parses, summarizes, and feeds them as episodes. |
| **CLI** | Graphiti has no CLI. ekko provides terminal-based inspection, search, and maintenance. |
| **Lifecycle management** | `ekko init` sets up Graphiti (container or uv), configures Ollama, runs health checks. `ekko doctor` diagnoses issues. |
| **Agent-specific context** | Detects current project from cwd, scopes queries to relevant `group_id`, formats results for agent consumption. |
| **Graceful degradation** | If Graphiti is down, ekko reports status clearly rather than crashing. If Ollama is down, ekko tells the agent. |

---

## Graphiti Integration

### API Surface

ekko talks to Graphiti's MCP server over HTTP. The full API surface is 8 tools:

| Tool | Parameters | Returns | ekko Uses For |
|---|---|---|---|
| `add_memory` | `name`, `episode_body`, `group_id`, `source`, `source_description` | Success/error | Storing memories, ingesting sessions |
| `search_nodes` | `query`, `group_ids`, `max_nodes`, `entity_types` | List of entities with summaries | Entity lookup, graph exploration |
| `search_memory_facts` | `query`, `group_ids`, `max_facts`, `center_node_uuid` | List of facts with temporal metadata | Memory recall, context assembly |
| `get_episodes` | `group_ids`, `max_episodes` | List of episodes | Session history |
| `get_entity_edge` | `uuid` | Single fact with full metadata | Detailed inspection |
| `delete_entity_edge` | `uuid` | Success/error | Forgetting specific facts |
| `delete_episode` | `uuid` | Success/error | Removing ingested sessions |
| `clear_graph` | `group_ids` | Success/error | Resetting project memory |
| `get_status` | — | Health status | Health checks |

### Graphiti Client (Rust)

A typed HTTP client wrapping these 8 tools:

```rust
pub struct GraphitiClient {
    http: reqwest::Client,
    base_url: String,
}

impl GraphitiClient {
    pub async fn add_memory(&self, req: AddMemoryRequest) -> Result<()>;
    pub async fn search_nodes(&self, req: SearchNodesRequest) -> Result<Vec<Node>>;
    pub async fn search_facts(&self, req: SearchFactsRequest) -> Result<Vec<Fact>>;
    pub async fn get_episodes(&self, req: GetEpisodesRequest) -> Result<Vec<Episode>>;
    pub async fn get_edge(&self, uuid: &str) -> Result<Fact>;
    pub async fn delete_edge(&self, uuid: &str) -> Result<()>;
    pub async fn delete_episode(&self, uuid: &str) -> Result<()>;
    pub async fn clear_graph(&self, group_ids: &[String]) -> Result<()>;
    pub async fn status(&self) -> Result<Status>;
}
```

### Configuration

Graphiti needs to know where to find its LLM and graph DB. ekko manages this via `~/.config/ekko/config.toml`:

```toml
[graphiti]
url = "http://localhost:8000"

[graphiti.llm]
provider = "openai"          # OpenAI-compatible (works with Ollama)
model = "llama3.2:3b"
api_url = "http://localhost:11434/v1"
api_key = "ollama"

[graphiti.embedder]
provider = "openai"
model = "nomic-embed-text"
dimensions = 768
api_url = "http://localhost:11434/v1"
api_key = "ollama"

[graphiti.database]
provider = "falkordb"        # or "neo4j"
uri = "redis://localhost:6379"
```

`ekko init` generates this config and starts the required services.

---

## Session Indexing

The highest-value feature ekko adds. Agents produce rich conversation history that currently evaporates. ekko captures it.

### Agent Session Discovery

| Agent | Session Location | Format |
|---|---|---|
| Claude Code | `~/.claude/projects/*/sessions/` | JSONL (messages with tool calls) |
| OpenCode | `~/.config/opencode/sessions/` | JSONL (messages with parts) |
| Cursor | `~/.cursor-tutor/` | JSON |
| Aider | `~/.aider.chat.history.md` | Markdown |
| Gemini CLI | `~/.gemini/` | JSON |

### Indexing Pipeline

```
Session file discovered
        │
        ▼
Parse agent-specific format → normalized messages
        │
        ▼
Summarize via Ollama (or skip if no LLM available)
  - 2-3 sentence summary
  - Key decisions made
  - Files changed
  - Outcome (success/partial/failed)
        │
        ▼
Feed to Graphiti as episode
  - group_id = project name (from cwd or session metadata)
  - source = agent name
  - source_description = session ID + timestamp
        │
        ▼
Graphiti extracts entities + facts automatically
  - Technologies mentioned
  - Patterns applied
  - Decisions and their rationale
  - Bugs encountered and fixes
```

### Incremental Indexing

ekko tracks which sessions have been indexed in a local SQLite database:

```sql
CREATE TABLE indexed_sessions (
    session_id TEXT PRIMARY KEY,
    agent TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,       -- detect modifications
    indexed_at TEXT NOT NULL,
    group_id TEXT,
    episode_count INTEGER DEFAULT 0
);
```

`ekko sync` checks for new/modified sessions and indexes only what's changed. Full re-index via `ekko sync --full`.

---

## MCP Server

ekko's MCP server is the primary integration point for AI agents. It wraps Graphiti's capabilities with agent-friendly semantics.

### Tools

| Tool | Description | Maps To |
|---|---|---|
| `ekko_remember` | Store a memory. Accepts freeform text + optional group_id. | `add_memory` |
| `ekko_recall` | Search for relevant memories. Returns facts + entities. | `search_memory_facts` + `search_nodes` |
| `ekko_forget` | Invalidate a specific fact by UUID. | `delete_entity_edge` |
| `ekko_entities` | Search for entities in the knowledge graph. | `search_nodes` |
| `ekko_episodes` | List recent episodes/sessions. | `get_episodes` |
| `ekko_status` | Health check — is Graphiti up? Is Ollama running? | `get_status` + local checks |
| `ekko_sync` | Trigger session indexing. | Session indexer |

### Tool Design Principles

- **Project-aware by default**: If the agent is working in `~/Workspace/ekko`, queries are automatically scoped to `group_id = "ekko"`. No need to specify project every time.
- **Thin wrappers**: Tools add minimal logic on top of Graphiti. No re-ranking, no custom scoring. Let Graphiti do its job.
- **Structured results**: Return facts with temporal metadata (`valid_at`, `invalid_at`) so the agent can reason about currency.
- **Fail clearly**: If Graphiti is down, return a clear error message, not a crash.

### STDIO Transport

The MCP server uses STDIO transport (stdin/stdout JSON-RPC). The host (Claude Code, OpenCode, etc.) spawns `ekko serve` as a child process.

```json
// claude_desktop_config.json or equivalent
{
  "mcpServers": {
    "ekko": {
      "command": "ekko",
      "args": ["serve"]
    }
  }
}
```

Startup target: <100ms (Rust binary, no Python in the critical path).

---

## CLI

```bash
# Setup
ekko init                        # Set up Graphiti (auto-detects docker/podman or uv)
ekko init --container            # Force container runtime (docker/podman)
ekko init --uv                   # Force uv/Python setup
ekko doctor                      # Health check (Graphiti, Ollama, graph DB)

# Memory
ekko add "Joel prefers bun"                    # Store a memory
ekko add "Switched to pnpm" --group ekko       # Store with project scope
ekko ask "what package manager"                # Search facts
ekko ask "what do we know about libSQL" --nodes # Include entity results
ekko show <uuid>                               # Inspect a specific fact
ekko rm <uuid>                                 # Delete a fact

# Graph exploration
ekko nodes                       # List entities
ekko nodes --type technology     # Filter by type
ekko nodes "libSQL"              # Search entities by name

# Sessions
ekko sync                        # Index new agent sessions
ekko sync --full                  # Re-index everything
ekko sync --agent claude-code     # Index only Claude Code sessions
ekko episodes                     # List recent episodes
ekko episodes --group ekko        # Episodes for a project

# Maintenance
ekko status                       # Graphiti + Ollama health
ekko clear --group ekko           # Wipe a project's memory
ekko export --group ekko          # Export to JSON
ekko import <file>                # Import from JSON
```

---

## Setup & Installation

### Prerequisites

- **Ollama** — For local LLM and embeddings. `curl -fsSL https://ollama.com/install.sh | sh`
- **Docker or Podman** (recommended) or **Python 3.10+ with uv** — For running Graphiti

### Install ekko

```bash
# Download the binary
curl -fsSL https://ekko.dev/install.sh | sh

# Or build from source
cargo install ekko
```

### Initialize

```bash
ekko init
```

This:
1. Detects whether a container runtime (docker/podman) or Python/uv is available
2. Pulls the Graphiti MCP server image (or creates a uv virtualenv)
3. Starts FalkorDB (via container) or configures Neo4j connection
4. Pulls required Ollama models (`nomic-embed-text`, `llama3.2:3b`)
5. Generates `~/.config/ekko/config.toml`
6. Creates the session index database at `~/.local/share/ekko/index.db`
7. Runs `ekko doctor` to verify everything works

### Container Setup (Recommended)

```bash
ekko init --container
# Auto-detects docker or podman
# Pulls: zepai/knowledge-graph-mcp:standalone + falkordb/falkordb:latest
# Starts: {docker|podman} compose up -d
# Graphiti available at localhost:8000
```

### Python/uv Setup (No Container Runtime)

```bash
ekko init --uv
# Creates: ~/.local/share/ekko/venv/
# Installs: graphiti-core[falkordb] (or [kuzu] for embedded)
# Starts: uv run graphiti-mcp-server --transport http
```

---

## Technology Stack

| Component | Technology | Why |
|---|---|---|
| **ekko binary** | Rust | Single static binary, fast startup, cross-platform |
| **MCP server** | `rmcp` crate | Official Rust MCP SDK, `#[tool]` proc macros |
| **CLI** | `clap` | Derive macros, subcommands, shell completions |
| **HTTP client** | `reqwest` | Async, connection pooling, standard Rust HTTP |
| **Session index DB** | `rusqlite` | Track which sessions have been indexed |
| **Async runtime** | `tokio` | Standard async runtime for Rust |
| **Serialization** | `serde` + `serde_json` | Type-safe JSON for Graphiti API |
| **Knowledge graph** | Graphiti (Python, sidecar) | Temporal knowledge graph engine |
| **Graph database** | FalkorDB (container) or Neo4j | Graphiti's storage backend |
| **LLM + Embeddings** | Ollama (local) | Entity extraction, embeddings, summarization |

### Why Rust

- **Single static binary** — `cargo build --release` produces one file. No runtime dependencies.
- **Fast startup** — <100ms for MCP STDIO server. Critical for agent integration.
- **Cross-platform** — linux/mac/windows from one codebase.
- **No Python in the critical path** — Graphiti runs as a sidecar. ekko itself has zero Python dependency.

### Why Not Rewrite Graphiti in Rust

Graphiti is ~15,000 lines of sophisticated Python: LLM-driven entity extraction, temporal edge resolution, community detection, multiple graph DB drivers. Porting it would take months and produce something worse. The HTTP boundary is clean and fast enough (localhost, <2ms overhead).

---

## Data Flow

### Storing a Memory

```
Agent calls ekko_remember("Joel prefers bun over npm")
    │
    ▼
ekko MCP server receives tool call
    │
    ▼
Detect project from cwd → group_id = "ekko"
    │
    ▼
POST to Graphiti: add_memory(
    name: "agent-memory-{timestamp}",
    episode_body: "Joel prefers bun over npm",
    group_id: "ekko",
    source: "text",
    source_description: "claude-code session"
)
    │
    ▼
Graphiti (async, 5-15s):
  1. LLM extracts entities: Joel, bun, npm
  2. LLM extracts facts: Joel→prefers→bun, Joel→avoids→npm
  3. Dedup against existing graph
  4. Check for contradictions (invalidate old facts)
  5. Store in graph DB
    │
    ▼
ekko returns success to agent immediately
(Graphiti processes async via internal queue)
```

### Recalling Memories

```
Agent calls ekko_recall("what package manager should I use")
    │
    ▼
ekko MCP server receives tool call
    │
    ▼
Detect project from cwd → group_id = "ekko"
    │
    ▼
Parallel requests to Graphiti:
  1. search_memory_facts(query, group_ids=["ekko"])
  2. search_nodes(query, group_ids=["ekko"])
    │
    ▼
Merge results, format for agent:
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
  ~/.config/opencode/sessions/*.jsonl
  ~/.cursor-tutor/*.json
    │
    ▼
For each new/modified session:
  1. Parse agent-specific format → normalized messages
  2. Detect project from session metadata or file path
  3. Summarize via Ollama:
     - "Designed ekko's architecture. Chose Rust + Graphiti.
        Decided against reimplementing graph logic."
  4. Feed to Graphiti as episode:
     add_memory(
       name: "claude-code-session-{id}",
       episode_body: summary,
       group_id: project,
       source: "text",
       source_description: "claude-code session {id}, 2026-03-18"
     )
  5. Mark session as indexed in local DB
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

6. **Observe everything** — `ekko status`, `ekko doctor`, `ekko episodes`. You can always see what's in the graph and whether the system is healthy.

---

## Open Questions

1. **FalkorDB vs Neo4j**: FalkorDB is lighter (Redis-based, container-friendly). Neo4j is more mature and has better tooling. Which should be the default?

2. **Ollama model selection**: What's the smallest model that Graphiti works reliably with? The README warns that small models may produce incorrect output schemas. Need to test `llama3.2:3b`, `qwen2.5:7b`, `deepseek-r1:7b`.

3. **Session summarization**: Should ekko summarize sessions before feeding to Graphiti, or feed raw transcripts and let Graphiti extract what it wants? Summarization reduces noise but loses detail.

4. **Write latency**: Graphiti's `add_memory` returns immediately (queued), but the actual processing takes 5-15s. Should ekko expose this latency to the agent, or always fire-and-forget?

5. **Kuzu as embedded alternative**: Graphiti supports Kuzu (embedded, no server). But Kuzu is archived and the MCP server doesn't support it. Worth patching, or stick with FalkorDB/Neo4j?

6. **What gaps will we discover?** Once Graphiti is running as the memory engine, what will be missing? Relevance decay? Working memory blocks? Context assembly? We'll find out through use.

---

## Milestones

### v0.1 — Graphiti Integration
- [x] Rust project scaffold (cargo, clap, reqwest, tokio)
- [x] Graphiti client (typed HTTP wrapper for 8 MCP tools)
- [x] `ekko init` (container/uv setup for Graphiti + FalkorDB + Ollama config)
- [x] `ekko doctor` (health checks for all services)
- [x] `ekko status` (Graphiti connection status)
- [x] Config file management (`~/.config/ekko/config.toml`)
- [x] `ekko update` (self-update from GitHub Releases)
- [x] CI/CD (release-please + cross-platform binary builds)

### v0.2 — MCP Server
- [ ] MCP server via STDIO (`ekko serve`)
- [ ] `ekko_remember` tool (store memory via Graphiti)
- [ ] `ekko_recall` tool (search facts + nodes)
- [ ] `ekko_forget` tool (delete edge)
- [ ] `ekko_entities` tool (search nodes)
- [ ] `ekko_episodes` tool (list episodes)
- [ ] `ekko_status` tool (health check)
- [ ] Project detection from cwd → automatic group_id

### v0.3 — CLI
- [ ] `ekko add` (store memory)
- [ ] `ekko ask` (search)
- [ ] `ekko show` (inspect fact/entity)
- [ ] `ekko rm` (delete)
- [ ] `ekko nodes` (list/search entities)
- [ ] `ekko episodes` (list episodes)
- [ ] `ekko clear` (wipe project memory)

### v0.4 — Session Indexing
- [ ] Session discovery (Claude Code, OpenCode, Cursor, Aider, Gemini)
- [ ] Agent-specific parsers (JSONL, JSON, Markdown)
- [ ] Session summarization via Ollama
- [ ] `ekko sync` (incremental indexing)
- [ ] `ekko sync --full` (re-index)
- [ ] Index tracking database (rusqlite)

### v0.5 — Polish
- [ ] `ekko export` / `ekko import` (JSON backup)
- [ ] Shell completions (bash, zsh, fish)
- [ ] Cross-platform release binaries (GitHub Actions)
- [ ] Installation script
- [ ] Documentation

### Future — Discover & Fill Gaps
- [ ] Evaluate whether Graphiti's search is sufficient or needs a relevance layer
- [ ] Evaluate whether working memory blocks are needed
- [ ] Evaluate whether context assembly (token-budgeted prompt blocks) adds value
- [ ] Evaluate decay/access tracking needs
- [ ] Consider Turso sync for cross-machine memory

---

## Name

**ekko** — from the Greek *echo* (reflection, reverberation). Memories that echo back when you need them.
