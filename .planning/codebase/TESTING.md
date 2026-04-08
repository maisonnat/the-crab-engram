# Testing Patterns

**Analysis Date:** 2026-04-08

## Test Framework

**Runner:**
- Built-in Rust `#[test]` framework (no external test runner)
- No custom test configuration files

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros
- `matches!` macro for enum variant matching (e.g., `assert!(matches!(result, Err(EngramError::Duplicate(_))))`)
- Float comparison via `f64::EPSILON`: `assert!((score - expected).abs() < f64::EPSILON)`

**Run Commands:**
```bash
cargo test --workspace          # Run all tests across all crates
cargo test -p engram-store      # Run tests for a single crate
cargo test -p engram-core observation  # Run tests matching a filter
```

## Test File Organization

**Location:**
- Co-located unit tests: `#[cfg(test)] mod tests { ... }` at the bottom of each source file
- Integration tests: `tests/integration_store.rs` at workspace root

**Naming:**
- Unit test modules always named `tests` inside `#[cfg(test)]`
- Integration test files: `tests/integration_<area>.rs`
- Test functions: `snake_case` descriptive names (e.g., `full_session_workflow`, `dedup_within_window`, `soft_delete_preserves_data`)

**Files with `#[cfg(test)]` modules (34 total):**

| Crate | File | Test Focus |
|-------|------|------------|
| core | `observation.rs` | Type roundtrip, defaults, hashing |
| core | `score.rs` | Decay scoring, frequency, lifecycle multiplier |
| core | `crypto.rs` | Encrypt/decrypt roundtrip, key derivation |
| core | `topic.rs` | Slugify, topic key suggestion |
| core | `attachment.rs` | Attachment serialization |
| core | `permissions.rs` | Permission rules |
| core | `memory.rs` | Memory classification |
| core | `boundary.rs` | Knowledge boundaries |
| core | `stream.rs` | Event throttle |
| core | `entity.rs` | Entity extraction |
| core | `graph.rs` | Edge/relationship types |
| core | `session.rs` | Session lifecycle |
| core | `belief.rs` | Belief state management |
| core | `lifecycle.rs` | Lifecycle policies |
| core | `salience.rs` | Salience scoring |
| core | `capsule.rs` | Knowledge capsules |
| core | `compaction.rs` | Compaction levels |
| store | `sqlite.rs` | CRUD, search, dedup, export/import, attachments |
| store | `migration.rs` | Schema migrations |
| learn | `consolidation.rs` | Consolidation engine |
| learn | `anti_pattern.rs` | Anti-pattern detection |
| learn | `capsule_builder.rs` | Capsule synthesis |
| learn | `graph_evolver.rs` | Graph evolution |
| learn | `smart_injector.rs` | Context injection |
| learn | `boundary_tracker.rs` | Boundary tracking |
| learn | `stream_engine.rs` | Memory stream |
| learn | `spaced_review.rs` | Spaced repetition |
| learn | `salience_infer.rs` | Salience inference |
| search | `embedder.rs` | Embedding metadata |
| search | `hybrid.rs` | Reciprocal rank fusion |
| sync | `crdt.rs` | CRDT operations |
| sync | `chunk.rs` | Chunk export/import |
| api | `lib.rs` | API error serialization |
| tui | `app.rs` | TUI application |

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let store = SqliteStore::in_memory().unwrap();
        let sid = store.create_session("test").unwrap();

        // Act
        let id = store.insert_observation(&params).unwrap();

        // Assert
        assert!(id > 0);
    }
}
```

**Patterns:**
- Setup via helper functions: `fn setup_test_store() -> (Arc<SqliteStore>, String)` (see `crates/learn/src/consolidation.rs`)
- Inline setup — most tests create their own `SqliteStore::in_memory()` and session at the start
- No shared test fixtures or test data files
- No teardown needed — in-memory SQLite drops automatically

## Mocking

**Framework:** None — no mocking framework used

**Approach:** Real in-memory SQLite for all tests
```rust
let store = SqliteStore::in_memory().unwrap();
```

**What to Mock:** N/A — the project uses real `SqliteStore::in_memory()` as the test double. The `Storage` trait enables future mocking but none is implemented.

**What NOT to Mock:**
- Storage layer — always use `SqliteStore::in_memory()`
- Core types — always use real `Observation`, `Session`, `Edge`, etc.
- No external API calls to mock — all external integrations (embedding, vector search) are stubbed

## In-Memory Test Store

The primary test infrastructure is `SqliteStore::in_memory()` from `crates/store/src/sqlite.rs`:

```rust
/// Create an in-memory store (for testing).
pub fn in_memory() -> crate::Result<Self> {
    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|e| EngramError::Database(e.to_string()))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )
    .map_err(|e| EngramError::Database(e.to_string()))?;
    migration::run_migrations(&conn)?;
    Ok(Self { conn: Mutex::new(conn) })
}
```

This provides a fully functional store with migrations applied — every test gets a clean database.

## Fixtures and Factories

**Test Data:**
- Inline struct construction — no factory functions or builder patterns for test data
- `..Default::default()` used extensively to minimize boilerplate
```rust
let params = AddObservationParams {
    r#type: ObservationType::Bugfix,
    scope: Scope::Project,
    title: "Fix N+1 query".into(),
    content: "Used eager loading".into(),
    session_id: sid,
    project: "test".into(),
    topic_key: Some("bug/n1-query".into()),
    ..Default::default()
};
```

**Common Setup Helper (example from `crates/learn/src/consolidation.rs`):**
```rust
fn setup_test_store() -> (Arc<SqliteStore>, String) {
    let store = Arc::new(SqliteStore::in_memory().unwrap());
    let sid = store.create_session("test").unwrap();
    (store, sid)
}
```

**Location:** Helpers co-located in `#[cfg(test)] mod tests` blocks.

