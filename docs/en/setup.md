# The Crab Engram — Setup Guide

## Prerequisites

- **Rust toolchain** (1.85+ for edition 2024 support)
  ```bash
  rustup update stable
  ```
- **Git** (for clone + git hook integration)
- No external database needed — SQLite is bundled

## Build

```bash
# Clone
git clone https://github.com/maisonnat/the-crab-engram.git
cd the-crab-engram

# Debug build
cargo build

# Release build (optimized, LTO, stripped)
cargo build --release
```

The release profile uses:
- LTO (Link-Time Optimization)
- Single codegen unit
- Symbol stripping

## Run

### MCP Server (for AI agents)

```bash
# Default project, agent profile
cargo run -- mcp

# Specific project + admin profile
cargo run -- mcp --project my-app --profile admin

# All tools (agent + admin)
cargo run -- mcp --profile all
```

The MCP server communicates over **stdio** (JSON-RPC). Configure your AI agent to launch `the-crab-engram mcp` as its MCP server.

### HTTP REST API

```bash
# Default port 7437
cargo run -- serve

# Custom port
cargo run -- serve --port 8080
```

The API listens on `0.0.0.0:7437` with permissive CORS.

### TUI (Terminal UI)

```bash
cargo run -- tui
```

**Keybindings:** `1` Dashboard, `2` Search, `3` Capsules, `4` Boundaries, `j/k` navigate, `Enter` select, `Esc` back, `q` quit.

### CLI

```bash
# Search
cargo run -- search "auth JWT"

# Save
cargo run -- save --title "Fix N+1" --content "Used eager loading" --type bugfix --session-id $(uuidgen)

# Stats
cargo run -- stats

# Export
cargo run -- export --output backup.json

# Export context as system prompt
cargo run -- export-context --max-tokens 2000

# Encrypt database
cargo run -- encrypt --passphrase "my-secret"
```

## Tests

```bash
# All tests across workspace
cargo test --workspace

# Tests for a specific crate
cargo test -p engram-core
cargo test -p engram-store
cargo test -p engram-learn

# Integration tests only
cargo test --test integration_store

# With output
cargo test --workspace -- --nocapture
```

### Test Coverage

| Crate | Tests | Focus |
|---|---|---|
| engram-core | ~50 | Observation types, crypto roundtrip, belief state machine, permissions |
| engram-store | ~30 | CRUD, search, FTS5, dedup, export/import, graph traversal |
| engram-learn | ~25 | Consolidation, capsules, anti-patterns, smart injection |
| engram-mcp | ~15 | Tool dispatch, profile filtering |
| engram-api | ~5 | Serialization, error types |
| engram-sync | ~10 | CRDT state, chunk export/import |
| integration | ~10 | Full session workflow, graph operations, export/import roundtrip |
| **Total** | **~199** | |

## Lint

```bash
cargo clippy --workspace -- -D warnings
```

## Configuration

### Database Path

Default: `~/.engram/engram.db`

Override with `--db` flag:
```bash
cargo run -- --db /path/to/custom.db mcp
```

### Project Name

Default: `"default"`

Override with `--project`:
```bash
cargo run -- --project my-app mcp
```

### Logging

Uses `tracing-subscriber` with env filter:
```bash
RUST_LOG=debug cargo run -- mcp
RUST_LOG=engram_store=trace cargo run -- serve
```

Default level: `warn`.

## Agent Setup

### Automatic (Claude Code, Cursor, Gemini CLI)

Engram can auto-configure itself for popular AI agents:

```bash
the-crab-engram setup claude-code   # → ~/.claude/skills/engram-memory.md
the-crab-engram setup cursor         # → ~/.cursor/rules/engram-memory.md
the-crab-engram setup gemini-cli     # → ~/.gemini/extensions/engram-memory.md
```

Each writes a SKILL.md with instructions for the agent on how to use The Crab Engram's MCP tools.

### Manual (OpenCode)

For OpenCode, configure in `~/.config/opencode/opencode.json`:

```json
"mcp": {
  "engram": {
    "command": ["the-crab-engram", "mcp"],
    "type": "local"
  }
}
```

The AGENTS.md protocol is included in the project. Copy the Engram Protocol section
into your `~/.config/opencode/AGENTS.md` to enable proactive memory capture.

## Project Structure (Runtime)

```
~/.engram/
└── engram.db          # SQLite database (WAL mode)
    ├── sessions
    ├── observations
    ├── observations_fts    # FTS5 index
    ├── edges
    ├── knowledge_capsules
    ├── observation_attachments
    ├── beliefs
    ├── entities
    └── ... (13 migrated tables)
```
