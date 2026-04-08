# Coding Conventions

**Analysis Date:** 2026-04-08

## Naming Patterns

**Files:**
- `snake_case` for all Rust source files (e.g., `observation.rs`, `smart_injector.rs`, `capsule_builder.rs`)
- `lib.rs` as crate root in every crate under `crates/*/src/`
- Module files use flat naming — no nested directories within `src/`

**Functions:**
- `snake_case` for all functions and methods
- Constructors named `new` (e.g., `SqliteStore::new`, `ConsolidationEngine::new`)
- Factory method `in_memory` for test constructors (see `SqliteStore::in_memory` in `crates/store/src/sqlite.rs`)
- Row mappers prefixed with `row_to_` (e.g., `row_to_observation`, `row_to_session`, `row_to_edge` in `crates/store/src/sqlite.rs`)
- Handler functions use `tool_<name>_handler` pattern (e.g., `tool_save_handler`, `tool_search_handler` in `crates/mcp/src/tools/mod.rs`)

**Variables:**
- `snake_case` for variables and struct fields
- Raw identifier `r#type` used extensively because `type` is a Rust keyword — used in `Observation`, `AddObservationParams`, `SearchOptions`, CLI structs

**Types:**
- `PascalCase` for structs, enums, traits (e.g., `SqliteStore`, `ObservationType`, `EngramError`, `Storage`)
- Enum variants are `PascalCase` (e.g., `Scope::Project`, `LifecycleState::Active`, `ProvenanceSource::LlmReasoning`)
- Serde rename to `snake_case` for all enums: `#[serde(rename_all = "snake_case")]`

**Crate naming:**
- Workspace crates prefixed with `engram-` (e.g., `engram-core`, `engram-store`, `engram-mcp`, `engram-learn`, `engram-search`, `engram-api`, `engram-tui`, `engram-sync`)
- Binary name: `the-crab-engram`

## Code Style

**Formatting:**
- Default `rustfmt` (no custom `rustfmt.toml` detected)
- CI enforces `cargo fmt --all -- --check` — formatting is mandatory
- 4-space indentation throughout

**Linting:**
- Clippy enforced with `-D warnings` in CI (`cargo clippy --workspace -- -D warnings`)
- Occasional `#[allow(clippy::type_complexity)]` for complex generic returns (see `crates/store/src/trait.rs` lines 266, 293)

## Import Organization

**Order:**
1. `std` imports first
2. External crate imports (e.g., `chrono`, `serde`, `rusqlite`, `tracing`, `axum`)
3. Workspace crate imports (e.g., `engram_core::`, `engram_store::`, `engram_learn::`)
4. Local module imports with `use crate::` or `use super::*` (in tests)

**Path Aliases:**
- No path aliases configured — all imports use full crate paths
- `use crate::r#trait::*` used in `crates/store/src/sqlite.rs` to import the Storage trait

## Error Handling

**Error Types:**
- Single unified error type: `EngramError` in `crates/core/src/error.rs`
- Uses `thiserror` 2.x for derive macros
- All `rusqlite::Error` mapped to `EngramError::Database(e.to_string())` at the boundary
- `EngramError::Other(#[from] anyhow::Error)` as catch-all

**Error Variants:**
- `Database(String)` — all SQL errors
- `NotFound(String)` — missing records
- `Duplicate(String)` — dedup violations
- `TooLong(String, usize)` — content validation
- `InvalidTopicKey(String)` — topic format errors
- `InvalidObservationType(String)` — parsing errors
- `Sync(String)` — sync/crdt errors
- `Embedding(String)` — vector search errors
- `Config(String)` — configuration errors
- `Serialization(#[from] serde_json::Error)` — JSON errors
- `Other(#[from] anyhow::Error)` — catch-all

**Result Type:**
- Custom `Result<T>` alias in `crates/store/src/trait.rs`: `pub type Result<T> = std::result::Result<T, EngramError>;`
- Used across all store operations

**Pattern: Always map at the boundary**
```rust
// rusqlite error → EngramError
.map_err(|e| EngramError::Database(e.to_string()))?;
```

**Pattern: .context() for main.rs**
```rust
// In main entry point, use anyhow::Context
let home = dirs::home_dir().context("could not determine home directory")?;
SqliteStore::new(&path).context("failed to open database")
```

