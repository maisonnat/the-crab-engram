# Feature Landscape: Update, Backup & Distribution for Rust CLI

**Domain:** Rust CLI tool — self-update, backup/restore, cross-platform distribution
**Researched:** 2026-04-08
**Confidence:** HIGH (direct evidence from uv, starship, ripgrep, SQLite official docs)

---

## Industry Benchmark Summary

| Feature | uv | starship | ripgrep | The Crab Engram (target) |
|---|---|---|---|---|
| `self update` command | ✅ `uv self update` | ❌ | ❌ | ✅ |
| `--check-only` flag | ✅ (inherent) | ❌ | ❌ | ✅ |
| curl\|sh installer | ✅ | ✅ | ❌ | ✅ |
| Homebrew | ✅ (core) | ✅ (core) | ✅ (core) | ✅ (custom tap) |
| winget | ✅ | ✅ | ❌ | ✅ |
| Scoop | ✅ | ✅ | ❌ | ✅ |
| .deb packages | ❌ | ❌ | ✅ | ✅ |
| .rpm packages | ❌ | ❌ | ✅ | ✅ |
| MSI installer | ❌ | ✅ | ❌ | ✅ |
| Background update check | ❌ | ❌ | ❌ | ✅ |
| Automatic backup | ❌ | ❌ | ❌ | ✅ |
| Manual backup/restore | ❌ | ❌ | ❌ | ✅ |

**Key insight:** NO major Rust CLI has automatic backup of user data. The Crab Engram would be first-to-market on this.

---

## Table Stakes

Features users expect. Missing = product feels incomplete, users consider alternatives.

### T1: Self-Update Command

| Feature | Why Expected | Complexity | Notes |
|---|---|---|---|
| `the-crab-engram update` | uv has it, users expect it on any data-critical tool. Manual download+replace is unacceptable for daily-use tools. | Medium | `self_update` crate v0.44.0, works out of the box with target-triple asset naming |
| `the-crab-engram update --check-only` | Users want to know before committing to download. uv lets you check version without upgrading. | Low | Just fetches latest release metadata, no download |

**Source:** uv's `self update` sets the modern standard. Users coming from uv/gh expect this.

### T2: Multiple Install Channels

| Feature | Why Expected | Complexity | Notes |
|---|---|---|---|
| Homebrew (macOS/Linux) | Starship, ripgrep, uv all have it. macOS users almost exclusively install via brew. | Low | Custom tap `maisonnat/homebrew-tap`, formula auto-updated via CI |
| winget (Windows 11) | Built into Windows 11. Starship and uv both support it. | Low | `winget-releaser` GitHub Action automates PR submission |
| curl\|sh one-liner | uv's primary install method. Expected by Unix users. | Low | `install.sh` script, prefers musl for portability |
| PowerShell installer | Windows users without winget/scoop need this. uv has `irm \| iex`. | Low | `install.ps1` script |

**Source:** Starship ships brew/winget/scoop + MSI. uv ships brew/winget/scoop + curl|sh.

### T3: Version Transparency

| Feature | Why Expected | Complexity | Notes |
|---|---|---|---|
| `the-crab-engram version` with commit hash | Essential for debugging. Users paste this in bug reports. | Low | `build.rs` embeds git commit + date at compile time |
| Target triple in version output | Helps users verify correct binary was downloaded. | Low | `std::env::consts::ARCH` |

**Source:** uv, ripgrep, starship all show version + commit info.

### T4: Platform-Native Packaging

| Feature | Why Expected | Complexity | Notes |
|---|---|---|---|
| .deb packages | ripgrep, fd, bat all ship .deb. Ubuntu/Debian users expect it. | Low | `cargo-deb` — de facto standard |
| .rpm packages | ripgrep, fd, bat all ship .rpm. Enterprise Linux users need it. | Low | `cargo-generate-rpm` |
| Target-triple asset names | Every major Rust CLI uses this. Required for `self_update` to auto-match. | None | Already the plan (`the-crab-engram-{version}-{target}.{ext}`) |

**Source:** ripgrep, fd, bat all use `cargo-deb` + `cargo-generate-rpm`. This is the Rust standard.

### T5: Windows MSI Installer

| Feature | Why Expected | Complexity | Notes |
|---|---|---|---|
| .msi installer | Enterprise deployment (Group Policy, SCCM). Starship ships MSI. | Low | `cargo-wix` v0.3.8 |
| Appears in "Add/Remove Programs" | Expected by Windows users. | Low | MSI handles this natively |

**Source:** Starship uses `cargo-wix`. MSI is the Windows standard for installed software.

### T6: Cross-Platform Binaries

| Feature | Why Expected | Complexity | Notes |
|---|---|---|---|
| Linux gnu + musl builds | musl = zero-dependency portability (Alpine, any Linux). gnu = .deb/.rpm compatibility. | Low | Native ARM runners since Jan 2026 |
| macOS x86_64 + ARM64 | Rosetta works but native ARM is expected. | Low | `macos-13` (Intel) + `macos-latest` (ARM) |
| Windows x64 + ARM64 | Growing ARM Windows market. | Low | `windows-11-arm` runner |

