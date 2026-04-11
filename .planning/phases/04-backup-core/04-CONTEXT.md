# Phase 4: Backup Core - Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Add manual backup creation and verification for the SQLite database. User can run `the-crab-engram backup` to create timestamped backups with metadata sidecars, and `the-crab-engram verify-backup FILE` to check integrity. Backup methods live on `Storage` trait — no new crate. Uses `rusqlite::backup::Backup::run_to_completion()` which works while MCP server is active (non-blocking).

</domain>

<decisions>
## Implementation Decisions

### Backup Directory
- **D-01:** Backups live at `~/.engram/backups/` alongside the database. Not configurable for now. Directory created automatically if it doesn't exist.

### File Naming Convention
- **D-02:** `engram-{ISO timestamp}.db` for the backup file, `engram-{ISO timestamp}.meta.json` for the sidecar. Example: `engram-2026-04-11T20-00-00Z.db`. Timestamps use UTC, ISO 8601 format with `:` replaced by `-` for filesystem safety.

### Connection Access
- **D-03:** Backup methods implemented directly on `SqliteStore`. Access `self.conn()` (the MutexGuard) for `rusqlite::backup::Backup`. No new public accessor — backup is an implementation detail of the store.

### CLI Structure
- **D-04:** `backup` and `verify-backup` as TOP-LEVEL commands (like `version`, `export`). Not nested under `self` — backup is about data, not binary management.
  - `the-crab-engram backup [--label "description"]`
  - `the-crab-engram verify-backup <file>`

### Stats in .meta.json
- **D-05:** Sidecar contains:
  - `version`: crate version from `CARGO_PKG_VERSION`
  - `schema_version`: from migration system
  - `created_at`: ISO 8601 timestamp
  - `trigger`: "manual" or "auto" (Phase 5 adds auto triggers)
  - `label`: optional user label
  - `size_bytes`: backup file size
  - `sha256`: checksum of the .db file
  - `stats`: `{ observations, sessions, edges }` counts

### Rotation
- **D-06:** Manual backups are NEVER auto-deleted. Only auto-backups rotate (keep last 10). Since Phase 4 only implements manual backup, rotation is a no-op for now — implemented in Phase 5.

### Backup API
- **D-07:** Methods on `Storage` trait:
  - `fn backup_create(&self, trigger: &str, label: Option<&str>) -> Result<BackupRecord>`
  - `fn backup_verify(&self, path: &Path) -> Result<BackupVerifyResult>`
  - `BackupRecord` struct with: id, path, created_at, trigger, label, size_bytes, sha256, stats
  - `BackupVerifyResult` struct with: valid, sha256_match, integrity_check_pass, error

</decisions>

<canonical_refs>
## Canonical References

### Source Code
- `crates/store/src/trait.rs` — Storage trait to extend with backup methods
- `crates/store/src/sqlite.rs` — SqliteStore with `conn: Mutex<rusqlite::Connection>`, implement backup here
- `crates/store/src/lib.rs` — module exports
- `src/main.rs` — CLI commands to add

### Research
- `.planning/research/ARCHITECTURE.md` §Backup/Restore Flow — exact flow diagram
- `.planning/research/PITFALLS.md` §Pitfall 4 — SQLite backup mutex deadlock warning

### Requirements
- `.planning/REQUIREMENTS.md` §Backup & Restore — BACKUP-01, BACKUP-02, BACKUP-06, BACKUP-12, BACKUP-13, BACKUP-14, BACKUP-15

### Project Constraints
- `.planning/PROJECT.md` §Constraints — stdout sacred, zero system deps
- Database safety decision (engram observation #10) — self-update must NEVER touch DB, but backup CAN and SHOULD

</canonical_refs>

<code_context>
## Existing Code Insights

### Storage Trait (trait.rs)
```rust
pub trait Storage: Send + Sync {
    fn add_observation(&self, params: &AddObservationParams) -> Result<i64>;
    fn search_observations(&self, query: &str, options: &SearchOptions) -> Result<Vec<Observation>>;
    fn update_observation(&self, id: i64, params: &UpdateObservationParams) -> Result<()>;
    fn delete_observation(&self, id: i64) -> Result<()>;
    // ... 20+ more methods
    fn import(&self, data: &ExportData) -> Result<ImportResult>;
}
```

### SqliteStore (sqlite.rs)
```rust
pub struct SqliteStore {
    conn: Mutex<rusqlite::Connection>,
}

impl SqliteStore {
    fn conn(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.conn.lock().expect("sqlite connection mutex poisoned")
    }
}
```

### rusqlite::backup API
```rust
use rusqlite::backup::Backup;
let mut dst = rusqlite::Connection::open(&backup_path)?;
let backup = Backup::new(&src_conn, &mut dst)?;
backup.run_to_completion(500, Duration::from_millis(0), None)?;
// 500 = pages per step, non-blocking when MCP is active
```

### CLI Commands (main.rs)
- Top-level `Commands` enum with clap derive
- Each variant maps to handler in `match` block
- Current top-level: Mcp, Search, Save, Context, Stats, Timeline, Export, Import, ExportContext, SessionStart, SessionEnd, Version, Serve, Tui, Consolidate, Sync, Encrypt, Setup, Self_

</code_context>

<specifics>
## Specific Ideas

### Backup CLI Commands
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing ...
    
    /// Create a backup of the knowledge store
    Backup {
        /// Optional label for the backup
        #[arg(long)]
        label: Option<String>,
    },
    
    /// Verify backup integrity
    VerifyBackup {
        /// Path to backup file
        file: PathBuf,
    },
}
```

### Backup Handler
```rust
Commands::Backup { label } => {
    let store = open_store(cli.db)?;
    let record = store.backup_create("manual", label.as_deref())?;
    eprintln!("Backup created: {}", record.path.display());
    eprintln!("SHA-256: {}", record.sha256);
    eprintln!("Size: {} bytes", record.size_bytes);
    eprintln!("Observations: {}", record.stats.observations);
}
```

### Backup Directory Resolution
```rust
fn backup_dir(db_path: &Path) -> PathBuf {
    // db_path = ~/.engram/engram.db → ~/.engram/backups/
    db_path.parent().unwrap_or(Path::new(".")).join("backups")
}
```

### Meta JSON Schema
```json
{
    "version": "2.0.0",
    "schema_version": 3,
    "created_at": "2026-04-11T20:00:00Z",
    "trigger": "manual",
    "label": "before experiment",
    "size_bytes": 1048576,
    "sha256": "abc123...",
    "stats": {
        "observations": 1234,
        "sessions": 56,
        "edges": 789
    }
}
```

</specifics>

<deferred>
## Deferred Ideas

- Automatic backup before schema migration (BACKUP-07) — Phase 5
- Automatic backup before data import (BACKUP-08) — Phase 5
- Restore functionality (BACKUP-03, 04, 05) — Phase 5
- Pre-restore backup (BACKUP-09) — Phase 5
- Restore confirmation (BACKUP-11) — Phase 5
- Configurable backup directory — not needed yet
- Backup compression — adds complexity, SQLite files compress well but backup speed matters more

</deferred>

---

*Phase: 04-backup-core*
*Context gathered: 2026-04-11*
