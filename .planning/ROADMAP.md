# Roadmap — The Crab Engram v2.0.0

**Project:** Zero-friction self-updates, zero-data-loss backup/restore, cross-platform distribution
**Granularity:** Standard (8 phases)
**Requirements:** 52 v1 requirements, 100% coverage
**Generated:** 2026-04-08

---

## Phases

- [ ] **Phase 1: Build Matrix** — Expand release to 8 targets, 12 artifacts with target-triple naming
- [ ] **Phase 2: Self-Update Engine** — In-binary update with checksum verification and rollback safety
- [ ] **Phase 3: Version Transparency** — Enhanced version command with commit hash, date, and update hint
- [ ] **Phase 4: Backup Core** — Manual backup with labels, verification, metadata sidecars, and rotation
- [ ] **Phase 5: Restore & Auto-Backup** — Restore workflow with safety backups, auto-backup before mutations
- [ ] **Phase 6: Packaging — Linux, macOS, Windows** — .deb, .rpm, Homebrew tap, MSI, winget, Scoop
- [ ] **Phase 7: Install Scripts** — Standalone curl|sh and PowerShell installers with checksum verification
- [ ] **Phase 8: Background Update Check** — Non-intrusive stderr update hint, 24h throttle, opt-out

---

## Phase Details

### Phase 1: Build Matrix

**Goal**: Release produces 8 platform targets and 12 artifacts with consistent naming
**Depends on**: Nothing
**Requirements**: BUILD-01, BUILD-02, BUILD-03, BUILD-04, BUILD-05, BUILD-06, BUILD-07
**Success Criteria** (what must be TRUE):
  1. Running the release workflow produces artifacts for all 8 targets (linux-gnu x2, linux-musl x2, macOS x2, Windows x2)
  2. Release includes 10 artifacts (8 archives + .deb + checksums-sha256.txt); .rpm and .msi deferred to Phase 6
  3. Every archive is named with target triple: `the-crab-engram-{version}-{target}.{ext}`
  4. All Linux targets build with `cross` (pinned v0.2.5) — no manual musl-tools or native ARM runners
  5. Windows ARM64 uses `windows-11-arm` native runner
  6. A single target failure does not cancel other targets (`fail-fast: false`)
**Plans**: 1 plan
- [x] 01-01-PLAN.md — Expand build matrix, add .deb job, update release job

### Phase 2: Self-Update Engine

**Goal**: User can update the binary in-place with a single command, safely
**Depends on**: Phase 1
**Requirements**: UPDATE-01, UPDATE-02, UPDATE-03, UPDATE-04, UPDATE-05, UPDATE-06
**Success Criteria** (what must be TRUE):
  1. User runs `the-crab-engram update` and gets the latest release from GitHub
  2. User runs `the-crab-engram update --check-only` and sees available version without downloading
  3. Downloaded binary is verified against `checksums-sha256.txt` (SHA-256)
  4. System creates automatic backup before replacing binary during update
  5. If update produces a 0-byte or corrupt binary (Windows edge case), system rolls back to previous version
  6. Update uses `self_update` with `rustls` — no OpenSSL dependency
**Plans**: TBD

### Phase 3: Version Transparency

**Goal**: User can always identify exactly what version is running
**Depends on**: Nothing
**Requirements**: VERSION-01, VERSION-02, VERSION-03
**Success Criteria** (what must be TRUE):
  1. `the-crab-engram --version` shows version, git commit hash, commit date, and target triple
  2. Git metadata is embedded at compile time via `build.rs`
  3. Version output includes update hint: "Run `the-crab-engram update` to check for updates"
**Plans**: TBD

### Phase 4: Backup Core

**Goal**: User can manually create and verify backups of their Engram knowledge store
**Depends on**: Nothing
**Requirements**: BACKUP-01, BACKUP-02, BACKUP-06, BACKUP-12, BACKUP-13, BACKUP-14, BACKUP-15
**Success Criteria** (what must be TRUE):
  1. User runs `the-crab-engram backup` and gets a timestamped backup file
  2. User runs `the-crab-engram backup --label "before experiment"` and the label appears in backup listing
  3. User runs `the-crab-engram verify-backup FILE` and confirms backup integrity
  4. Backup uses `rusqlite::backup::Backup::run_to_completion()` — non-blocking, works while MCP server is active
  5. Each backup includes a `.meta.json` sidecar with version, schema version, timestamp, trigger, size, checksum, and stats
  6. Auto-backups rotate (last 10 kept), manual backups are never auto-deleted
  7. Backup methods live on the `Storage` trait — no new crate required
