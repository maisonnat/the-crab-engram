# Tasks: OpenCode Deep Integration

**Input**: Design documents from `specs/009-opencode-deep-integration/`
**Prerequisites**: plan.md, spec.md, data-model.md, contracts/api-contracts.md, research.md, quickstart.md

**Tests**: Not explicitly requested in spec. Tests included only for critical Rust modules (config merge, doctor checks) per Constitution IV (TDD: Rojo, Verde, Engram).

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Path Conventions

- Rust source: `crates/mcp/src/`, `crates/api/src/`, `src/`
- TypeScript plugin: `plugins/opencode/`
- Integration tests: `tests/`
- Documentation: `docs/en/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project structure initialization and HTTP API prerequisites for plugin communication.

- [X] T001 Create module declarations in `crates/mcp/src/lib.rs` for `opencode_paths`, `config_merge`, `plugin_template`, and `doctor`
- [X] T002 Add `GET /health` endpoint returning `{ "status": "ok", "version": env!("CARGO_PKG_VERSION") }` in `crates/api/src/lib.rs`
- [X] T003 [P] Add `POST /sessions/:id/end` route accepting `{ "summary?": "string" }` in `crates/api/src/lib.rs`
- [X] T003b [P] Write integration tests for HTTP endpoints: `GET /health` returns 200 + version, `POST /sessions/:id/end` returns 200 or 404 in `tests/opencode_setup_test.rs`
- [X] T004 [P] Add clap subcommand variants for `setup opencode` flags (`--uninstall`, `--dry-run`, `--profile`, `--project`) and `doctor opencode` with `--fix` in `src/main.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core Rust modules that ALL user stories depend on — path detection, config merge, plugin template, and doctor checks.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 Implement `OpenCodePaths` struct with `detect()`, `ensure_dirs()`, `detect_json_format()` in `crates/mcp/src/opencode_paths.rs` — resolution order: `$OPENCODE_CONFIG_DIR` → `dirs::config_dir()/opencode/` → `.opencode/`
- [X] T006 [P] Implement JSON/JSONC merge functions: `merge_mcp_entry()`, `remove_mcp_entry()`, `merge_agents_md()` in `crates/mcp/src/config_merge.rs` — strip `//` comments for JSONC, preserve existing keys, upsert MCP entry, append plugin path without duplicates
- [X] T007 [P] Implement plugin template functions: `write_plugin()`, `remove_plugin()`, with SHA-256 hash-check to skip unnecessary writes in `crates/mcp/src/plugin_template.rs` — embed TS file via `include_str!`
- [X] T008 [P] Implement `DoctorCheck` enum with 7 checks (BinaryInPath, OpencodeInstalled, ConfigExists, McpEntryValid, PluginExists, ServerRunning, DatabaseOk) + 1 probe check (McpToolsProbe via `POST /stats` to verify tool availability) + `--fix` support in `crates/mcp/src/doctor.rs` — each check returns `CheckResult` with status Pass/Fail/Warn
- [X] T009 Write integration test for config merge (preserves existing keys, JSONC strip, idempotent merge, uninstall removes entry) in `tests/opencode_setup_test.rs`
- [X] T010 Write integration test for doctor checks (all pass scenario, server down scenario, `--fix` auto-repair) in `tests/opencode_setup_test.rs`

**Checkpoint**: Foundation ready — path detection, config merge, plugin template, and doctor checks are functional and tested.

---

## Phase 3: User Story 1 — Cross-Platform Setup Command (Priority: P1) 🎯 MVP

**Goal**: `the-crab-engram setup opencode` detects OS, merges MCP config, copies plugin, injects Memory Protocol into AGENTS.md, and reports actions.

**Independent Test**: Run `the-crab-engram setup opencode` → verify `opencode.json` has MCP entry, plugin file exists, AGENTS.md contains Memory Protocol block. Run again → verify idempotent (Skipped actions).

### Implementation for User Story 1

