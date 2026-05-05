# Fix Migration Deadlock / Duplicate Column Bug

> **Goal:** Fix the silent hang / failure when opening a DB with pending migrations due to `fix_schema_compat()` adding columns that migrations also add.

**Root Cause:** `fix_schema_compat()` in `crates/store/src/migration.rs` adds columns like `recorded_at`, `pinned`, `normalized_hash` to `observations` and `edges` tables via ALTER TABLE **before** the migration loop runs. When migration 017 or 018 later tries `ALTER TABLE ... ADD COLUMN`, the column already exists → error:

1. `run_migrations(conn)` calls `fix_schema_compat(conn)` (line 87)
2. `fix_schema_compat` adds `recorded_at` to observations + edges  
3. Migration loop reaches 017 → tries `ALTER TABLE ... ADD recorded_at` → **fails** (duplicate column)
4. Error propagates → `open_store()` returns Err → binary fails to start

**Fix Strategy (option C — cleanest):** After `fix_schema_compat` runs ALTER TABLE commands, record the corresponding migration versions in `_migrations` so the migration loop skips them.

---

### Task 1: Track applied schema-compat columns in _migrations

**Objective:** After `fix_schema_compat()` adds columns, insert the matching migration versions into `_migrations` so the migration loop skips them.

**Files:**
- Modify: `crates/store/src/migration.rs` (lines 147-220)

**Fix in `fix_schema_compat()`:**

Add logic after the ALTER TABLE commands (after line 216) to check which columns were actually added and INSERT corresponding migration versions:

```rust
// After applying ALTER TABLE commands (after line 216)
// If fix_schema_compat added columns that migrations also add,
// mark those migrations as applied to prevent duplicate ALTER TABLE errors.
if !obs_cols.iter().any(|c| c == "recorded_at") {
    // recorded_at was just added — migration 017 would fail if we try again
    let already: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 17",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if !already {
        let _ = conn.execute("INSERT OR IGNORE INTO _migrations (version) VALUES (17)", []);
    }
}
if !edge_cols.iter().any(|c| c == "recorded_at") {
    // Same as above — edges recorded_at comes from migration 017 too
    let already: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 17",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if !already {
        let _ = conn.execute("INSERT OR IGNORE INTO _migrations (version) VALUES (17)", []);
    }
}
```

Actually, a better approach: track which migration versions cover the columns `fix_schema_compat` may have added, and insert them:
- Migration 017 adds `recorded_at` to observations + edges
- Migration 018 adds `binary_hash` to embeddings

But `fix_schema_compat` only handles observations, sessions, and edges columns. So we only need to worry about migration 017.

**Even cleaner fix:** Move `fix_schema_compat` to AFTER the migration loop, or have it run only on columns NOT covered by migrations.

**Cleanest: make migration 017 idempotent** — use `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`. But SQLite doesn't support that syntax.

**Best approach:** Track the column additions in a simple set, then insert migration versions at the end:

```rust
// Track which migrations need to be marked as applied
// based on which columns fix_schema_compat actually added
let mut applied_migrations: Vec<i32> = Vec::new();

// ... existing PRAGMA checks ...

// For each batch of ALTER TABLE commands, if any were applied:
let obs_missing_count = missing_obs.iter().filter(|(col, _)| !obs_cols.iter().any(|c| c == col)).count();
let was_recorded_at_added = !obs_cols.iter().any(|c| c == "recorded_at");
let was_binary_hash_added = false; // embeddings table not checked here

// Apply ALTER TABLE commands
if !alter_cmds.is_empty() {
    for cmd in &alter_cmds {
        let _ = conn.execute_batch(cmd);
    }
}

// Mark migrations as applied if their columns were already handled
if was_recorded_at_added && !applied_migrations.contains(&17) {
    conn.execute("INSERT OR IGNORE INTO _migrations (version) VALUES (17)", []).ok();
}
```

**Verification:**
1. Create a Go-compat DB (without `recorded_at` column)
2. Run `the-crab-engram stats` — should succeed, not hang
3. Check `SELECT * FROM _migrations` — version 17 should be present
4. Run same command again — should be idempotent
5. Run `cargo test --workspace` — all existing tests pass

---

### Task 2: Add regression test for schema-compat + migration overlap

**Objective:** Write a test that reproduces the Go→Rust migration scenario: create a DB with Go-era schema (no recorded_at, pinned, etc.), open with SqliteStore, verify it doesn't error.

**Files:**
- Modify: `crates/store/src/migration.rs` (tests module)

**Test:**
```rust
#[test]
fn fix_schema_compat_does_not_clobber_migrations() {
    // Create a DB with Go-era schema (minimal columns)
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE observations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            project TEXT NOT NULL DEFAULT 'default'
        );
        CREATE TABLE edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL,
            target_id INTEGER NOT NULL
        );
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            project TEXT NOT NULL,
            started_at TEXT NOT NULL
        );"
    ).unwrap();
    
    // Run migrations — should succeed, not error
    let result = run_migrations(&conn);
    assert!(result.is_ok(), "migrations should succeed: {:?}", result.err());
    
    // Verify both schema column and _migrations entry exist
    let has_17: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM _migrations WHERE version = 17",
        [], |row| row.get(0)
    ).unwrap();
    assert!(has_17, "migration 17 should be marked as applied");
    
    // Running again should be idempotent
    assert!(run_migrations(&conn).is_ok());
}
```