**Plans**: TBD

### Phase 5: Restore & Auto-Backup

**Goal**: User can safely restore from any backup, and the system auto-protects before dangerous operations
**Depends on**: Phase 4
**Requirements**: BACKUP-03, BACKUP-04, BACKUP-05, BACKUP-07, BACKUP-08, BACKUP-09, BACKUP-10, BACKUP-11
**Success Criteria** (what must be TRUE):
  1. User runs `the-crab-engram restore --list` and sees all backups with timestamps, sizes, and triggers
  2. User runs `the-crab-engram restore --id N` and restores a specific backup by ID
  3. User runs `the-crab-engram restore --file PATH` and restores from an explicit backup file
  4. System verifies backup integrity before applying any restore
  5. System creates a pre-restore backup before overwriting current data
  6. System creates automatic backup before schema migrations
  7. System creates automatic backup before data imports
  8. Restore requires user confirmation unless `--yes` flag is passed
**Plans**: TBD

### Phase 6: Packaging — Linux, macOS, Windows

**Goal**: Users can install via their platform's native package manager
**Depends on**: Phase 1, Phase 2
**Requirements**: PKG-01, PKG-02, PKG-03, PKG-04, HOMEBREW-01, HOMEBREW-02, HOMEBREW-03, HOMEBREW-04, WIN-01, WIN-02, WIN-03, WIN-04, WIN-05, WIN-06
**Success Criteria** (what must be TRUE):
  1. User can install on Debian/Ubuntu via `apt install` from .deb package
  2. User can install on Fedora/RHEL via `rpm -i` from .rpm package
  3. User can install on macOS via `brew install maisonnat/tap/the-crab-engram`
  4. Homebrew formula auto-updates on release via `repository_dispatch`
  5. User can install on Windows via `winget install the-crab-engram`
  6. User can install on Windows via `scoop install the-crab-engram`
  7. User can install on Windows via MSI installer (appears in Add/Remove Programs)
  8. MSI installer exists for both x86_64 and aarch64 Windows
**Plans**: TBD

### Phase 7: Install Scripts

**Goal**: Users can install with a one-liner, regardless of platform
**Depends on**: Phase 1, Phase 2
**Requirements**: INSTALL-01, INSTALL-02, INSTALL-03
**Success Criteria** (what must be TRUE):
  1. User runs `curl -fsSL https://.../install.sh | sh` on Linux/macOS and binary is installed
  2. User runs PowerShell install script on Windows and binary is installed
  3. Scripts verify SHA-256 checksums of downloaded binaries
  4. Scripts detect and warn if a package-manager version is already installed
**Plans**: TBD

### Phase 8: Background Update Check

**Goal**: User gets a non-intrusive hint when a new version is available
**Depends on**: Phase 2
**Requirements**: BG-01, BG-02, BG-03, BG-04
**Success Criteria** (what must be TRUE):
  1. User sees a subtle update hint on stderr when a newer version exists (once per 24h)
  2. Update hint never appears on stdout (MCP JSON-RPC protocol is never corrupted)
  3. User can disable background check with `CRAB_ENGRAM_NO_UPDATE_CHECK=1`
  4. Background check fires in `Commands::Mcp` and `Commands::Serve` handlers
**Plans**: TBD

---

## Progress Table

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Build Matrix | 0/1 | Planning complete | - |
| 2. Self-Update Engine | 0/6 | Not started | - |
| 3. Version Transparency | 0/3 | Not started | - |
| 4. Backup Core | 0/7 | Not started | - |
| 5. Restore & Auto-Backup | 0/8 | Not started | - |
| 6. Packaging | 0/14 | Not started | - |
| 7. Install Scripts | 0/3 | Not started | - |
| 8. Background Update Check | 0/4 | Not started | - |

---

## Dependency Graph

```
Phase 1 (Build Matrix)
  ├── Phase 2 (Self-Update) ← needs build artifacts for asset naming
  │     ├── Phase 6 (Packaging) ← needs artifacts + update hint for formula
  │     ├── Phase 7 (Install Scripts) ← needs artifacts + update mechanism
  │     └── Phase 8 (Background Check) ← needs self_update mechanism
  │
Phase 3 (Version) ← independent
Phase 4 (Backup Core) ← independent
  └── Phase 5 (Restore & Auto-Backup) ← needs backup engine from Phase 4
```

**Critical path:** Phase 1 → Phase 2 → Phase 6/7/8
**Independent threads:** Phase 3, Phase 4 → Phase 5

---

*Generated: 2026-04-08 by GSD Roadmapper*
