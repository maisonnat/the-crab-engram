# Phase 04: Backup Core — Summary

**Completed:** 2026-04-12
**Plan:** 04-01
**Requirements:** BACKUP-01, BACKUP-02, BACKUP-06, BACKUP-12, BACKUP-13, BACKUP-14, BACKUP-15
**Commit:** 1daf860

## What Was Built

- **Storage trait** — added `BackupStats`, `BackupRecord`, `BackupVerifyResult` structs + `backup_create()` and `backup_verify()` methods
- **SqliteStore** — implemented backup using `rusqlite::backup::Backup::run_to_completion()`, SHA-256 checksums, .meta.json sidecar
- **CLI** — `the-crab-engram backup [--label]` and `the-crab-engram verify-backup FILE` as top-level commands

## Decisions Applied

| ID | Decision | Status |
|----|----------|--------|
| D-01 | Backups at `~/.engram/backups/` | ✅ |
| D-02 | `engram-{ISO}.db` naming | ✅ |
| D-03 | Methods on SqliteStore directly | ✅ |
| D-04 | Top-level CLI commands | ✅ |
| D-05 | .meta.json sidecar with full metadata | ✅ |
| D-06 | Manual backups never auto-deleted | ✅ |
| D-07 | Methods on Storage trait | ✅ |

## Files Changed

- `Cargo.toml` — added `backup` feature to rusqlite
- `crates/store/Cargo.toml` — added `dirs` dependency
- `crates/store/src/trait.rs` — Backup types + trait methods
- `crates/store/src/sqlite.rs` — backup_create, backup_verify, helpers
- `crates/store/src/lib.rs` — re-exports
- `src/main.rs` — CLI commands + handlers
