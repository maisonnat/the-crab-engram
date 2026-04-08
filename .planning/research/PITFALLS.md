# Domain Pitfalls: Rust CLI Self-Update, SQLite Backup & Cross-Platform Distribution

**Domain:** Rust CLI with self-update, SQLite persistence, multi-platform packaging
**Researched:** 2026-04-08
**Confidence:** HIGH (verified against self_update issues #81, #136, #151; rusqlite docs; starship release workflow; existing codebase)

---

## Critical Pitfalls

### Pitfall 1: Asset Naming Mismatch — self_update Can't Find Your Binary

**What goes wrong:**
`self_update` can't find the release asset because the archive names don't match what it expects. The user runs `the-crab-engram update` and gets "no asset found for target" or downloads the wrong variant (gnu instead of musl on Alpine).

**Why it happens:**
The existing release workflow uses *custom* archive names (`the-crab-engram-linux-x86_64.tar.gz`) instead of target-triple names (`the-crab-engram-x86_64-unknown-linux-gnu.tar.gz`). `self_update`'s `asset_for()` method does a best-match against target triples in asset names. When the naming convention doesn't include the full target triple, it falls back to OS/arch substring matching — which has a known bug (issue #136) where it matches the *first* asset containing "linux" and "x86_64", ignoring the glibc/musl distinction.

**Consequences:**
- Alpine/musl users get a glibc binary → crashes on startup
- Users on any Linux get the wrong variant silently
- The update command appears to succeed but the binary doesn't work

**How to avoid:**
Switch the release workflow to target-triple archive naming. This is the industry standard (starship, ripgrep, fd, bat all do it). The archive name *must* contain the full target triple: `the-crab-engram-{version}-{target}.{ext}`.

```yaml
# WRONG (current)
archive: the-crab-engram-linux-x86_64.tar.gz

# RIGHT (target triple)
archive: the-crab-engram-${{ matrix.target }}.tar.gz
```

**Warning signs:**
- Testing `update` on Alpine/musl downloads but binary segfaults
- `self_update` logs show "matched by OS/ARCH" instead of exact target triple match
- `the-crab-engram --version` shows gnu target on a musl system

**Phase to address:** Phase 2 (Expanded Build Matrix)

---

### Pitfall 2: Windows 0-Byte Executable After Update

**What goes wrong:**
After self-update on Windows, the binary is 0 bytes. The tool is completely bricked. User must manually download from GitHub to recover.

**Why it happens:**
Windows cannot overwrite a running executable. `self_update` handles this via a rename→`.old`→copy→cleanup pattern. But the temp file extraction can fail silently if:
1. Antivirus/Windows Defender scans and locks the temp file during extraction
2. The extraction directory has restrictive permissions
3. Disk runs out of space during extraction (temp file creation succeeds but copy fails)
4. The `fs::rename` fails on Windows when source and dest are on different drives

The crate has an open issue (#81) documenting this exact failure mode — download succeeds (13.2MB) but the result is a 0-byte executable.

**Consequences:**
- Tool is completely non-functional after update
- No automatic rollback mechanism
- User loses access until manual re-download

**How to avoid:**
1. After `self_update` completes, verify the new binary is non-zero size AND executable before reporting success
2. Keep the old binary as `the-crab-engram.backup` until verification passes
3. If verification fails, restore from `.backup` automatically
4. Add `--force` flag for re-download if binary is corrupt
5. Test the update flow on Windows CI with `windows-latest` and `windows-11-arm`

```rust
// Post-update verification (critical for Windows)
let new_binary = std::fs::metadata(&binary_path)?;
if new_binary.len() == 0 {
    // Restore from backup
    std::fs::rename(&backup_path, &binary_path)?;
    return Err("Update produced 0-byte binary, rolled back".into());
}
```

**Warning signs:**
- Update reports success but `the-crab-engram --version` fails
- Binary file size is 0 bytes after update
- Windows Event Log shows antivirus quarantining the temp file

**Phase to address:** Phase 1 (Self-Update Engine)

---

### Pitfall 3: SQLite Backup Mutex Deadlock — Backup Blocks MCP Server

**What goes wrong:**
When a backup is triggered while the MCP server is active, either the backup hangs forever or the MCP server stops responding. The system deadlocks.

**Why it happens:**
`SqliteStore` wraps the connection in a single `Mutex<rusqlite::Connection>`. The `rusqlite::backup::Backup::new()` needs a reference to the source connection. If the backup operation holds the mutex lock for the duration of `run_to_completion()`, ALL other database operations (including MCP requests) block waiting for the mutex.

Looking at the existing code, `self.conn()` returns a `MutexGuard` — every operation holds this lock. The backup implementation must:
1. NOT hold the mutex across the entire backup duration
2. Open a *second* connection to the same database file for the backup destination
3. Use the source connection reference briefly for `Backup::new()`, then step through pages

**Consequences:**
- MCP server becomes unresponsive during backup (seconds, not milliseconds)
- If backup is slow (large DB), users think the tool crashed
- CI tests that run MCP + backup concurrently will deadlock

**How to avoid:**
The backup implementation must be designed around the single-connection mutex constraint:

```rust
fn backup_create(&self, trigger: &str, label: Option<&str>) -> Result<BackupRecord> {
    let backup_path = self.generate_backup_path(trigger);

    // Open destination connection (no mutex contention)
    let mut dst_conn = Connection::open(&backup_path)?;

    // Hold source lock ONLY for Backup::new + step, not the full duration
    // Use small page batches with yield points
    {
        let src_conn = self.conn(); // Lock acquired
        let backup = Backup::new(&src_conn, &mut dst_conn)?;
        // Use small page count + sleep between steps to yield the lock
        backup.run_to_completion(5, Duration::from_millis(10), None)?;
        // Lock released when src_conn drops
    }

    // Write metadata sidecar AFTER backup (no lock needed)
    self.write_backup_metadata(&backup_path, trigger, label)?;
    Ok(record)
}
```

Alternative: Use `Backup::step()` in a loop, dropping the lock between steps — more complex but truly non-blocking.

**Warning signs:**
- MCP server latency spikes during backup
- Integration tests that do MCP requests + backup concurrently hang
- `cargo test` hangs on backup tests when run with MCP tests

**Phase to address:** Phase 4 (Backup & Restore System)

---

### Pitfall 4: WAL Checkpoint Interference with Backup Consistency

**What goes wrong:**
A backup completes successfully but when restored, it's missing recent data. Or: a backup captures the `.db` file but not the WAL file state, resulting in a corrupt restore.

**Why it happens:**
The existing store enables WAL mode (`PRAGMA journal_mode = WAL`). WAL means recent writes go to `engram.db-wal`, not the main `.db` file. The `rusqlite::backup::Backup` API handles this correctly internally (it's the official SQLite online backup API). However, two edge cases can bite you:

1. **WAL file not included in backup if using file-copy approach**: If anyone on the team suggests "just copy the .db and .db-wal files" as a simpler alternative, they're wrong — there's a race condition between copying the two files where committed transactions in WAL haven't been checkpointed.

2. **`PRAGMA wal_checkpoint(TRUNCATE)` before backup**: If you call checkpoint before backup to "ensure consistency," you can cause data loss if the checkpoint races with an active write.

3. **Backup destination is on a different filesystem**: On some systems (NFS, Docker volumes), `rusqlite::backup` to a different filesystem can fail silently.

**Consequences:**
- Silent data loss — backup appears valid but is missing transactions
- Restored database has gap between last checkpoint and backup time

**How to avoid:**
- Use ONLY `rusqlite::backup::Backup::run_to_completion()` — never file-copy
- Do NOT call `wal_checkpoint` before or during backup — let the backup API handle it
- After backup, call `PRAGMA integrity_check` on the destination to verify
- Store the SHA-256 checksum in the metadata sidecar and verify before restore

**Warning signs:**
- Backup `.db` file is significantly smaller than expected
- `PRAGMA integrity_check` on restored backup returns errors
- Observation count in backup metadata doesn't match restored database

**Phase to address:** Phase 4 (Backup & Restore System)

---

### Pitfall 5: stdout Contamination Crashes MCP Protocol

**What goes wrong:**
The MCP server stops responding to the AI agent. JSON-RPC messages become garbled. The AI agent can no longer communicate with the tool.

**Why it happens:**
MCP uses stdio (stdin/stdout) for JSON-RPC communication. ANY output to stdout that isn't a valid JSON-RPC message corrupts the protocol. The existing codebase correctly sends MCP messages over stdout. But adding these features creates new stdout contamination vectors:

1. **Background update check** (Phase 9): If the update check prints "A new version is available!" to stdout instead of stderr, the MCP client receives garbage.
2. **`the-crab-engram update` command**: If called by the agent (not the user), output goes to stdout and corrupts the active MCP session.
3. **`tracing::info!` during MCP serve**: Tracing subscriber defaults to stdout. If any `info!()` fires during MCP request handling, the JSON-RPC stream is corrupted.
4. **SQLite warnings/errors**: `rusqlite` can emit warnings to stdout during backup operations.

**Consequences:**
- MCP server becomes non-functional until restarted
- AI agent loses all memory/context access
- May require manual intervention to diagnose (error looks like protocol corruption, not a logging issue)

**How to enforce:**
1. ALL user-facing output in `Commands::Mcp` and `Commands::Serve` MUST go to `eprintln!` or `tracing` with stderr subscriber
2. The background update check (Phase 9) MUST use `eprintln!` only — this is called out in PROJECT.md as a constraint
3. Add a CI test that runs `the-crab-engram mcp` for 5 seconds and verifies stdout contains ONLY valid JSON-RPC
4. Consider `setvbuf(stdout, NULL, _IONBF, 0)` to disable stdout buffering during MCP mode

```rust
// In MCP/serve mode, redirect tracing to stderr
tracing_subscriber::fmt()
    .with_writer(std::io::stderr) // CRITICAL: not stdout
    .with_env_filter(...)
    .init();
```

**Warning signs:**
- MCP client shows "invalid JSON" errors
- Agent can't connect to the tool after adding update check
- Debugging reveals human-readable text mixed with JSON-RPC in stdout

**Phase to address:** Phase 9 (Background Update Check) — but must be considered in every phase

---

### Pitfall 6: musl + rusqlite/bundled Compilation Failures in CI

**What goes wrong:**
CI build for `x86_64-unknown-linux-musl` or `aarch64-unknown-linux-musl` fails with linker errors like `cannot find -lc` or C compiler errors during SQLite compilation.

**Why it happens:**
`rusqlite/bundled` compiles the C SQLite source via `cc` crate. For musl targets, this requires:
1. `musl-tools` package installed (provides `musl-gcc`)
2. The `musl-gcc` wrapper must be on PATH
3. For ARM musl cross-compilation, you need `musl-tools` + `gcc-aarch64-linux-gnu`

GitHub's `ubuntu-latest` does not include `musl-tools` by default. The `cc` crate will try to use the system `gcc`, which links against glibc — producing a binary that's not truly static.

**Consequences:**
- CI fails for musl targets (build step errors)
- If it "works" without musl-tools, the binary isn't actually static (has glibc deps)
- Alpine users get "not found" error because the dynamic linker path is wrong

**How to avoid:**
```yaml
- name: Install musl tools
  if: contains(matrix.target, 'musl')
  run: |
    sudo apt-get update
    sudo apt-get install -y musl-tools
    # For ARM musl cross-compilation
    if [ "${{ matrix.target }}" = "aarch64-unknown-linux-musl" ]; then
      sudo apt-get install -y gcc-aarch64-linux-gnu
    fi

- name: Build release
  run: cargo build --release --target ${{ matrix.target }}
  env:
    CC: ${{ contains(matrix.target, 'musl') && 'musl-gcc' || '' }}
```

**Warning signs:**
- `ldd target/.../the-crab-engram` shows `libc.so` dependency on musl build
- Build succeeds but binary doesn't run on Alpine
- `file target/.../the-crab-engram` shows "dynamically linked" for musl target

**Phase to address:** Phase 2 (Expanded Build Matrix)

---

### Pitfall 7: Pre-Migration Backup Timing — Race with Schema Changes

**What goes wrong:**
The pre-migration backup captures the database *after* some migrations have already been applied, or the backup itself triggers migrations, creating a recursive loop.

**Why it happens:**
Looking at the existing code: `SqliteStore::new()` calls `migration::run_migrations(&conn)` immediately after opening the connection. If we add pre-migration backup logic, we need to be careful about the ordering:

```
Current flow:
  open connection → run_migrations → done

If backup is added WRONG:
  open connection → create backup → BUT backup opens second connection →
  second connection triggers SqliteStore::new() → run_migrations → ???
```

The `run_migrations` function takes a `&rusqlite::Connection` reference, not `&SqliteStore`. But the backup needs to happen BEFORE migrations run, which means we need to:
1. Open the connection
2. Check if migrations are pending
3. If yes, create backup
4. Then run migrations

**Consequences:**
- Backup of partially-migrated database is unrecoverable
- Recursive migration loop if backup opens SqliteStore
- Schema version in backup metadata doesn't match actual schema

**How to avoid:**
The backup should NOT go through `SqliteStore::new()` — use a raw `rusqlite::Connection` for the backup destination. The pre-migration check should be in `SqliteStore::new()` *before* `migration::run_migrations`:

```rust
pub fn new(path: &Path) -> crate::Result<Self> {
    let conn = rusqlite::Connection::open(path)...;
    conn.execute_batch("PRAGMA journal_mode = WAL; ...")?;

    // Check if migrations are pending BEFORE running them
    let pending = migration::pending_count(&conn)?;
    if pending > 0 {
        // Create backup with raw connection (no SqliteStore, no recursion)
        Self::backup_before_migration(path)?;
    }

    migration::run_migrations(&conn)?;
    ...
}
```

**Warning signs:**
- Backup created at every startup (migrations "pending" even after running)
- Stack overflow from recursive `SqliteStore::new()` calls
- Backup metadata shows `schema_version: 0` or wrong version

**Phase to address:** Phase 4 (Backup & Restore System)

---

### Pitfall 8: GitHub Actions Matrix Explosion — CI Time and Cost

**What goes wrong:**
Release workflow takes 45+ minutes. One flaky target blocks the release. CI minutes burn through free tier.

**Why it happens:**
Going from 3 targets to 8 targets means 8× the build time. Each Rust release build takes 5-10 minutes. Plus:
- musl builds are slower (static linking)
- ARM builds on `ubuntu-24.04-arm` are new and less reliable
- Windows builds are inherently slower (filesystem, linker)
- `cargo-wix` MSI builds add 2-3 minutes per Windows target

With `fail-fast: false`, all 8 jobs run to completion even if one fails. Total CI time: 8 × 8 min = ~64 minutes.

**Consequences:**
- Release takes over an hour to complete
- If one target fails, you have 7 working artifacts + 1 missing (partial release)
- Flaky ARM runner causes false failures, requiring re-runs

**How to avoid:**
1. Use `cargo` caching aggressively (already done, but ensure cache key includes target)
2. Consider skipping musl builds on non-release tags (only build on version tags)
3. Set per-job timeout (15 min) to fail fast on hung builds
4. Use `concurrency` groups to cancel in-progress builds on new push
5. Test matrix expansion incrementally: add musl first, then ARM, then MSI

**Warning signs:**
- Release workflow takes > 30 minutes
- Frequent "runner not available" errors for ARM targets
- Cache hit rate drops below 50%

**Phase to address:** Phase 2 (Expanded Build Matrix)

---

### Pitfall 9: RPM/DEB Package Conflicts with Manual Install

**What goes wrong:**
User installed via `curl | sh` then tries `dpkg -i the-crab-engram.deb`. The deb package installs to `/usr/bin/` but the manual install is in `~/.local/bin/`. Now there are two binaries. Which one runs depends on PATH order. Self-update updates the wrong one.

**Why it happens:**
The install script (Phase 8) typically installs to `~/.local/bin/` or `/usr/local/bin/`. The `.deb` package installs to `/usr/bin/`. Both are in PATH. The user has two versions. When they run `the-crab-engram update`, it updates whichever one is first in PATH — which might not be the one they think.

**Consequences:**
- User thinks they updated but the other binary is still old
- Database migrations run on one binary but not the other
- Confusing "already up to date" when the wrong binary is checked

**How to avoid:**
1. The deb/rpm packages should conflict with each other and provide the binary
2. The install script should detect existing package-manager installations and refuse to install alongside them
3. `the-crab-engram update` should print the path of the binary it's updating
4. Document clearly: "choose ONE installation method"

**Warning signs:**
- `which -a the-crab-engram` shows multiple paths
- `the-crab-engram --version` shows different version than `dpkg -l the-crab-engram`
- Update succeeds but PATH picks up old binary

**Phase to address:** Phase 5 (Deb/RPM) + Phase 8 (Install Scripts)

---

### Pitfall 10: Backup Rotation Deletes User's Manual Backups

**What goes wrong:**
User creates a manual backup before a risky import. Later, the automatic rotation (keep last 10) deletes their manual backup. When they need to restore, it's gone.

**Why it happens:**
The ADR-013 specifies "10 auto-backup rotation limit" and "manual backups never auto-deleted." But if the implementation doesn't clearly distinguish between manual and auto backups in the filename/metadata, rotation will blindly delete the oldest files regardless of origin.

**Consequences:**
- User's important manual backup silently disappears
- User thinks they have a safety net but it was rotated away
- Trust in the backup system is broken

**How to avoid:**
1. Manual backups use a different filename prefix or directory: `engram-manual-{timestamp}-{label}.db`
2. Rotation only touches files matching `engram-auto-*` pattern
3. The `.meta.json` sidecar includes `"trigger": "manual"` vs `"trigger": "pre-update"`
4. `backup --list` clearly marks which backups are auto (rotated) vs manual (permanent)

**Warning signs:**
- After 10+ auto-backups, manual backups disappear
- `backup --list` doesn't distinguish manual from auto
- Rotation test doesn't check for manual backup preservation

**Phase to address:** Phase 4 (Backup & Restore System)

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| File-copy backup instead of `Backup` API | Simpler code, no rusqlite feature flag | Race condition with WAL, silent data loss | NEVER — data safety is core value |
| Custom archive names instead of target triples | Matches current workflow | Breaks self_update, confuses users | NEVER — ADR-006 decision already made |
| Print update info to stdout | Simpler than stderr routing | Crashes MCP protocol | NEVER — stdout is sacred |
| Skip pre-migration backup | Faster startup | No recovery from bad migration | Only in dev, NEVER in release |
| Use `cross` for musl instead of native | Works today | Slow (QEMU), harder to debug | Temporarily until native ARM runners tested |
| Single backup directory, no rotation | Simpler file management | Unbounded disk growth | Only for first implementation, add rotation before release |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `self_update` + GitHub Releases | Default asset matching breaks with gnu+musl | Use target-triple naming, test on Alpine |
| `rusqlite::backup` + WAL mode | Calling `wal_checkpoint` before backup | Let Backup API handle WAL internally |
| `cargo-wix` + CI | Not installing WiX toolset | `cargo install cargo-wix` in workflow step |
| `cargo-deb` + musl | Building deb from musl binary | Build deb from gnu binary only |
| Homebrew tap + auto-update | Manual formula updates on every release | Use `repository_dispatch` from release workflow |
| Background update + MCP | Printing to stdout | All output to stderr, test with MCP client |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Backup holds mutex for full duration | MCP server latency spikes during backup | Use small page batches, yield between steps | DB > 10MB |
| GitHub API rate limit on update check | "403 Forbidden" on update check | Cache last check time, respect 24h interval | > 60 checks/hour (CI/test loops) |
| FTS5 index not rebuilt after restore | Search returns wrong results after restore | Run `INSERT INTO observations_fts(observations_fts) VALUES('rebuild')` after restore | Any restore |
| Checksum computation on large backup | Backup takes seconds instead of ms | Compute SHA-256 asynchronously after backup completes | DB > 100MB |
| Release workflow artifact size | Upload/download artifact step times out | Use `.tar.gz` compression, not raw binaries | Binaries > 50MB |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Install script without checksum verification | MITM injects malicious binary | Verify SHA-256 from `checksums-sha256.txt` in install script |
| Backup stored with world-readable permissions | Other users read agent's memory | `chmod 600` backup files, `chmod 700` backup directory |
| Self-update over HTTP | Binary tampered in transit | `self_update` uses HTTPS by default, but verify `rustls` feature is active |
| Backup metadata includes sensitive data | Observation content leaked via sidecar | Sidecar only has stats + checksum, NOT observation content |
| GitHub token in release workflow exposed | Attacker pushes malicious release | Use `GITHUB_TOKEN` (auto-scoped), never PAT with broad permissions |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Update check prints on every command | Annoying, users disable it entirely | Only in `mcp`/`serve`, max 1x per 24h, stderr only |
| Backup requires `--yes` to suppress confirmation | Scripts break on interactive prompt | `--yes` flag, detect non-interactive terminal (piped input) |
| `restore --list` shows raw timestamps | Hard to parse "2026-04-08T14:30:00Z" | Show relative time: "2 hours ago", "3 days ago" |
| Update fails with cryptic self_update error | "Asset not found" doesn't explain why | Catch and re-wrap with user-friendly message + link to releases page |
| No progress during large backup | User thinks it hung | Use `Backup::progress` callback to show percentage |

## "Looks Done But Isn't" Checklist

- [ ] **Self-update on Windows:** Test actual binary replacement, not just download. Verify non-zero size. Test with antivirus enabled.
- [ ] **musl build is truly static:** Run `ldd` on the output. If it shows `libc.so`, it's not static.
- [ ] **Backup during active MCP:** Test MCP request + backup concurrently. Verify no deadlock.
- [ ] **WAL consistency after backup:** Write data, backup immediately, restore, verify data present.
- [ ] **Asset naming matches self_update:** Run `the-crab-engram update --check-only` against actual release. Verify correct asset found.
- [ ] **stdout is clean during MCP:** Run `the-crab-engram mcp 2>/dev/null | head -1` — output must be valid JSON-RPC, not log text.
- [ ] **Manual backup survives rotation:** Create manual backup, create 15 auto-backups, verify manual still exists.
- [ ] **DEB package doesn't conflict:** Install via curl, then install deb — script should warn or deb should conflict.
- [ ] **Restore creates pre-restore backup:** Restore from backup #3, verify backup #4 is the pre-restore backup of the previous state.
- [ ] **ARM builds actually run:** Don't just verify build succeeds — run `the-crab-engram --version` on the ARM binary (use QEMU in CI or actual ARM runner).

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 0-byte binary after Windows update | HIGH (manual download) | Keep `.old` file, rename back. Implement in Phase 1. |
| MCP protocol corruption from stdout | MEDIUM (restart MCP) | Kill MCP process, restart. Fix tracing subscriber. |
| Wrong musl/gnu binary downloaded | MEDIUM (manual install) | Download correct variant from GitHub Releases. |
| Backup of corrupt database | HIGH (data loss) | Always run `integrity_check` after backup. Multiple backup generations. |
| Deadlocked backup + MCP | LOW (restart) | Kill process. Redesign backup to not hold mutex. |
| Flaky ARM CI build | LOW (re-run) | Re-run failed job. Consider timeout increase. |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|-------------|
| Asset naming mismatch | Phase 2 (Build Matrix) | `update --check-only` finds correct asset on all platforms |
| Windows 0-byte executable | Phase 1 (Self-Update) | Update on Windows, verify binary size + runs |
| Backup mutex deadlock | Phase 4 (Backup/Restore) | Concurrent MCP + backup test passes |
| WAL checkpoint interference | Phase 4 (Backup/Restore) | `integrity_check` passes on every backup |
| stdout contamination | Phase 9 (Background Check) | MCP stdout purity test passes |
| musl compilation failures | Phase 2 (Build Matrix) | `ldd` shows "statically linked" for musl builds |
| Pre-migration backup race | Phase 4 (Backup/Restore) | Backup created before first migration, not after |
| CI matrix explosion | Phase 2 (Build Matrix) | Release completes in < 20 min |
| RPM/DEB conflicts | Phase 5 (Deb/RPM) + Phase 8 (Scripts) | Install script detects package-manager install |
| Backup rotation deletes manual | Phase 4 (Backup/Restore) | Manual backup persists after 15 auto-backups |

## Sources

- [self_update issue #81](https://github.com/jaemk/self_update/issues/81) — 0-byte executable on Windows (10 reactions, open since 2022)
- [self_update issue #136](https://github.com/jaemk/self_update/issues/136) — asset_for matches wrong target with gnu+musl (bug, Aug 2024)
- [self_update issue #151](https://github.com/jaemk/self_update/issues/151) — per-target version failures (Apr 2025)
- [rusqlite::backup docs](https://docs.rs/rusqlite/latest/rusqlite/backup/index.html) — online backup API constraints
- [starship release.yml](https://github.com/starship/starship/blob/master/.github/workflows/release.yml) — industry benchmark for 11-target matrix + MSI + winget
- [PROJECT.md](.planning/PROJECT.md) — project constraints and decisions
- [Master Plan](the-crab-engram-master-plan.md) — ADR-001 through ADR-014

---

*Pitfalls research for: Rust CLI self-update, SQLite backup, cross-platform distribution*
*Researched: 2026-04-08*
*Confidence: HIGH — verified against actual issues, docs, and codebase*
