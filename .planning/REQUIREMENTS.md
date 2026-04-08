# Requirements — The Crab Engram v2.0.0

**Scope:** Self-update, SQLite backup/restore, cross-platform distribution (8 targets, 12 artifacts)
**Generated:** 2026-04-08 from master plan + research

---

## v1 Requirements

### Self-Update

- [ ] **UPDATE-01**: User can update to latest release with `the-crab-engram update`
- [ ] **UPDATE-02**: User can check for updates without downloading with `the-crab-engram update --check-only`
- [ ] **UPDATE-03**: System verifies SHA-256 checksum of downloaded binary against `checksums-sha256.txt`
- [ ] **UPDATE-04**: System creates automatic backup before replacing binary during update
- [ ] **UPDATE-05**: System verifies binary size after update (catches Windows 0-byte executable bug) and rolls back on failure
- [ ] **UPDATE-06**: Update uses `self_update` crate with `rustls` feature (no OpenSSL dependency)

### Build Matrix

- [ ] **BUILD-01**: Release produces 8 targets (linux-gnu x86_64, linux-musl x86_64, linux-gnu aarch64, linux-musl aarch64, macOS x86_64, macOS aarch64, Windows x64, Windows ARM64)
- [ ] **BUILD-02**: Release produces 12 artifacts (8× .tar.gz/.zip + .deb + .rpm for linux x64 gnu + 2× .msi for Windows)
- [ ] **BUILD-03**: Asset naming uses target-triple convention: `the-crab-engram-{version}-{target}.{ext}`
- [ ] **BUILD-04**: Linux musl builds use native `musl-tools` install (not cross-compilation)
- [ ] **BUILD-05**: ARM Linux uses `ubuntu-24.04-arm` native runner
- [ ] **BUILD-06**: ARM Windows uses `windows-11-arm` native runner
- [ ] **BUILD-07**: Build matrix uses `fail-fast: false` so one target failure doesn't cancel others

### Version Transparency

- [ ] **VERSION-01**: Enhanced `version` command shows version, git commit hash, commit date, and target triple
- [ ] **VERSION-02**: `build.rs` embeds git commit hash and date at compile time
- [ ] **VERSION-03**: Version output includes update hint ("Run `the-crab-engram update` to check for updates")

### Backup & Restore

- [ ] **BACKUP-01**: User can create manual backup with `the-crab-engram backup`
- [ ] **BACKUP-02**: User can create labeled backup with `the-crab-engram backup --label "description"`
- [ ] **BACKUP-03**: User can list backups with `the-crab-engram restore --list` (shows timestamps, sizes, triggers)
- [ ] **BACKUP-04**: User can restore backup by ID with `the-crab-engram restore --id N`
- [ ] **BACKUP-05**: User can restore backup by file with `the-crab-engram restore --file PATH`
- [ ] **BACKUP-06**: User can verify backup integrity with `the-crab-engram verify-backup FILE`
- [ ] **BACKUP-07**: System creates automatic backup before schema migration
- [ ] **BACKUP-08**: System creates automatic backup before data import
- [ ] **BACKUP-09**: Restore creates pre-restore backup before applying
- [ ] **BACKUP-10**: Restore verifies backup integrity before applying
- [ ] **BACKUP-11**: Restore requires confirmation unless `--yes` flag
- [ ] **BACKUP-12**: Backup uses `rusqlite::backup::Backup::run_to_completion()` (non-blocking, works while MCP active)
- [ ] **BACKUP-13**: Each backup includes metadata sidecar (`.meta.json`) with version, schema version, timestamp, trigger, size, checksum, stats
- [ ] **BACKUP-14**: System rotates old backups — keeps last 10 automatic backups, manual backups never auto-deleted
- [ ] **BACKUP-15**: Backup methods added to `Storage` trait (no new crate)

### Packaging — Debian/RPM

- [ ] **PKG-01**: Release includes `.deb` package for `x86_64-unknown-linux-gnu`
- [ ] **PKG-02**: Release includes `.rpm` package for `x86_64-unknown-linux-gnu`
- [ ] **PKG-03**: `Cargo.toml` includes `[package.metadata.deb]` configuration
- [ ] **PKG-04**: `Cargo.toml` includes `[package.metadata.rpm]` configuration