- [X] T011 [US1] Implement `setup_opencode()` handler in `src/main.rs` using OpenCodePaths + config_merge + plugin_template modules — handles normal setup, `--dry-run`, `--uninstall`, `--profile`, and `--project` flags
- [X] T012 [US1] Implement `SetupResult` + `SetupAction` + `ActionKind` types and display as table output in `crates/mcp/src/config_merge.rs` (or dedicated display module)
- [X] T013 [US1] Implement AGENTS.md merge: find `<!-- gentle-ai:engram-protocol -->` / `<!-- engram-protocol-start -->` markers, replace block between markers; if missing, append Memory Protocol block at end — in `crates/mcp/src/config_merge.rs`
- [X] T014 [US1] Implement `--uninstall` flow: remove MCP entry from opencode.json, delete plugin file, preserve AGENTS.md — in `src/main.rs`
- [X] T015 [US1] Implement `--dry-run` flag: compute all actions but only print what would be done, exit 0 — in `src/main.rs`
- [X] T016 [US1] Write integration test for full setup flow (create config, merge, plugin copy, AGENTS.md inject, idempotent re-run) in `tests/opencode_setup_test.rs`
- [X] T017 [US1] Write integration test for uninstall flow (remove entry + plugin, preserve AGENTS.md) in `tests/opencode_setup_test.rs`

**Checkpoint**: `the-crab-engram setup opencode` fully functional — setup, uninstall, dry-run, idempotency all working.

---

## Phase 4: User Story 2 — TypeScript Plugin Lifecycle Hooks (Priority: P2)

**Goal**: TypeScript plugin that auto-starts server, handles session lifecycle events (created/idle/deleted), and manages compaction recovery.

**Independent Test**: Load plugin in OpenCode → verify session starts on `session.created`, consolidation triggers on `session.idle`, session ends on `session.deleted`.

### Implementation for User Story 2

- [X] T018 [US2] Create plugin file `plugins/opencode/the-crab-engram.ts` with `EngramPluginState` interface and `Plugin` export function per `@opencode-ai/plugin` v1.4.6 API
- [X] T019 [US2] Implement auto-start logic: health check `GET /health` with 2s timeout → spawn `the-crab-engram serve --port 7437` via BunShell if unreachable → retry 3x with 2s interval → graceful degradation with warning in `plugins/opencode/the-crab-engram.ts`
- [X] T020 [US2] Implement `event` hook: `session.created` → `POST /sessions`, `session.idle` → `POST /consolidate`, `session.deleted` → `POST /sessions/:id/end` in `plugins/opencode/the-crab-engram.ts`
- [X] T021 [US2] Implement `experimental.session.compacting` hook: fetch context (`GET /context?limit=10`), capsules (`GET /capsules`), anti-patterns (`GET /antipatterns`), push each as separate `output.context.push()` block with recovery instructions in `plugins/opencode/the-crab-engram.ts`
- [X] T022 [US2] Implement `experimental.chat.system.transform` hook: push Memory Protocol markdown into `output.system.push()`, inject cached pendingContext if available in `plugins/opencode/the-crab-engram.ts`

**Checkpoint**: Plugin handles full session lifecycle — auto-start, session events, compaction recovery, and system prompt injection.

---

## Phase 5: User Story 3 — Push-Based Memory Injection (Priority: P3)

**Goal**: Plugin searches memories on user messages and injects relevant context automatically. Skips injection when no relevant memories found (zero-cost path).

**Independent Test**: Send message referencing past work → plugin injects relevant context. Send unrelated message → no injection occurs. Verify token budget respected.

### Implementation for User Story 3

- [X] T023 [US3] Implement `chat.message` hook: extract text from user message → `POST /search { query, limit: 1, format: "ids-only" }` for fast keyword check → if results found, cache injection context in `pendingContext` state in `plugins/opencode/the-crab-engram.ts`
- [X] T024 [US3] Implement push injection in `system.transform`: if `pendingContext` cached → `POST /inject { task, max_tokens }` with budget from `ENGRAM_INJECT_BUDGET` env var (default 2000 tokens) → `output.system.push(result)` → clear cache in `plugins/opencode/the-crab-engram.ts`
- [X] T025 [US3] Implement rate limiting: max 1 injection per 30s per session, skip messages < 5 chars, zero-cost path when no search results in `plugins/opencode/the-crab-engram.ts`

**Checkpoint**: Push injection fully functional — relevant memories injected automatically, irrelevant queries cost zero tokens.

---

## Phase 6: User Story 4 — Auto-Project Bootstrapping (Priority: P3)

**Goal**: On first session for a project with zero observations, scan project and create initial knowledge boundaries. For returning projects, inject recent session context.

**Independent Test**: Open new project → verify bootstrapping context injected. Open existing project → verify `mem_context` from last 3 sessions injected.

