# Codebase Structure

**Analysis Date:** 2026-04-08

## Directory Layout

```
engram-rust/
├── src/
│   └── main.rs                    # Binary entry point, CLI definitions
├── crates/
│   ├── core/src/                  # Domain types and pure logic (no I/O)
│   │   ├── lib.rs                 # Public re-exports
│   │   ├── observation.rs         # Observation type, enum, struct
│   │   ├── graph.rs               # Edge, RelationType
│   │   ├── entity.rs              # Entity, EntityType, extract_entities
│   │   ├── capsule.rs             # KnowledgeCapsule
│   │   ├── belief.rs              # Belief state machine
│   │   ├── memory.rs              # EpisodicMemory, SemanticMemory, classify_query_type
│   │   ├── session.rs             # Session struct
│   │   ├── boundary.rs            # KnowledgeBoundary, ConfidenceLevel
│   │   ├── salience.rs            # MemorySalience (decay multiplier)
│   │   ├── compaction.rs          # CompactionLevel, determine_level
│   │   ├── lifecycle.rs           # LifecyclePolicy per observation type
│   │   ├── permissions.rs         # PermissionEngine, AccessLevel
│   │   ├── crypto.rs              # encrypt/decrypt/derive_key (ChaCha20Poly1305)
│   │   ├── stream.rs              # MemoryEvent, EventThrottle, NotificationThrottle
│   │   ├── topic.rs               # slugify, suggest_topic_key
│   │   ├── score.rs               # compute_final_score, decay_score
│   │   ├── error.rs               # EngramError (the only error type)
│   │   └── attachment.rs          # Attachment enum, MultimodalObservation
│   ├── store/src/                 # Persistence layer
│   │   ├── lib.rs                 # Public re-exports
│   │   ├── trait.rs               # Storage trait (~40 methods, 316 lines)
│   │   ├── sqlite.rs              # SqliteStore implementation (~1400 lines)
│   │   ├── params.rs              # AddObservationParams, UpdateObservationParams, SearchOptions, etc.
│   │   └── migration.rs           # Schema migrations
│   ├── mcp/src/                   # MCP protocol server
│   │   ├── lib.rs                 # Public re-exports
│   │   ├── server.rs              # EngramServer, EngramConfig, ToolProfile
│   │   └── tools/
│   │       └── mod.rs             # 31 tool definitions + handlers (~1800 lines)
│   ├── api/src/
│   │   └── lib.rs                 # Axum REST API router + handlers (428 lines)
│   ├── tui/src/                   # Terminal UI
│   │   ├── lib.rs                 # run_tui entry point
│   │   └── app.rs                 # App state, AppState enum, draw/renderers
│   ├── search/src/                # Search and embeddings
│   │   ├── lib.rs                 # Public re-exports
│   │   ├── embedder.rs            # Embedder (fastembed), EmbeddingMeta, cosine_similarity
│   │   └── hybrid.rs              # reciprocal_rank_fusion, compute_relevance_score
│   ├── learn/src/                 # Intelligence layer
│   │   ├── lib.rs                 # Public re-exports
│   │   ├── consolidation.rs       # ConsolidationEngine (duplicates, obsolete, contradictions, patterns)
│   │   ├── anti_pattern.rs        # AntiPatternDetector
│   │   ├── smart_injector.rs      # SmartInjector (context injection)
│   │   ├── capsule_builder.rs     # CapsuleBuilder, CapsuleSynthesizer, HeuristicSynthesizer
│   │   ├── graph_evolver.rs       # GraphEvolver (auto edge creation)
│   │   ├── spaced_review.rs       # SpacedRepetition (SM-2 algorithm)
│   │   ├── stream_engine.rs       # MemoryStream (event detection)
│   │   ├── boundary_tracker.rs    # BoundaryTracker
│   │   └── salience_infer.rs      # infer_salience
│   └── sync/src/                  # Cross-device sync
│       ├── lib.rs                 # Public re-exports
│       ├── crdt.rs                # CrdtState, SyncDelta, LWW conflict resolution
│       └── chunk.rs               # export_chunks, import_chunks (JSONL.gz)
├── tests/
│   └── integration_store.rs       # Integration tests for Storage trait
├── docs/                          # Documentation
├── plugins/                       # Plugin definitions
├── scripts/                       # Utility scripts
├── assets/                        # Static assets
├── Cargo.toml                     # Workspace root (members = ["crates/*"])
├── Cargo.lock                     # Dependency lockfile
└── README.md
```

