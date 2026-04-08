# Architecture

**Analysis Date:** 2026-04-08

## Pattern Overview

**Overall:** Cargo Workspace with Hexagonal (Ports & Adapters) architecture

**Key Characteristics:**
- Single binary (`the-crab-engram`) composing multiple domain crates via workspace
- Storage-agnostic `Storage` trait (port) in `engram-store`, with `SqliteStore` as the concrete adapter
- Core domain types live exclusively in `engram-core` — no external crate leakage into type signatures
- Three transport layers (CLI, MCP stdio, HTTP REST) all converging on the same `Arc<dyn Storage>` + core domain types
- Clean dependency flow: transport layers depend on core + store + learn; learn depends on core + store + search; store depends on core

## Layers

**Domain Core (`engram-core`):**
- Purpose: Pure domain types, enums, scoring logic, crypto, error type — zero I/O
- Location: `crates/core/src/`
- Contains: `Observation`, `Edge`, `Session`, `Belief`, `KnowledgeCapsule`, `Entity`, `Attachment`, `KnowledgeBoundary`, `MemorySalience`, `LifecyclePolicy`, `PermissionEngine`, scoring/compaction/crypto utilities, `EngramError`
- Depends on: `serde`, `chrono`, `uuid`, `sha2`, `chacha20poly1305`, `thiserror`, `anyhow`
- Used by: ALL other crates

**Storage (`engram-store`):**
- Purpose: Persistence layer — the `Storage` trait (port) and `SqliteStore` (adapter)
- Location: `crates/store/src/`
- Contains: `Storage` trait (316 lines, ~40 methods), `SqliteStore` implementation, `Params` structs, migration engine, export/import types
- Depends on: `engram-core`, `rusqlite`, `serde`, `chrono`
- Used by: `engram-mcp`, `engram-api`, `engram-tui`, `engram-sync`, `engram-learn`, `engram-rust` (binary)

**Search (`engram-search`):**
- Purpose: Embedding generation and hybrid search (FTS5 + vector)
- Location: `crates/search/src/`
- Contains: `Embedder` (fastembed/all-MiniLM-L6-v2, 384d), `EmbeddingMeta`, `recipro_rank_fusion`, `compute_relevance_score`
- Depends on: `engram-core`, `fastembed`
- Used by: `engram-learn`

**Learn (`engram-learn`):**
- Purpose: Intelligence layer — consolidation, anti-pattern detection, smart injection, spaced repetition, capsule synthesis, stream events, graph evolution, salience inference
- Location: `crates/learn/src/`
- Contains: `ConsolidationEngine`, `AntiPatternDetector`, `SmartInjector`, `SpacedRepetition`, `CapsuleBuilder`/`CapsuleSynthesizer`, `GraphEvolver`, `MemoryStream`, `BoundaryTracker`, `infer_salience`
- Depends on: `engram-core`, `engram-store`, `engram-search`
- Used by: `engram-mcp`, `engram-api`

**MCP Server (`engram-mcp`):**
- Purpose: Model Context Protocol transport — 31 tools, 3 resources, streaming notifications
- Location: `crates/mcp/src/`
- Contains: `EngramServer` (implements `ServerHandler`), `EngramConfig`, `ToolProfile` (Agent/Admin/All), tool definitions and dispatch
- Depends on: `engram-core`, `engram-store`, `engram-learn`, `rmcp`, `tokio`
- Used by: `engram-rust` (binary)

**API Server (`engram-api`):**
- Purpose: HTTP REST transport via Axum
- Location: `crates/api/src/`
- Contains: `AppState`, `create_router`, REST handlers for CRUD/search/capsules/consolidate/graph/inject/antipatterns
- Depends on: `engram-core`, `engram-store`, `engram-learn`, `axum`, `tower-http`
- Used by: `engram-rust` (binary)

