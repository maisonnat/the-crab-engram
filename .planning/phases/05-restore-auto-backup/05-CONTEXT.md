# Phase 5: Restore & Auto-Backup - Context

**Gathered:** 2026-04-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Add restore functionality (list, restore by ID, restore by file) and automatic backup triggers (before schema migration, before data import). Restore flow: verify integrity → create pre-restore backup → replace DB → reabrir conexión. Auto-backup triggers at migration and import call sites.

</domain>

<decisions>
## Implementation Decisions

### Restore by ID
- **D-01:** ID = sequential index in backup listing (1 = most recent). `restore --list` shows numbered list. `restore --id N` selects by position. Simple, no new ID system needed.

### Restore Flow
- **D-02:** Verify backup integrity first (reuse `backup_verify` from Phase 4) → create pre-restore backup (trigger = "pre-restore") → copy backup file over current DB → reabrir conexión. On Unix: atomic `rename()`. On Windows: delete old, copy new.

### Auto-Backup Triggers
- **D-03:** Two call sites:
  - **Schema migration:** `SqliteStore::new()` calls `migration::run_migrations()`. Auto-backup BEFORE migrations run (check if migrations pending, backup if yes).
  - **Data import:** `Commands::Import` handler in main.rs calls `store.import()`. Auto-backup BEFORE import runs.

### Confirmation UX
- **D-04:** Restore prompts for confirmation by default: "This will replace your current database. Continue? [y/N]". `--yes` flag skips prompt. This IS destructive — prompt is appropriate (unlike self-update which is additive).

### Rotation Activation
- **D-05:** Rotation activates when auto-backups are created. Keep last 10 auto-backups (trigger != "manual"). Manual backups never auto-deleted. Rotation runs after each auto-backup creation.

### Restore CLI
- **D-06:** `restore` as top-level command with subcommands:
  - `the-crab-engram restore --list` — list all backups
  - `the-crab-engram restore --id N` — restore by position
  - `the-crab-engram restore --file PATH` — restore from explicit file
  - `--yes` flag for skip confirmation

### Storage Trait Extensions
- **D-07:** Add to Storage trait:
  - `fn backup_list(&self) -> Result<Vec<BackupRecord>>` — list backups from disk
  - `fn backup_restore(&self, backup_path: &Path, confirm: bool) -> Result<()>` — restore from backup

</decisions>

<canonical_refs>
## Canonical References

### Source Code
- `crates/store/src/trait.rs` — Storage trait to extend with backup_list, backup_restore
- `crates/store/src/sqlite.rs` — SqliteStore, conn() method, migration::run_migrations call sites (lines 39, 61)
- `crates/store/src/migration.rs` — run_migrations function
- `src/main.rs` — CLI commands, Import handler (line 426)

### Phase 4 (prerequisite)
- `.planning/phases/04-backup-core/04-CONTEXT.md` — backup_create, backup_verify decisions
- `crates/store/src/sqlite.rs` — backup_dir(), schema_version(), sha256_file() helpers

### Requirements
- `.planning/REQUIREMENTS.md` §Backup & Restore — BACKUP-03 through BACKUP-11

### Research
- `.planning/research/ARCHITECTURE.md` §Backup/Restore Flow — restore flow diagram

</canonical_refs>

<code_context>
## Existing Code Insights

### Migration Call Sites (sqlite.rs)
```rust
// Line 39 — SqliteStore::new() (file-backed)
pub fn new(path: &Path) -> Result<Self> {
    let conn = rusqlite::Connection::open(path)...;
    migration::run_migrations(&conn)?;
    ...
}

// Line 61 — SqliteStore::memory() (in-memory, for testing)
pub fn memory() -> Result<Self> {
    let conn = rusqlite::Connection::open_in_memory()?;
    migration::run_migrations(&conn)?;
    ...
}
```

### Import Call Site (main.rs)
```rust
Commands::Import { file } => {
    let store = open_store(cli.db)?;
    let data: ExportData = serde_json::from_str(&std::fs::read_to_string(file)?)?;
    let result = store.import(&data)?;
    // auto-backup should happen BEFORE this line
}
```

### Backup Infrastructure (Phase 4)
- `backup_dir()` — resolves `~/.engram/backups/`
- `schema_version()` — queries `_migrations` table
- `sha256_file()` — computes file checksum
- `backup_create(trigger, label)` — creates .db + .meta.json
- `backup_verify(path)` — SHA-256 + PRAGMA integrity_check

</code_context>

<specifics>
## Specific Ideas

### Restore CLI (main.rs)
```rust
/// Restore from backup
Restore {
    /// List all backups
    #[arg(long)]
    list: bool,
    /// Restore by backup ID (position in list)
    #[arg(long)]
    id: Option<usize>,
    /// Restore from explicit backup file
    #[arg(long)]
    file: Option<PathBuf>,
    /// Skip confirmation prompt
    #[arg(long)]
    yes: bool,
},
```

### Restore List Output
```
$ the-crab-engram restore --list
#  | Created              | Trigger  | Label           | Size    | SHA-256
1  | 2026-04-12T00:50:00Z | manual   | before upgrade  | 1.2 MB  | abc123...
2  | 2026-04-11T20:00:00Z | manual   |                 | 1.1 MB  | def456...
3  | 2026-04-11T15:00:00Z | auto     |                 | 1.0 MB  | ghi789...
```

### Rotation Implementation
```rust
fn rotate_old_backups(&self) -> Result<()> {
    let dir = self.backup_dir()?;
    let mut auto_backups: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "db"))
        .filter(|e| {
            // Read .meta.json to check trigger
            let meta_path = e.path().with_extension("meta.json");
            std::fs::read_to_string(&meta_path)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .map_or(false, |m| m["trigger"] != "manual")
        })
        .collect();
    
    auto_backups.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified()).ok());
    
    // Keep last 10, delete older
    while auto_backups.len() > 10 {
        let old = auto_backups.remove(0);
        let _ = std::fs::remove_file(old.path());
        let _ = std::fs::remove_file(old.path().with_extension("meta.json"));
    }
    Ok(())
}
```

### Auto-Backup in Migration (sqlite.rs)
```rust
pub fn new(path: &Path) -> Result<Self> {
    let conn = rusqlite::Connection::open(path)...;
    
    // Check if migrations are pending
    let pending = migration::pending_migrations(&conn)?;
    if !pending.is_empty() {
        // Auto-backup before migrations
        // ... (needs Self instance, but conn not fully set up yet)
    }
    
    migration::run_migrations(&conn)?;
    ...
}
```

</specifics>

<deferred>
## Deferred Ideas

- Pre-restore backup as separate backup entry with trigger "pre-restore" — already decided in D-02
- Backup compression — not needed, SQLite files are small
- Incremental backups — over-engineering for current scale
- Backup encryption — Phase 9 territory

</deferred>

---

*Phase: 05-restore-auto-backup*
*Context gathered: 2026-04-12*
