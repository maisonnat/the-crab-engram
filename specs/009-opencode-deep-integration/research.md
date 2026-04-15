# Research: OpenCode Deep Integration

**Date**: 2026-04-15
**Branch**: `009-opencode-deep-integration`

---

## R1: OpenCode Plugin API (@opencode-ai/plugin v1.4.6)

### Decision: Use server-side Plugin type with Hooks interface

**Rationale**: The plugin API is stable and well-typed. The `Plugin` function receives `PluginInput` (client, project, directory, worktree, serverUrl, `$` BunShell) and returns `Promise<Hooks>`. Hooks are an interface with optional methods for events, tools, chat interception, and experimental features.

**Alternatives considered**:
- TUI plugin (`TuiPlugin`) — rejected because it's for UI extensions, not agent behavior
- MCP-only approach (no plugin) — rejected because it requires the agent to call tools manually; no auto-capture

### Available Hooks (confirmed from types)

**Stable hooks we'll use:**
- `event` — receives ALL events, filter by `event.type` string
- `tool.execute.after` — fires after any tool execution (detect git commits, errors)
- `chat.message` — fires when a new message is sent (push injection trigger)

**Experimental hooks we'll use:**
- `experimental.chat.system.transform` — appends strings to system prompt via `output.system.push()`
- `experimental.session.compacting` — injects context before compaction via `output.context.push()`

### Events we need (confirmed from SDK v2 types)

| Event Type String | Properties | Use Case |
|---|---|---|
| `session.created` | `{ sessionID, info: Session }` | Start engram session |
| `session.idle` | `{ sessionID }` | Trigger consolidation |
| `session.deleted` | `{ sessionID, info: Session }` | End engram session |
| `session.compacted` | `{ sessionID }` | Post-compaction awareness |
| `file.edited` | `{ file }` | File change tracking |
| `message.updated` | `{ sessionID, info: Message }` | Push injection trigger |

---

## R2: Plugin Communication Architecture

### Decision: Plugin communicates with `the-crab-engram serve` via HTTP API

**Rationale**: The HTTP API (`the-crab-engram serve --port 7437`) already exists with 18 endpoints covering search, inject, stats, capsules, antipatterns, consolidate, sessions, and full CRUD. The plugin can call these via `fetch()` — no CLI subcommands needed.

**Alternatives considered**:
- Plugin calls CLI subcommands — rejected: spawning processes is slower and fragile
- Plugin uses MCP stdio — rejected: complex to set up stdio transport from a plugin
- Plugin embeds SQLite directly — rejected: violates Constitution III (Storage Trait)

### HTTP API gaps (endpoints needed but missing)

| Missing Endpoint | Purpose | Effort |
|---|---|---|
| `GET /health` | Plugin health check before operations | Trivial (5 lines) |
| `POST /sessions/:id/end` | End session with summary | ~20 lines |
| `POST /capture/passive` | Passive learning capture from output | ~30 lines |
| `POST /capture/git` | Git commit capture | ~30 lines |
| `POST /capture/error` | Error capture | ~30 lines |

**Decision**: Add `/health` and session end in this feature. Capture endpoints can be deferred — the plugin can use `POST /observations` for the same effect (just less structured).

---

## R3: OpenCode Config Format

### Decision: Merge MCP entry + plugin path into existing config, never overwrite

**Rationale**: OpenCode config (`opencode.json`) uses a flat JSON structure with `mcp` object and `plugin` array. The MCP registration format is:

```json
{
  "mcp": {
    "the-crab-engram": {
      "type": "local",
      "command": ["the-crab-engram", "mcp"],
      "enabled": true
    }
  },
  "plugin": ["./plugins/the-crab-engram.ts"]
}
```

The plugin string resolves relative to `.opencode/` directory. The plugin must export a `PluginModule`:

```typescript
export default {
  server: async (input, options) => { return { /* hooks */ }; }
};
```

**JSONC support**: OpenCode supports `.jsonc` files (JSON with `//` line comments). The merge engine must strip comments before parsing.

