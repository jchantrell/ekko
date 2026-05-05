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
- An LLM provider: [Ollama](https://ollama.com) (default), Anthropic, or any OpenAI-compatible API

## Getting Started

```bash
ekko init       # set up Neo4j, Python venv, install provider deps
ekko doctor     # verify everything is working
ekko update     # check for updates
```

`ekko init` will:
1. Start a Neo4j container via Docker/Podman
2. Create a Python venv with `graphiti-core` and provider-specific extras
3. Pull Ollama models if using the default local provider

## Configuration

Config lives at `~/.config/ekko/config.toml`. The default uses Ollama for local inference:

```toml
[graphiti.llm]
provider = "openai"           # "openai" (OpenAI-compatible) or "anthropic"
model = "qwen3:8b"
api_url = "http://localhost:11434/v1"
api_key = "ollama"

[graphiti.embedder]
provider = "openai"           # "openai" (OpenAI-compatible) or "voyageai"
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

### Using Anthropic

```toml
[graphiti.llm]
provider = "anthropic"
model = "claude-haiku-4-5-latest"
api_key = "sk-ant-..."

[graphiti.embedder]
provider = "voyageai"
model = "voyage-3"
dimensions = 1024
api_key = "pa-..."
```

After changing providers, re-run `ekko init` to install the required dependencies.

## CLI

```bash
ekko add "prefers pnpm over npm"           # store a memory
ekko ask "what package manager"            # search facts (searches all memories)
ekko nodes "search query"                  # search entities (searches all memories)
ekko episodes                              # list recent episodes (all origins)
ekko show <uuid>                           # inspect a fact
ekko rm fact <uuid>                        # delete a fact
ekko origins                               # list all known origins
ekko origins --stats                       # include entity/episode counts
ekko origins set myproj --name "My Project" --description "Web app backend"
ekko clear                                 # wipe current project's memories
ekko clear my-project                      # wipe a specific origin's memories
ekko sync                                  # index agent sessions (last 30 days)
ekko sync --full                           # index all agent sessions
```

## MCP Server

ekko exposes 9 tools over STDIO: `remember`, `recall`, `forget`, `entities`, `episodes`, `origins`, `set_origin`, `queue`, `status`.

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
- `origins` — list all known project sources (discover what's in memory)
- `set_origin` — set a display name and description for a project origin
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

### How memory works

All memories live in a single unified graph — `recall` searches across everything, regardless of which project stored it. Each memory is tagged with an **origin** (auto-detected from the working directory) so you can see where it came from, but origins never filter search results.
```