### Implementation for User Story 4

- [X] T026 [US4] Implement first-session detection in `session.created` hook: `GET /stats` → if observations === 0 → `POST /observations { type: "discovery", content: <scan-project results> }` with passive scan of project directory → `POST /boundaries` to create initial knowledge boundaries → inject bootstrapping welcome context in `plugins/opencode/the-crab-engram.ts`
- [X] T027 [US4] Implement returning-session path: if project has existing observations → `GET /context?limit=3` → inject as welcome-back context message in `plugins/opencode/the-crab-engram.ts`

**Checkpoint**: Auto-bootstrapping works for both new and returning projects.

---

## Phase 7: User Story 5 — Auto-Capture (Git Commits & Errors) (Priority: P3)

**Goal**: Plugin detects git commits and errors from tool output and auto-captures them as observations.

**Independent Test**: Make a git commit via OpenCode shell → verify observation auto-captured. Trigger an error → verify error observation captured.

### Implementation for User Story 5

- [X] T028 [US5] Implement `tool.execute.after` hook: detect bash/shell tool → parse output for "git commit" → `POST /observations { type: "file_change" }` → detect error patterns → `POST /observations { type: "bugfix" }` in `plugins/opencode/the-crab-engram.ts`
- [X] T028b [US5] Implement `file.edited` event handler in `event` hook: track file path → `POST /observations { type: "file_change", content: <file_path> }` in `plugins/opencode/the-crab-engram.ts`
- [X] T029 [US5] Implement rate limiting: max 1 capture per 2 seconds, ignore successful tool runs without patterns in `plugins/opencode/the-crab-engram.ts`

**Checkpoint**: Git commits and errors are auto-captured without manual intervention.

---

## Phase 8: User Story 6 — Doctor Command (Priority: P2)

**Goal**: `the-crab-engram doctor opencode` verifies full integration chain with 7 checks, outputs status table, and supports `--fix` auto-repair.

**Independent Test**: Run `the-crab-engram doctor opencode` with everything configured → all PASS. Stop server → run doctor → Server FAIL. Run with `--fix` → server restarted, all PASS.

### Implementation for User Story 6

- [X] T030 [US6] Implement `doctor_opencode()` command handler in `src/main.rs` — iterate DoctorCheck variants, collect CheckResults, display table with ✅/❌/⚠️ symbols, exit 0 if all pass or 1 if any fail
- [X] T031 [US6] Implement `--fix` flag: for each Fail status, attempt auto-repair (re-run setup, restart server, recreate plugin) — only fix Fail, not Warn — in `crates/mcp/src/doctor.rs`
- [X] T032 [US6] Write integration test for doctor command (all pass, server down, fix auto-repairs) in `tests/opencode_setup_test.rs`

**Checkpoint**: Doctor command fully functional with diagnostic table and auto-repair.

---

## Phase 9: User Story 7 — Profile-Aware Plugin (Priority: P3)

**Goal**: Plugin respects configured MCP profile. Push injection only uses read + search tools. Capture operations run with agent profile.

**Independent Test**: Configure plugin with `admin` profile → verify injection only uses read/search. Verify capture uses `agent` profile regardless.

### Implementation for User Story 7

- [X] T033 [US7] Implement profile awareness: read MCP profile from plugin config → `mem_inject` uses only read + search tools → capture operations (`tool.execute.after`) always use `agent` profile in `plugins/opencode/the-crab-engram.ts`

**Checkpoint**: Plugin respects profile boundaries for all operations.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, cross-platform validation, and final cleanup.