**TUI (`engram-tui`):**
- Purpose: Interactive terminal UI via ratatui/crossterm
- Location: `crates/tui/src/`
- Contains: `App` state machine, `AppState` enum (Dashboard/Search/Detail/Timeline/Capsules/Boundaries), `draw` renderer
- Depends on: `engram-core`, `engram-store`, `ratatui`, `crossterm`
- Used by: `engram-rust` (binary)

**Sync (`engram-sync`):**
- Purpose: Cross-device sync via CRDT-style chunks
- Location: `crates/sync/src/`
- Contains: `CrdtState` (device ID + vector clock), `SyncDelta`/`SyncOperation`, LWW conflict resolution, chunked export/import with gzip compression
- Depends on: `engram-core`, `engram-store`, `flate2`, `sha2`
- Used by: `engram-rust` (binary)

**Binary (`engram-rust`):**
- Purpose: CLI entry point — glues all crates together via clap subcommands
- Location: `src/main.rs`
- Contains: `Cli` struct, `Commands` enum (Mcp/Search/Save/Context/Stats/Timeline/Export/Import/ExportContext/SessionStart/SessionEnd/Serve/Tui/Consolidate/Sync/Encrypt/Setup/Version)
- Depends on: all workspace crates, `clap`, `tokio`, `axum`

## Data Flow

**Observation Creation (MCP path):**

1. Agent calls `mem_save` tool via MCP stdio
2. `tools/mod.rs` dispatches to `tool_save_handler` which validates params, builds `AddObservationParams`
3. `EngramServer.store.insert_observation(&params)` → `SqliteStore::insert_observation`
4. SQLite: SHA-256 hash dedup check (15-minute window), INSERT into `observations` table
5. Auto-classify into episodic/semantic tables
6. Auto-extract beliefs from content (`extract_and_upsert_beliefs`)
7. Anti-pattern check for bugfix types (3+ bugs on same file = warning)
8. Returns `CallToolResult` with observation ID + optional warnings

**Search Flow:**

1. Query enters via MCP tool, REST API, CLI, or TUI
2. `SqliteStore::search()` decides path based on query content:
   - Empty query: filter-only SQL with type/scope/project filters
   - Non-empty: FTS5 full-text search on `observations_fts` virtual table
3. Results reranked by `rerank_by_relevance()` (score = type_weight × lifecycle_weight × salience_decay × recency_weight × pin_bonus)
4. Vector search path available but requires sqlite-vec extension (currently returns empty gracefully)

**Smart Context Injection:**

1. Agent calls `mem_inject` with task description
2. `SmartInjector::build_context()` searches observations, extracts entities, checks knowledge boundaries
3. Returns `InjectionContext` with relevant memories, warnings, boundary info, markdown output

**Auto-Consolidation:**

1. MCP server spawns background task every 30 minutes
2. `ConsolidationEngine::run_consolidation()` runs 4-step pipeline:
   - merge_duplicates (hash + optional semantic similarity)
   - mark_obsolete (lifecycle-based)
   - find_contradictions (same topic, different values)
   - extract_patterns (cluster similar bugfixes → create Pattern observations)

**State Management:**
- `SqliteStore` holds a `Mutex<rusqlite::Connection>` — single-writer SQLite with WAL mode
- `Arc<dyn Storage>` shared across MCP server threads and auto-consolidation task
- No in-memory state except TUI `App` struct and MCP stream event channels

## Key Abstractions

**`Storage` trait (`crates/store/src/trait.rs`):**
- Purpose: THE abstraction that decouples all transport layers from SQLite
- Pattern: All return types use core types (no rusqlite leakage), all params are structs (no raw SQL), all errors are `EngramError`
- Methods: ~40 covering observations CRUD, sessions, prompts, timeline, stats, graph edges, embeddings, export/import, lifecycle, attachments, capsules, spaced repetition, boundaries, beliefs, entities, cross-project transfers, agent personality
- Implementation: `SqliteStore` in `crates/store/src/sqlite.rs`

