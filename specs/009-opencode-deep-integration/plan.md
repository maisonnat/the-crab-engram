# Implementation Plan: OpenCode Deep Integration

**Branch**: `009-opencode-deep-integration` | **Date**: 2026-04-15 | **Spec**: `specs/009-opencode-deep-integration/spec.md`
**Input**: Feature specification from `C:\Users\maiso\Downloads\locura\openspec-opencode-setup\2026-04-15-opencode-setup\spec.md`

## Summary

Deep integration between `the-crab-engram` and OpenCode via a TypeScript plugin that auto-starts the HTTP server, captures session lifecycle events, injects memory context via push-based injection, and handles compaction recovery. The `setup opencode` CLI command is enhanced to merge MCP config, copy the plugin, and inject the Memory Protocol. A `doctor opencode` diagnostic command verifies the full integration chain.

## Technical Context

**Language/Version**: Rust 1.85+ (edition 2024) + TypeScript (Bun runtime, ES2022)
**Primary Dependencies**: clap, serde_json, dirs, axum, @opencode-ai/plugin v1.4.6, @opencode-ai/sdk v2
**Storage**: SQLite via engram-store (Storage trait) — no new storage needed
**Testing**: cargo test (integration tests in `tests/`), manual plugin loading in OpenCode
**Target Platform**: Cross-platform (Linux, macOS, Windows)
**Project Type**: CLI tool with HTTP API server + TypeScript plugin
**Performance Goals**: Plugin init < 2s, push injection < 50ms, health check < 100ms
**Constraints**: No npm build step for MVP (Bun loads TS directly), localhost HTTP only
**Scale/Scope**: Single user, single project per session, ~32 MCP tools exposed

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|---|---|---|
| I. Modularidad Cruda | PASS | New modules in `crates/mcp/` (paths, merge, template) and `crates/api/` (endpoints). Plugin TS is external artifact. |
| II. Conocimiento Tipificado | PASS | No new observation types. Plugin uses HTTP API with existing types. |
| III. Storage Trait Sagrado | PASS | Plugin communicates via HTTP API which respects Storage Trait. Setup CLI touches config files only. |
| IV. TDD: Rojo, Verde, Engram | ATTENTION | Must write integration tests for: config merge, setup flow, doctor checks, new HTTP endpoints. |
| V. Seguridad ChaCha | PASS | No data at rest affected. Localhost HTTP only. |
| Async-First | PASS | HTTP endpoints use async axum handlers. Setup CLI is sync file ops (acceptable for CLI). |
| Binario Estático | JUSTIFIED | Plugin TS is copied as resource file (same as existing `plugins/hooks/` shell scripts). Not linked into binary. |
| Privacidad Local | PASS | All communication is localhost. No telemetry. |

**Post-Phase 1 re-check**: No violations found in design. All Constitution principles satisfied.

## Project Structure

### Documentation (this feature)

```text
specs/009-opencode-deep-integration/
├── plan.md              # This file
├── research.md          # Phase 0 — research findings
├── data-model.md        # Phase 1 — entities and validation
├── quickstart.md        # Phase 1 — setup guide
├── contracts/
│   └── api-contracts.md # Phase 1 — API + CLI + plugin hook contracts
└── tasks.md             # Phase 2 (/speckit.tasks output)
```

### Source Code (repository root)

```text
crates/
├── api/src/lib.rs           # MODIFY: add /health, /sessions/:id/end endpoints
└── mcp/src/
    ├── lib.rs               # MODIFY: add mod declarations
    ├── opencode_paths.rs    # NEW: OpenCodePaths struct + path detection
    ├── config_merge.rs      # NEW: JSON/JSONC merge, MCP entry, AGENTS.md merge
    ├── plugin_template.rs   # NEW: embed + write + remove plugin TS
    └── doctor.rs            # NEW: DoctorCheck enum + 7 checks + --fix

plugins/
└── opencode/
    └── the-crab-engram.ts   # NEW: TypeScript plugin with all hooks

src/
└── main.rs                  # MODIFY: enhanced setup opencode, new doctor command

tests/
└── opencode_setup_test.rs   # NEW: integration tests for setup + merge + doctor
```