**Source:** uv ships 12+ targets. Starship ships 13. 8 targets is minimum competitive parity.

---

## Differentiators

Features that set The Crab Engram apart. Not expected, but HIGHLY valued.

### D1: Automatic Pre-Operation Backups (UNIQUE — nobody else does this)

| Feature | Value Proposition | Complexity | Notes |
|---|---|---|---|
| Pre-update backup | If update breaks something, user can restore in seconds. Zero data loss on update. | Medium | Triggered before `self_update` replaces binary |
| Pre-migration backup | Schema migrations can corrupt data. Automatic backup = safety net. | Medium | Triggered before `ALTER TABLE` in migration.rs |
| Pre-import backup | `engram import` overwriting data is a real risk. Backup before import. | Low | Triggered before import handler runs |
| Pre-restore backup | Before restoring a backup, save current state in case restore was wrong. | Low | Self-defense: "undo the undo" |

**Why this differentiates:** The agent's brain (knowledge graph, observations, beliefs) is IRREPLACEABLE. uv, starship, ripgrep don't store user state — they can be reinstalled. Engram stores months of accumulated knowledge. Automatic backup is the killer feature.

**Source:** Confirmed via SQLite official docs — `rusqlite::backup::Backup::run_to_completion()` is non-blocking, works while MCP server is active, completes in <100ms for typical databases.

### D2: Manual Backup & Restore CLI

| Feature | Value Proposition | Complexity | Notes |
|---|---|---|---|
| `the-crab-engram backup` | User can snapshot before experiments, risky operations. Peace of mind. | Medium | Creates labeled backup with metadata sidecar |
| `the-crab-engram backup --label "v1 setup"` | Named backups for organization. | Low | Stored in `.meta.json` |
| `the-crab-engram restore --list` | Browse available backups with timestamps, sizes, triggers. | Low | Scan backup dir, parse sidecars |
| `the-crab-engram restore --id N` | One-command restore. | Medium | Verify → pre-restore backup → atomic rename → post-restore verify |
| `the-crab-engram verify-backup FILE` | Check backup integrity without restoring. | Low | `PRAGMA integrity_check` + SHA-256 verification |

**Why this differentiates:** No other Rust CLI tool offers built-in backup/restore for its data store. This positions Engram as the "enterprise-grade" memory system.

### D3: Background Update Check

| Feature | Value Proposition | Complexity | Notes |
|---|---|---|---|
| Background update hint on stderr | Non-intrusive nudge. "Hey, v2.1 is available." | Medium | Once per 24h, stderr only, respects `CRAB_ENGRAM_NO_UPDATE_CHECK=1` |
| `--check-only` flag | Check without committing. uv-adjacent UX. | Low | Fetches latest release metadata |

**Why this differentiates:** Most Rust CLIs don't check for updates at all. A non-intrusive nudge (stderr only, once/day) is a premium UX pattern. Critical constraint: stdout is sacred for MCP JSON-RPC.

### D4: Backup Metadata & Transparency

| Feature | Value Proposition | Complexity | Notes |
|---|---|---|---|
| Backup metadata sidecar (`.meta.json`) | Users know exactly what's in each backup: version, schema, stats, encryption status. | Low | JSON sidecar with SHA-256, stats, trigger |
| `--check-only` with backup size | "How big is this going to be?" before download. | Low | GitHub API returns content-length |

**Why this differentiates:** Transparency builds trust. Users managing their agent's brain want to know exactly what state they're restoring to.

### D5: Smart Backup Rotation

| Feature | Value Proposition | Complexity | Notes |
|---|---|---|---|
| 10 auto-backup rotation | Bounded disk usage (~150MB). Users don't have to think about cleanup. | Low | Oldest auto-backups deleted first |
| Manual backups never auto-deleted | User-created backups are sacred. Only user can delete them. | None | Separate naming convention |

---

## Anti-Features

Features to explicitly NOT build.

| Anti-Feature | Why Avoid | What to Do Instead |
|---|---|---|
| **Cloud backup sync** | Massive scope creep. Users can sync `~/.engram/` themselves (rsync, Dropbox, etc.). | Document recommended sync methods in README |
| **Backup encryption** | The database itself already supports ChaCha20Poly1305. Re-encrypting backups adds complexity with no user value. | SQLite backup preserves existing encryption if set |
| **Delta/incremental backups** | SQLite backup API does full copies. Deltas are complex and unnecessary — typical databases are <50MB. | Full backup is <100ms via `run_to_completion()` |
| **GUI backup manager** | This is a CLI tool. Adding a GUI breaks the single-binary, zero-dependency philosophy. | TUI dashboard can show backup status (already exists) |
| **Backup to remote (S3/GCS)** | Scope creep. Users can script this. | Provide good backup path configuration |
| **Semver-compliant auto-migration with rollback** | Extremely complex. Current schema is stable at v16. | Pre-migration backup + `PRAGMA integrity_check` |
| **Changelog shown during update** | `self_update` doesn't support this natively. Users can check GitHub. | Link to release notes in update output |
| **`cargo-dist` adoption** | Replaces entire release workflow. `self_update` achieves the same without coupling. | Use `self_update` v0.44.0 |
| **Cross-compilation toolchain** | GitHub ARM runners went GA Jan 2026. No need for `cross`/`cargo-zigbuild`. | Native ARM runners (`ubuntu-24.04-arm`, `windows-11-arm`) |
| **Homebrew core submission** | Too early at v2.0.0. Core has strict review criteria and slow merge cycle. | Custom tap (`maisonnat/homebrew-tap`) for velocity |
| **Cargo install support** | Building from source requires Rust toolchain. Target audience is non-Rust users. | Document `cargo install` as unsupported/last resort |