## Directory Purposes

**`src/` (Binary Root):**
- Purpose: Single binary entry point — CLI glue code
- Contains: `main.rs` (693 lines) — clap CLI, all subcommands, `build_export_context`, `generate_skill_md`
- Key files: `src/main.rs`

**`crates/core/src/`:**
- Purpose: Domain model — pure types, enums, scoring, crypto. ZERO I/O or database concerns
- Contains: 19 modules covering the full domain vocabulary
- Key files: `observation.rs` (313 lines, the central type), `error.rs` (36 lines), `graph.rs`, `belief.rs`, `capsule.rs`
- Pattern: All enums implement `Display` + `FromStr` + `Serialize` + `Deserialize`

**`crates/store/src/`:**
- Purpose: Persistence — the `Storage` trait port and `SqliteStore` adapter
- Contains: 4 modules — `trait.rs` (THE abstraction), `sqlite.rs` (implementation), `params.rs` (structured params), `migration.rs`
- Key files: `trait.rs` (316 lines), `sqlite.rs` (~1400 lines)
- Pattern: All methods take param structs, return core types, wrap errors as `EngramError`

**`crates/mcp/src/`:**
- Purpose: MCP protocol transport — tools, resources, streaming
- Contains: `server.rs` (EngramServer + ServerHandler impl), `tools/mod.rs` (31 tools + dispatch)
- Key files: `tools/mod.rs` (~1800 lines — largest single file)

**`crates/api/src/`:**
- Purpose: HTTP REST API via Axum
- Contains: Single file with router, handlers, request/response types
- Key file: `lib.rs` (428 lines)

**`crates/tui/src/`:**
- Purpose: Interactive terminal UI
- Contains: `lib.rs` (run_tui), `app.rs` (App + draw functions)
- Key file: `app.rs` (511 lines)

**`crates/search/src/`:**
- Purpose: Embedding generation and hybrid search ranking
- Contains: `embedder.rs` (fastembed wrapper), `hybrid.rs` (RRF fusion)
- Key file: `embedder.rs` (308 lines)

**`crates/learn/src/`:**
- Purpose: All intelligence — consolidation, anti-patterns, injection, synthesis, spaced repetition
- Contains: 9 modules, each focused on one intelligence capability
- Key files: `consolidation.rs` (510 lines), `smart_injector.rs`, `capsule_builder.rs`

**`crates/sync/src/`:**
- Purpose: Cross-device data sync via compressed JSONL chunks
- Contains: `crdt.rs` (vector clock + LWW), `chunk.rs` (gzip export/import)
- Key files: `chunk.rs` (226 lines)

**`tests/`:**
- Purpose: Cross-crate integration tests
- Contains: `integration_store.rs` (293 lines) — full workflow, dedup, soft delete, prompts, graph edges, type filtering

## Key File Locations

**Entry Points:**
- `src/main.rs`: Binary entry point — CLI, all subcommands, `[tokio::main]`
- `crates/mcp/src/server.rs`: MCP server entry — `EngramServer::serve_stdio()`
- `crates/api/src/lib.rs`: HTTP server entry — `create_router(state)`
- `crates/tui/src/lib.rs`: TUI entry — `run_tui(store, project)`

**Configuration:**
- `Cargo.toml`: Workspace config, dependency versions, build profiles
- `crates/*/Cargo.toml`: Per-crate dependencies and features
- No `.env` files detected, no config files — CLI args only

**Core Logic:**
- `crates/core/src/observation.rs`: Central domain type
- `crates/store/src/trait.rs`: Storage port (THE abstraction boundary)
- `crates/store/src/sqlite.rs`: Storage adapter
- `crates/mcp/src/tools/mod.rs`: All 31 MCP tools

