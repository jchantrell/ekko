# ekko

Persistent memory for AI agents, powered by [Graphiti](https://github.com/getzep/graphiti)'s temporal knowledge graph.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/jchantrell/ekko/main/install.sh | sh
```

Or build from source:

```bash
cargo install --git https://github.com/jchantrell/ekko
```

## Requirements

- Python 3.10+ (for the graphiti-core shim)
- Docker or Podman (for Neo4j)
- [Ollama](https://ollama.com) (LLM and embedding inference)

## Getting Started

```bash
ekko init       # set up Neo4j, Python venv, pull Ollama models
ekko doctor     # verify everything is working
ekko update     # check for updates
```

`ekko init` will:
1. Start a Neo4j container via Docker/Podman
2. Create a Python venv with `graphiti-core` installed
3. Pull the default Ollama models (`qwen3:8b`, `qwen3-embedding:8b`)

## CLI

```bash
ekko add "prefers pnpm over npm"           # store a memory
ekko ask "what package manager"            # search facts
ekko nodes "search query"                  # search entities
ekko episodes                              # list recent episodes
ekko show <uuid>                           # inspect a fact
ekko rm fact <uuid>                        # delete a fact
ekko clear                                 # wipe current project's memory
ekko clear my-project                      # wipe a specific project's memory
ekko sync                                  # index agent sessions (last 30 days)
ekko sync --full                           # index all agent sessions
```

## MCP Server

ekko exposes 7 tools over STDIO: `remember`, `recall`, `forget`, `entities`, `episodes`, `queue`, `status`.

### Claude Code

Global (all projects):

```bash
claude mcp add --scope user ekko -- ekko serve
```

Project-level:

```bash
claude mcp add ekko -- ekko serve
```

### OpenCode

Add to your `opencode.json` (global: `~/.config/opencode/opencode.jsonc`, or project-level):

```jsonc
{
  "mcp": {
    "ekko": {
      "type": "local",
      "command": ["ekko", "serve"]
    }
  }
}
```

### Cursor / Windsurf / Other MCP Hosts

Add to your MCP config (e.g. `~/.cursor/mcp.json`):

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

## AGENTS.md

Add this to your `AGENTS.md` (global or project-level) so the agent knows how to use memory:

```markdown
## Memory (ekko)

Use ekko for all persistent memory. It's an MCP server backed by a temporal knowledge graph.

- `remember` — store preferences, decisions, patterns, learnings
- `recall` — search for relevant memories before solving problems from scratch
- `forget` — remove incorrect facts by UUID
- `entities` — explore the knowledge graph (people, technologies, concepts)
- `episodes` — list memory ingestion history
- `queue` — check memory processing queue status
- `status` — check if the memory system is healthy

### When to use memory

**Always `recall` at the start of a task** — check if relevant context already exists before solving from scratch.

**Always `remember` things worth preserving** — use your judgement. If future sessions would benefit from knowing it, store it. Examples:

- A tricky bug and its root cause
- An architecture decision and why it was made
- A user preference or workflow pattern you observed
- A dependency gotcha or environment quirk
- A pattern that worked well (or failed)

### Scoping

Memory is **project-scoped** by default. The `group_id` is auto-detected from the working directory name (e.g. `/home/user/projects/my-app` → `my-app`). You can override with an explicit `group_id` param.

### Global memory

For knowledge that transcends any single project — user preferences, tooling choices, workflow patterns, environment details — pass `group_id: "_global"` explicitly. Use your judgement on what qualifies:

- **Global**: "prefers Rust or Golang", "uses CachyOS", "always use conventional commits"
- **Project**: bug patterns, architecture decisions, dependency notes, codebase-specific learnings
```