### Packaging — Homebrew

- [ ] **HOMEBREW-01**: Custom tap `maisonnat/homebrew-tap` with `Formula/the-crab-engram.rb`
- [ ] **HOMEBREW-02**: Formula uses musl binary for Linux (maximum portability)
- [ ] **HOMEBREW-03**: Release workflow triggers tap update via `repository_dispatch`
- [ ] **HOMEBREW-04**: Requires `HOMEBREW_TAP_TOKEN` secret configured

### Packaging — Windows

- [ ] **WIN-01**: Release includes `.msi` installer for `x86_64-pc-windows-msvc`
- [ ] **WIN-02**: Release includes `.msi` installer for `aarch64-pc-windows-msvc`
- [ ] **WIN-03**: MSI via `cargo-wix` (same as starship)
- [ ] **WIN-04**: MSI appears in "Add/Remove Programs"
- [ ] **WIN-05**: winget manifest submitted to `microsoft/winget-pkgs` via `winget-releaser@v2`
- [ ] **WIN-06**: Scoop bucket `maisonnat/scoop-bucket` with `checkver` + `autoupdate` manifest

### Install Scripts

- [ ] **INSTALL-01**: `scripts/install.sh` — Linux/macOS one-liner (prefers musl, SHA-256 verification)
- [ ] **INSTALL-02**: `scripts/install.ps1` — Windows PowerShell installer
- [ ] **INSTALL-03**: Scripts detect and avoid conflicting with existing package-manager installs

### Background Update Check

- [ ] **BG-01**: System checks for updates in background (once per 24 hours)
- [ ] **BG-02**: Update hint goes to stderr only (stdout is sacred for MCP JSON-RPC)
- [ ] **BG-03**: Respects `CRAB_ENGRAM_NO_UPDATE_CHECK=1` environment variable
- [ ] **BG-04**: Spawned in `Commands::Mcp` and `Commands::Serve` handlers

---

## v2 Requirements (Deferred)

- [ ] ARM Linux .deb/.rpm packages — deferred (x86_64 only for now)
- [ ] Homebrew core submission — too early for v2.0.0, custom tap sufficient
- [ ] Windows code signing via signpath — deferred (adds complexity)
- [ ] Changelog display during update — `self_update` doesn't natively support this

---

## Out of Scope

- **cargo-dist adoption** — requires replacing entire release workflow, unnecessary with `self_update`
- **Cross-compilation via `cross`/`cargo-zigbuild`** — native ARM runners available since Jan 2026
- **Cloud backup sync** — massive scope creep, users can sync `~/.engram/` themselves
- **Backup encryption** — SQLite backup preserves existing ChaCha20Poly1305 encryption if set
- **Delta/incremental backups** — full backup via `run_to_completion()` is <100ms, no need for complexity
- **GUI backup manager** — this is a CLI tool
- **Remote backup (S3/GCS)** — scope creep, users can script this
- **`cargo install` from source** — target audience is non-Rust users, document as unsupported

---

## Traceability

| Requirement | Phase | Research Source |
|-------------|-------|-----------------|
| UPDATE-01..06 | Self-Update Engine | FEATURES.md T1, ADR-001 |
| BUILD-01..07 | Expanded Build Matrix | FEATURES.md T6, ADR-002/006/007 |
| VERSION-01..03 | Version Transparency | FEATURES.md T3 |
| BACKUP-01..15 | Backup & Restore System | FEATURES.md D1/D2/D4/D5, ADR-010..014 |
| PKG-01..04 | Debian/RPM Packaging | FEATURES.md T4, ADR-004 |
| HOMEBREW-01..04 | Homebrew Tap | FEATURES.md T2, ADR-003 |
| WIN-01..06 | Windows Distribution | FEATURES.md T5, ADR-005/008/009 |
| INSTALL-01..03 | Install Scripts | FEATURES.md T2 |
| BG-01..04 | Background Update Check | FEATURES.md D3 |

---

*Generated: 2026-04-08 after research phase*