**Structure Decision**: Single Rust project with new modules in existing crates. Plugin TS file lives in `plugins/opencode/` (mirrors existing `plugins/hooks/` pattern).

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Plugin TS not in binary | OpenCode requires a TS file loadable by Bun; cannot embed TS in Rust binary | In-process plugin via WASM — would require Bun WASM support that doesn't exist |
| JSONC parsing (strips `//` comments) | OpenCode config may be `.jsonc` format | Require `.json` only — breaks users with existing `.jsonc` configs |

## Implementation Phases

### Phase 0: HTTP API Prerequisites

**Goal**: Add endpoints the plugin needs for health check and session management.

**Files to modify**:
- `crates/api/src/lib.rs` — add `GET /health`, `POST /sessions/:id/end`

**Changes**:
1. Add `GET /health` route returning `{ "status": "ok", "version": env!("CARGO_PKG_VERSION") }`
2. Add `POST /sessions/:id/end` route accepting `{ "summary?": "string" }` → calls `store.end_session()`
3. Add unit tests for both endpoints

### Phase 1: Setup CLI Enhancement

**Goal**: `the-crab-engram setup opencode` does real setup with config merge, plugin copy, and AGENTS.md injection.

**Files to create**:
- `crates/mcp/src/opencode_paths.rs` — `OpenCodePaths::detect()`, `ensure_dirs()`, `detect_json_format()`
- `crates/mcp/src/config_merge.rs` — `merge_mcp_entry()`, `remove_mcp_entry()`, `merge_agents_md()`, JSONC support
- `crates/mcp/src/plugin_template.rs` — `write_plugin()`, `remove_plugin()`, hash-check

**Files to modify**:
- `crates/mcp/src/lib.rs` — add `mod opencode_paths; mod config_merge; mod plugin_template;`
- `src/main.rs` — enhance `Commands::Setup { agent: Opencode }` handler + add `--uninstall`, `--dry-run`, `--profile`, `--project` flags

### Phase 2: TypeScript Plugin

**Goal**: Full plugin that auto-starts server, handles session lifecycle, push injection, and compaction recovery.

**Files to create**:
- `plugins/opencode/the-crab-engram.ts` — single file implementing all hooks

**Plugin hooks**:
1. Auto-start: health check → spawn server if needed
2. `event` hook: `session.created` → create session, `session.idle` → consolidate, `session.deleted` → end session
3. `experimental.session.compacting` → inject context + capsules + anti-patterns
4. `experimental.chat.system.transform` → inject Memory Protocol + cached context
5. `chat.message` → search + inject relevant memories (push injection)
6. `tool.execute.after` → detect git commits and errors, auto-capture

### Phase 3: Doctor Command + Polish

**Goal**: Diagnostic command and documentation.

**Files to create**:
- `crates/mcp/src/doctor.rs` — `DoctorCheck` enum, 7 checks, `--fix` support
- `docs/en/opencode-setup.md` — documentation

**Files to modify**:
- `src/main.rs` — add `Commands::Doctor { agent }` subcommand
- `README.md` — add OpenCode to agent table

## Key Technical Decisions

1. **HTTP over CLI**: Plugin calls HTTP API endpoints, not CLI subcommands. Faster, typed responses, no process spawn overhead.
2. **Two-phase injection**: `chat.message` searches and caches; `system.transform` actually injects. Prevents duplicate injections.
3. **Graceful degradation**: If server can't start, plugin logs warning but doesn't crash. Reduced functionality mode.
4. **Idempotent setup**: Running setup twice produces same result. Hash-check prevents unnecessary writes.
5. **Bun-native TS**: No build step. OpenCode's Bun runtime loads `.ts` files directly.
