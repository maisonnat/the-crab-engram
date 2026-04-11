---
gsd_state_version: 1.0
milestone: v2.0.0
milestone_name: milestone
status: Ready to plan
last_updated: "2026-04-08T23:09:53.429Z"
progress:
  total_phases: 8
  completed_phases: 1
  total_plans: 1
  completed_plans: 1
  percent: 100
---

# Project State — The Crab Engram v2.0.0

## Project Reference

- **Core Value**: The agent's brain must never be lost. All update, distribution, and packaging work must preserve data safety.
- **Current Focus**: Roadmap creation — phase structure approved, ready for planning
- **Milestone**: v2.0.0 — zero-friction self-updates, zero-data-loss backup/restore, cross-platform distribution

## Current Position

- **Phase**: 02-self-update-engine
- **Plan**: 01 / 1 (completed)
- **Current Plan**: 01 / 1
- **Status**: Plan 01 completed, ready for next plan
- **Progress**: `█░░░░░░░░░░░░░░░░░░░ 1/8 phases`

## Performance Metrics

| Metric | Value |
|--------|-------|
| Total Requirements | 52 |
| Phases | 8 |
| Requirements/Phase (avg) | 6.5 |
| Critical Path Length | 4 phases (1→2→6/7/8) |
| Parallel Tracks | 2 (Build+Update vs Backup) |
| Phase 01-build-matrix P01 | 5min | 3 tasks | 1 files |
| Phase 02-self-update-engine P01 | 35min | 2 tasks | 2 files |

## Accumulated Context

### Key Decisions (from research)

- `self_update` v0.44.0 (master plan cited v0.27 — corrected after crates.io verification)
- Native ARM runners (not cross-compilation) — GA since Jan 2026
- `rusqlite::backup::Backup` for online SQLite backup (<100ms typical)
- Custom Homebrew tap (not core) — faster release velocity
- Backup as Storage trait methods — no new crate

### Key Decisions (from execution)

- Use `cross` for ALL Linux targets (not native ARM runners) — industry standard, simpler CI
- Use `windows-11-arm` native runner for Windows ARM64 — cross doesn't support Windows
- Target-triple naming: `the-crab-engram-{version}-{target}.{ext}` — required by self_update
- `.deb` in separate job using `cargo-deb` — follows ripgrep pattern
- Pin `cross` to v0.2.5 — avoid upstream breakage
- `self` namespace for self-update commands (D-01) — groups self-management logically
- Hardcoded repo owner/name constants (D-02) — simple, no env var override needed for v1
- No persistent binary backup (D-03) — atomic replace + post-update size verification
- No interactive confirmation prompt (D-04) — user intent assumed, --dry-run flag provided
- All error messages colored and actionable (D-05) with recovery hints
- Self-update never touches SQLite database (D-06) — only binary executable modified
- self_update v0.44.0 with rustls + archive features (D-07) — no OpenSSL dependency

### Critical Pitfalls to Watch

1. **Asset naming mismatch** — `self_update` needs full target triple in archive names
2. **Windows 0-byte executable** — post-update size verification + automatic rollback
3. **Backup mutex deadlock** — single `Mutex<Connection>` needs careful batch handling
4. **stdout contamination** — all update/background output must go to `eprintln!`
5. **Pre-migration backup race** — must use raw `rusqlite::Connection`, not `SqliteStore::new()`

### Research Flags

- Phase 4 (Backup Core): mutex-safe design review needed during planning
- Phase 6 (Packaging): `cargo-wix` .wxs template + `winget-releaser@v2` compatibility check

## Session Continuity

| Session | Date | Activity | Notes |
|---------|------|----------|-------|
| 1 | 2026-04-08 | Research + Roadmap | 52 requirements, 8 phases, 100% coverage |
| 2 | 2026-04-08 | Execute Phase 01 Plan 01 | Expanded release workflow to 8 targets with cross-compilation |
| 3 | 2026-04-11 | Execute Phase 02 Plan 01 | Implemented self-update CLI subcommand with checksum verification and binary size check |

---

*Last updated: 2026-04-11 — Phase 02 Plan 01 completed*