---

## Feature Dependencies

```
Phase 2 (Build Matrix) ──────────────────────────┐
  │                                                │
  ├── Phase 1 (Self-Update) ──────────────────────┤
  │     ├── Phase 9 (Background Check)             │
  │     └── Pre-update backup (D1)                 │
  │                                                │
  ├── Phase 5 (Deb/RPM) ──────────────────────────┤
  │                                                │
  ├── Phase 7 (Winget/Scoop/MSI) ─────────────────┤
  │                                                │
  └── Phase 8 (Install Scripts) ──────────────────┤
                                                    │
Phase 4 (Backup/Restore) ─────────────────────────┤
  ├── Pre-migration backup (D1)                    │
  ├── Pre-import backup (D1)                       │
  └── Pre-restore backup (D1)                      │
                                                    │
Phase 3 (Version Info) ───────────────────────────┤
                                                    │
Phase 6 (Homebrew Tap) ───────────────────────────┘
  (needs Phase 2 artifacts + Phase 1 for self-update hint)
```

**Critical path:** Phase 2 (Build Matrix) must complete first — Phases 5-8 reference its artifacts.

**Phase ordering rationale:**
1. **P2+P1+P3+P4** — Foundation. Can be parallelized (different files).
2. **P5+P7+P8** — Packaging & distribution. All depend on P2 artifacts.
3. **P6+P9** — Polish. P6 needs P2 artifacts, P9 needs P1.

---

## MVP Recommendation

**Minimum competitive product needs:**

1. ✅ **Self-update command** (T1) — Table stakes, uv parity
2. ✅ **Homebrew + winget + curl|sh** (T2) — Install channels
3. ✅ **Version with commit hash** (T3) — Debugging essential
4. ✅ **.deb + .rpm packages** (T4) — Linux users
5. ✅ **Target-triple naming** (T4) — Required for self_update
6. ✅ **Pre-update automatic backup** (D1) — THE differentiator
7. ✅ **Manual backup/restore** (D2) — Data safety
8. ✅ **Background update check** (D3) — Premium UX

**Can defer:**
- MSI installer: Nice-to-have, starship parity, but not blocking
- Scoop: Secondary Windows channel
- ARM builds for packaging (.deb/.rpm): x86_64 only initially
- Homebrew core: Too early

**The minimum viable differentiator is automatic backup.** That's what makes Engram unique. Everything else is table-stakes parity with uv/starship.

---

## Confidence Assessment

| Feature Category | Confidence | Source Evidence |
|---|---|---|
| Self-update patterns | HIGH | `self_update` crate docs (jaemk/self_update), uv docs, Context7 rusqlite backup API |
| Package manager support | HIGH | Starship install page, uv installation docs, ripgrep releases |
| SQLite backup capability | HIGH | SQLite official backup API docs, rusqlite backup feature confirmed |
| musl builds | HIGH | ADR-007 in master plan, GitHub ARM runners GA Jan 2026 |
| MSI via cargo-wix | MEDIUM | Starship uses it, `cargo-wix` v0.3.8 is standard |
| Background check stderr-only | HIGH | MCP uses stdio for JSON-RPC — architectural constraint confirmed |
| No other CLI has automatic backup | HIGH | Verified against uv, starship, ripgrep, fd, bat — none offer it |

## Gaps to Address

- **Scoop bucket maintenance**: How exactly does `checkver` + `autoupdate` work? Needs Phase 7 research.
- **winget PR automation**: `vedantmgoyal9/winget-releaser@v2` — verify it still works in 2026. Needs Phase 7 research.
- **cargo-wix template**: How does starship structure its `.wxs` file? May need to reference their repo. Phase 7.
- **Install script edge cases**: Alpine Linux without curl? Needs testing during Phase 8.

---

*Sources:*
- *uv installation docs — https://docs.astral.sh/uv/getting-started/installation/ [HIGH]*
- *Starship advanced installation — https://starship.rs/installing/ [HIGH]*
- *self_update crate — https://github.com/jaemk/self_update [HIGH]*
- *SQLite Backup API — https://www.sqlite.org/backup.html [HIGH]*
- *rusqlite backup feature — Context7 /rusqlite/rusqlite [HIGH]*
- *Master plan ADRs — the-crab-engram-master-plan.md [HIGH]*