- [X] T034 [P] Create `docs/en/opencode-setup.md` with: one-command setup, manual fallback, plugin architecture, token budget config, troubleshooting guide
- [X] T035 [P] Update `README.md` agent setup table with OpenCode one-liner
- [X] T036 Validate cross-platform paths: test on Windows (`%APPDATA%`), verify `dirs::config_dir()` resolution on Linux/macOS
- [X] T037 [P] Run `cargo clippy` and `cargo test` to ensure no warnings or regressions
- [X] T038 Validate quickstart.md end-to-end: `setup opencode` → `doctor opencode` → open OpenCode → verify plugin loads

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 completion — BLOCKS all user stories
- **US1 Setup Command (Phase 3)**: Depends on Phase 2 (uses all foundational modules)
- **US2 Plugin Lifecycle (Phase 4)**: Depends on Phase 1 (HTTP endpoints) + Phase 3 (plugin file)
- **US3 Push Injection (Phase 5)**: Depends on Phase 4 (extends plugin hooks)
- **US4 Auto-Bootstrap (Phase 6)**: Depends on Phase 4 (extends session.created hook)
- **US5 Auto-Capture (Phase 7)**: Depends on Phase 4 (extends plugin hooks)
- **US6 Doctor Command (Phase 8)**: Depends on Phase 2 (DoctorCheck module)
- **US7 Profile-Aware (Phase 9)**: Depends on Phase 4 (extends plugin)
- **Polish (Phase 10)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: After Phase 2 — no dependencies on other stories
- **US2 (P2)**: After Phase 1 + Phase 3 (needs plugin file path for TS) — independently testable
- **US3 (P3)**: After Phase 4 (extends plugin) — independently testable
- **US4 (P3)**: After Phase 4 (extends session hook) — independently testable
- **US5 (P3)**: After Phase 4 (extends plugin hooks) — independently testable
- **US6 (P2)**: After Phase 2 — independently testable (can run in parallel with US1)
- **US7 (P3)**: After Phase 4 — independently testable

### Within Each User Story

- Tests before implementation (Constitution IV)
- Models/structs before services
- Services before CLI handlers
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- T002 + T003 + T004: All Phase 1 endpoints/CLI can run in parallel
- T006 + T007 + T008: All Phase 2 foundational modules can run in parallel (different files)
- T009 + T010: Integration tests can run in parallel
- US6 (Phase 8) can run in parallel with US2-US5 after Phase 2 is complete
- T034 + T035: Documentation tasks can run in parallel
- US3, US4, US5: Can run in parallel after Phase 4 (all modify same TS file but different functions)

---

## Parallel Example: Phase 2 (Foundational)

```bash
Task: "Implement OpenCodePaths struct in crates/mcp/src/opencode_paths.rs"
Task: "Implement config merge functions in crates/mcp/src/config_merge.rs"
Task: "Implement plugin template in crates/mcp/src/plugin_template.rs"
Task: "Implement DoctorCheck enum in crates/mcp/src/doctor.rs"
```

## Parallel Example: Phase 1 (Setup)

```bash
Task: "Add GET /health endpoint in crates/api/src/lib.rs"
Task: "Add POST /sessions/:id/end route in crates/api/src/lib.rs"
Task: "Add clap subcommand variants in src/main.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (HTTP endpoints + CLI structure)
2. Complete Phase 2: Foundational (path detection, config merge, plugin template, doctor)
3. Complete Phase 3: User Story 1 (setup opencode command)
4. **STOP and VALIDATE**: Run `the-crab-engram setup opencode` then `the-crab-engram doctor opencode`
5. Ship MVP — users can install and verify integration

### Incremental Delivery

1. Setup + Foundational → Core Rust infrastructure ready
2. Add US1 (Setup Command) → Users can run `setup opencode` → **MVP!**
3. Add US2 (Plugin Lifecycle) → Plugin auto-starts server and handles sessions
4. Add US3 (Push Injection) → Automatic memory context injection on messages
5. Add US6 (Doctor) → Diagnostic command for troubleshooting
6. Add US4, US5, US7 → Enhanced capture, bootstrapping, profile awareness
7. Polish → Documentation and cross-platform validation

### Parallel Team Strategy

1. Team completes Phase 1 + Phase 2 together
2. Once foundational is done:
   - Developer A: US1 (Setup Command) — highest priority
   - Developer B: US6 (Doctor Command) — independent of US1
3. After US1 completes:
   - Developer A: US2 (Plugin Lifecycle)
   - Developer B: US3-US5 (Plugin features — can be done sequentially in same TS file)
4. Polish together

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Tests written for critical Rust modules (config merge, doctor) per Constitution IV
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- TypeScript plugin is a single file (`plugins/opencode/the-crab-engram.ts`) — US2-US7 all modify it but add different functions
- No npm build step — Bun loads `.ts` files directly (per research R8)
- WRQ-10 (npm package) is explicitly out of scope for MVP — deferred post-launch per research R8
- `file.edited` event covered by T028b (WRQ-3 compliance)
- MCP tools probe included in DoctorCheck T008 (WRQ-7 compliance)