## Coverage

**Requirements:** None enforced — no coverage tooling detected

**View Coverage:**
```bash
# Manual approach with cargo-tarpaulin (not configured)
cargo tarpaulin --workspace
```

## Test Types

**Unit Tests:**
- Located in `#[cfg(test)] mod tests` within each source file
- Test individual functions and methods in isolation
- Examples: scoring algorithms, crypto roundtrip, topic slugification, enum parsing
- 34 test modules across all crates

**Integration Tests:**
- Single file: `tests/integration_store.rs` (293 lines)
- Tests full store workflow: session → insert → search → update → timeline → stats → export → import
- Tests cross-cutting concerns: dedup, soft delete, graph edges, prompt storage, type filtering
- Uses `SqliteStore::in_memory()` — no external dependencies

**E2E Tests:** Not used

## Common Patterns

**Async Testing:**
- Not used — all tests are synchronous `#[test]`
- Even though the binary uses `#[tokio::main]`, all store/learn/core operations are sync

**Error Testing:**
```rust
// Test that an operation returns an error
let result = store.insert_observation(&params);
assert!(result.is_err());

// Test specific error variant
assert!(matches!(result, Err(EngramError::Duplicate(_))));

// Test invalid input parsing
let result: Result<ObservationType, _> = "invalid_type".parse();
assert!(result.is_err());
```

**Float Comparison:**
```rust
// Compare floats with epsilon tolerance
assert!((score - expected).abs() < f64::EPSILON);
assert!((edges[0].weight - 0.9).abs() < f64::EPSILON);
```

**Roundtrip Testing:**
```rust
// Parse → Display → Parse roundtrip
for t in &types {
    let s = t.to_string();
    let parsed: ObservationType = s.parse().unwrap();
    assert_eq!(*t, parsed, "roundtrip failed for {s}");
}
```

**Export/Import Roundtrip:**
```rust
let data = store.export(None).unwrap();
let store2 = SqliteStore::in_memory().unwrap();
let result = store2.import(&data).unwrap();
assert_eq!(result.observations_imported, 2);
```

**Enum Pattern Match Assertions:**
```rust
match &attachments[0] {
    Attachment::ErrorTrace { message, .. } => {
        assert_eq!(message, "index out of bounds");
    }
    _ => panic!("expected ErrorTrace"),
}
```

## CI Configuration

**File:** `.github/workflows/ci.yml`

**Jobs:**
1. **test** — runs on `ubuntu-latest`, `windows-latest`, `macos-latest`
   - `cargo check --workspace`
   - `cargo clippy --workspace -- -D warnings`
   - `cargo test --workspace`
2. **lint** — `cargo fmt --all -- --check`

**Release:** `.github/workflows/release.yml` — builds for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`

---

*Testing analysis: 2026-04-08*