**Testing:**
- Unit tests: Inline `#[cfg(test)] mod tests` in most core and store modules
- Integration tests: `tests/integration_store.rs`

## Naming Conventions

**Files:**
- Modules: `snake_case.rs` (e.g., `observation.rs`, `smart_injector.rs`)
- Entry points: `lib.rs` (library), `main.rs` (binary), `mod.rs` (directory modules)
- No component/prefix naming — domain-driven names

**Directories:**
- Crates: `engram-{name}` in Cargo.toml, `{name}/` in filesystem
- Source: `crates/{name}/src/`

**Structs:**
- `PascalCase`: `Observation`, `SqliteStore`, `EngramServer`, `SmartInjector`
- No prefixes — rely on module paths for disambiguation

**Enums:**
- `PascalCase` variants: `ObservationType::Bugfix`, `Scope::Project`, `RelationType::CausedBy`
- All implement `Display` (snake_case string) + `FromStr` roundtrip

**Functions:**
- `snake_case`: `insert_observation`, `get_session_context`, `decay_score`
- Tool handlers: `tool_{name}_handler` pattern (e.g., `tool_save_handler`)
- Tool definitions: `tool_{name}` pattern (e.g., `tool_save`)

**Trait Methods:**
- CRUD pattern: `insert_`, `get_`, `peek_` (no side-effects), `update_`, `delete_`
- Query pattern: `search`, `get_edges`, `get_related`, `get_timeline`
- State pattern: `create_session`, `end_session`

## Where to Add New Code

**New Observation Type:**
1. Add variant to `ObservationType` enum in `crates/core/src/observation.rs`
2. Add `Display` + `FromStr` arms
3. Add lifecycle policy in `crates/core/src/lifecycle.rs` `for_type()`
4. Add to `all_defaults()` array

**New MCP Tool:**
1. Add `tool_{name}()` definition function in `crates/mcp/src/tools/mod.rs`
2. Add `tool_{name}_handler()` async function
3. Add to `all_tool_definitions()` vector
4. Add to `dispatch_tool()` match arm
5. If admin-only, add name to `is_tool_allowed()` filter in `server.rs`

**New REST Endpoint:**
1. Add handler function in `crates/api/src/lib.rs`
2. Add route to `create_router()`
3. Add request/response types if needed

**New Storage Method:**
1. Add method signature to `Storage` trait in `crates/store/src/trait.rs`
2. Implement in `SqliteStore` in `crates/store/src/sqlite.rs`
3. Add SQL migration if schema changes needed in `crates/store/src/migration.rs`

**New Domain Type:**
1. Create module in `crates/core/src/` (e.g., `new_type.rs`)
2. Add `pub mod new_type;` to `crates/core/src/lib.rs`
3. Add `pub use new_type::NewType;` re-export
4. Implement `Display`, `FromStr`, `Serialize`, `Deserialize`

**New Intelligence Feature:**
1. Create module in `crates/learn/src/` (e.g., `new_feature.rs`)
2. Add `pub mod new_feature;` to `crates/learn/src/lib.rs`
3. Add public re-exports
4. Depend on `engram-core` types + `engram-store::Storage`
5. Wire into MCP tool handler in `crates/mcp/src/tools/mod.rs`

**Shared Utilities:**
- Core domain utils: `crates/core/src/` (e.g., `topic.rs`, `score.rs`)
- No separate `utils` crate — utilities are domain-specific modules

## Special Directories

**`crates/mcp/src/tools/`:**
- Purpose: All MCP tool definitions and handlers
- Generated: No
- Committed: Yes
- Note: Largest code concentration (~1800 lines) — single `mod.rs` file

**`tests/`:**
- Purpose: Cross-crate integration tests
- Generated: No
- Committed: Yes
- Note: Only `integration_store.rs` currently — most tests are inline `#[cfg(test)]`

**`target/`:**
- Purpose: Build artifacts
- Generated: Yes
- Committed: No (`.gitignore`)

**`docs/`:**
- Purpose: Documentation files
- Generated: No
- Committed: Yes

---

*Structure analysis: 2026-04-08*
