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

## Setup

```bash
ekko init       # set up Graphiti, graph DB, pull Ollama models
ekko doctor     # verify everything is working
```

## Update

```bash
ekko update
```

## License

MIT