**`Observation` (`crates/core/src/observation.rs`):**
- Purpose: The fundamental unit of memory — everything revolves around observations
- Fields: id, type (14 enum variants), scope (Project/Personal), title, content, session_id, project, topic_key, timestamps, access_count, pinned, normalized_hash, provenance (source/confidence/evidence), lifecycle_state, salience (emotional_valence/surprise_factor/effort_invested)
- Pattern: SHA-256 hash of title+content for dedup

**`Edge` (`crates/core/src/graph.rs`):**
- Purpose: Temporal relationship in the knowledge graph
- Pattern: Has validity windows — new edges auto-close previous active edges between same nodes with same relation type
- Relations: CausedBy, RelatedTo, Supersedes, Blocks, PartOf

**`KnowledgeCapsule` (`crates/core/src/capsule.rs`):**
- Purpose: Dense synthesis of knowledge by topic — what the system "understands"
- Fields: topic, summary, key_decisions, known_issues, anti_patterns, best_practices, source_observations, confidence, version
- Pattern: Upsert by (topic, project) with auto-increment version

**`Belief` (`crates/core/src/belief.rs`):**
- Purpose: Evolves with evidence — state machine (Active → Confirmed/Contested/Superseded/Retracted)
- Pattern: `process_evidence()` determines operation; `execute_operation()` mutates state with history preservation

**`EngramError` (`crates/core/src/error.rs`):**
- Purpose: Single error type across ALL crates
- Variants: Database, NotFound, Duplicate, TooLong, InvalidTopicKey, InvalidObservationType, Sync, Embedding, Config, Serialization, Other
- Pattern: No crate-specific error types leak across boundaries

**`MemoryEvent` (`crates/core/src/stream.rs`):**
- Purpose: Real-time streaming events for MCP notifications
- Variants: RelevantFileContext, AntiPatternWarning, DejaVu, KnowledgeUpdated, ReviewDue, EntityExtracted
- Pattern: Tagged enum with `#[serde(tag = "type")]` for JSON serialization

## Entry Points

**CLI Binary:**
- Location: `src/main.rs`
- Triggers: `the-crab-engram <subcommand>`
- Responsibilities: Parse CLI args, open SqliteStore, dispatch to appropriate crate

**MCP Server:**
- Location: `crates/mcp/src/server.rs` (`EngramServer`)
- Triggers: `the-crab-engram mcp` or via `serve_stdio()`
- Responsibilities: stdio transport, tool/resource dispatch, streaming notifications, auto-consolidation

**HTTP API:**
- Location: `crates/api/src/lib.rs` (`create_router`)
- Triggers: `the-crab-engram serve --port 7437`
- Responsibilities: REST endpoints, CORS, shared `AppState`

**TUI:**
- Location: `crates/tui/src/lib.rs` (`run_tui`)
- Triggers: `the-crab-engram tui`
- Responsibilities: Interactive terminal UI with Dashboard, Search, Detail, Timeline, Capsules, Boundaries views

## Error Handling

**Strategy:** Single `EngramError` type propagated through `Storage` trait's `Result<T>` alias

**Patterns:**
- SQLite errors wrapped: `rusqlite::Error` → `EngramError::Database(e.to_string())`
- JSON errors auto-converted: `#[from] serde_json::Error`
- Generic errors auto-converted: `#[from] anyhow::Error`
- MCP handlers return `ErrorData::invalid_params()` for bad input
- REST handlers return `ApiError { error: String }` with 400 status

## Cross-Cutting Concerns

**Logging:** `tracing` crate with `tracing-subscriber` + `EnvFilter` (default: warn level, configurable via `RUST_LOG`)
**Validation:** Inline in handler functions — type parsing via `FromStr`, required fields checked before store calls
**Authentication:** Not implemented — CLI is local-only, MCP is stdio transport
**Encryption:** ChaCha20Poly1305 file-level encryption in `crates/core/src/crypto.rs` (opt-in via `encrypt` subcommand)
**Deduplication:** SHA-256 hash of title+content with 15-minute window in `SqliteStore::insert_observation`

---

*Architecture analysis: 2026-04-08*