**Pattern: match for user-facing error surfacing**
```rust
match store.insert_observation(&params) {
    Ok(id) => { /* success path */ }
    Err(e) => Ok(error_result(&format!("failed to save: {e}"))),
}
```

## Logging

**Framework:** `tracing` crate with `tracing-subscriber`

**Setup (in `src/main.rs`):**
```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
    )
    .init();
```

**Patterns:**
- `tracing::info!` for operations (e.g., `info!("SqliteStore opened at {:?}", path)` in `crates/store/src/sqlite.rs`)
- `tracing::warn!` for non-fatal issues (e.g., `warn!("Failed to store attachment: {e}")` in `crates/mcp/src/tools/mod.rs`)
- `#[instrument(skip(self), fields(project = %project))]` for tracing method calls (see `crates/learn/src/consolidation.rs`)

## Comments

**When to Comment:**
- Section separators with `// ── Section Name ──────` (extensively used in `crates/store/src/sqlite.rs`, `crates/store/src/trait.rs`, `crates/mcp/src/tools/mod.rs`)
- Doc comments (`///`) on all public types, traits, and methods
- Inline comments for non-obvious logic (e.g., dedup window, scoring weights)
- Feature markers as comments: `// ── F2+ Routes ──────`, `// (F2.5.9)`, `// (F3.7)`

**Doc Comments:**
- Always use `///` for public API documentation
- Include design rationale in doc comments (e.g., `/// THE firewall against vendor lock-in.` in `crates/store/src/trait.rs`)

## Struct Design

**Parameter structs over raw arguments:**
- All complex operations use dedicated param structs: `AddObservationParams`, `UpdateObservationParams`, `SearchOptions`, `AddEdgeParams`, `AddPromptParams` (all in `crates/store/src/params.rs`)
- Param structs implement `Default` for partial construction with `..Default::default()`

**Builder pattern via struct literals:**
```rust
let params = AddObservationParams {
    r#type: ObservationType::Bugfix,
    scope: Scope::Project,
    title: "Fix N+1 query".into(),
    content: "Used eager loading".into(),
    session_id: sid,
    project: "test".into(),
    ..Default::default()
};
```

**Trait-based abstraction:**
- `Storage` trait in `crates/store/src/trait.rs` — single trait with ~30 methods covering all storage operations
- All return types are from `engram-core` or `engram-store` (no rusqlite types leak through)
- All parameters are structs (no raw SQL strings)

## Module Design

**Exports:**
- Each crate's `lib.rs` re-exports key types: `pub use module::{Type1, Type2};`
- Module-level `pub mod` declarations in `lib.rs`
- Items made public at module level, re-exported at crate root

**Barrel files:**
- Every crate has `lib.rs` that serves as barrel file
- No nested barrel files — flat module structure

## Enum Patterns

**Serde + Display + FromStr:**
- All domain enums implement `Display`, `FromStr`, `Serialize`, `Deserialize`
- Use `#[serde(rename_all = "snake_case")]` for wire format
- `FromStr` returns `EngramError` on invalid input
- Display returns lowercase string matching wire format

**Example (from `crates/core/src/observation.rs`):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationType { Bugfix, Decision, ... }

impl std::fmt::Display for ObservationType { /* lowercase strings */ }
impl std::str::FromStr for ObservationType { /* returns EngramError */ }
```

## Concurrency

**Thread safety:**
- `SqliteStore` wraps `rusqlite::Connection` in `Mutex<rusqlite::Connection>`
- `MutexGuard` acquired via `self.conn()` helper, with `expect("sqlite connection mutex poisoned")`
- `Arc<dyn Storage>` for shared ownership across async handlers
- `Storage` trait requires `Send + Sync`

## Key Conventions Summary

| Aspect | Convention |
|--------|-----------|
| File naming | `snake_case.rs` |
| Function naming | `snake_case` |
| Type naming | `PascalCase` |
| Enum serde | `snake_case` |
| Error handling | `EngramError` unified type |
| Parameter passing | Dedicated param structs with `Default` |
| SQL error mapping | `.map_err(\|e\| EngramError::Database(e.to_string()))` |
| Logging | `tracing` crate |
| Formatting | Default rustfmt |
| Linting | Clippy with `-D warnings` |
| Section separators | `// ── Name ──────` |

---

*Convention analysis: 2026-04-08*