**Config path resolution order**:
1. `$OPENCODE_CONFIG_DIR` env var
2. Platform default: `~/.config/opencode/` (Linux/macOS), `%APPDATA%\opencode\` (Windows)
3. Project-level: `.opencode/` in current directory

---

## R4: Cross-Platform Path Handling

### Decision: Use `dirs` crate (already in workspace deps)

**Rationale**: The `dirs` crate is already used in `main.rs:717` for `dirs::home_dir()`. It provides `dirs::config_dir()` which resolves correctly on all 3 platforms.

**Path mapping**:

| Platform | Config Dir | Source |
|---|---|---|
| Linux | `~/.config/opencode/` | `dirs::config_dir()` + `/opencode/` |
| macOS | `~/Library/Application Support/opencode/` | `dirs::config_dir()` + `/opencode/` |
| Windows | `%APPDATA%\opencode\` | `dirs::config_dir()` + `\opencode\` |

---

## R5: Auto-Start Server Strategy

### Decision: Health check + spawn via BunShell

**Rationale**: The plugin receives `$: BunShell` which can spawn processes. The plugin will:
1. `fetch("http://localhost:7437/health")` with 2s timeout
2. If fails → `$.spawn("the-crab-engram", ["serve", "--port", "7437"])` in background
3. Retry health check 3x with 2s interval
4. If still fails → graceful degradation (log warning, plugin works in reduced mode)

**Alternatives considered**:
- Use `pgrep`/`tasklist` — rejected: platform-specific, unreliable
- Start on every plugin load — rejected: might conflict with existing server
- No auto-start — rejected: poor UX, requires manual server management

---

## R6: Push-Based Memory Injection Architecture

### Decision: Two-phase injection via system prompt transform

**Rationale**: OpenCode's `experimental.chat.system.transform` hook allows appending strings to the system prompt. The plugin will:

**Phase 1 (on `chat.message` hook)**:
- Extract text from user message
- Fast keyword search via `POST /search?query=...&limit=1` (HTTP, <50ms)
- If results found → cache the injection context

**Phase 2 (on `experimental.chat.system.transform`)**:
- Check cache for pending injection
- If present → `POST /inject` with full context + token budget
- Push result into `output.system.push(contextString)`

This avoids injecting on every message — only when relevant memories exist.

**Token budget**: Default 2000 tokens (~8000 chars), configurable via `ENGRAM_INJECT_BUDGET` env var.

**Rate limiting**: Max 1 injection per 30s per session. Skip messages < 5 chars.

---

## R7: Compaction Recovery Strategy

### Decision: Multi-source context injection via `experimental.session.compacting`

**Rationale**: When OpenCode compacts a session, the `experimental.session.compacting` hook fires with `output.context: string[]`. The plugin concatenates:

1. **Session context**: `GET /context?limit=10` — recent observations
2. **Capsules**: `GET /capsules` — synthesized knowledge capsules
3. **Anti-patterns**: `GET /antipatterns` — warnings about recurring issues
4. **Recovery instructions**: Static markdown block telling the agent to call `mem_context` first

Each source is pushed as a separate string in `output.context`. The hook also supports `output.prompt` to replace the entire compaction prompt, but we use `context` only (preserves default behavior).

---

## R8: TypeScript Plugin Build & Distribution

### Decision: Local file copy via setup command, npm packaging deferred

**Rationale**: The plugin is a single TypeScript file that OpenCode loads directly (Bun runtime, no build step needed). The `setup opencode` command copies it to the plugins directory. This keeps the distribution simple — no npm build pipeline required for MVP.

**Future (post-MVP)**: Publish as npm package `the-crab-engram-opencode` for `plugin: ["the-crab-engram-opencode"]` in config.

---

## R9: Constitution Compliance Analysis

### I. Modularidad Cruda (Crate-First)
- New Rust modules: `opencode_paths.rs`, `config_merge.rs`, `plugin_template.rs` → all in `crates/mcp/` (correct crate)
- New HTTP endpoints in `crates/api/src/lib.rs` (correct crate)
- TypeScript plugin is an external artifact, not a Rust crate → NO VIOLATION

### II. Conocimiento Tipificado
- No new observation types needed → NO VIOLATION

### III. Storage Trait Sagrado
- Plugin accesses data via HTTP API → respects Storage Trait → NO VIOLATION
- Setup CLI only modifies config files, not database → NO VIOLATION

### IV. TDD: Rojo, Verde, Engram
- **ATTENTION**: Need integration tests for config merge, setup flow, and doctor checks
- Tests in `tests/` directory → PLAN TO ADDRESS

### V. Seguridad ChaCha
- Plugin communicates via localhost HTTP only → NO VIOLATION
- No data at rest affected → NO VIOLATION

### Async-First
- Setup CLI is synchronous file operations (acceptable for CLI tools) → NO VIOLATION
- HTTP endpoints use async axum handlers → NO VIOLATION

### Binario Estático
- TypeScript plugin is copied as a resource file, not linked into binary
- **JUSTIFICATION**: The plugin is an external configuration artifact, not part of the Rust compilation. Same as the existing `plugins/hooks/` shell scripts.

### Privacidad Local
- Plugin only communicates with localhost → NO VIOLATION
