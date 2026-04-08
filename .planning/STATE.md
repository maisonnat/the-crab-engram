# Project State — The Crab Engram v2.0.0

## Project Reference

- **Core Value**: The agent's brain must never be lost. All update, distribution, and packaging work must preserve data safety.
- **Current Focus**: Roadmap creation — phase structure approved, ready for planning
- **Milestone**: v2.0.0 — zero-friction self-updates, zero-data-loss backup/restore, cross-platform distribution

## Current Position

- **Phase**: None (roadmap just created)
- **Plan**: None
- **Status**: Awaiting roadmap approval
- **Progress**: `░░░░░░░░░░░░░░░░░░░░ 0/8 phases`

## Performance Metrics

| Metric | Value |
|--------|-------|
| Total Requirements | 52 |
| Phases | 8 |
| Requirements/Phase (avg) | 6.5 |
| Critical Path Length | 4 phases (1→2→6/7/8) |
| Parallel Tracks | 2 (Build+Update vs Backup) |

## Accumulated Context

### Key Decisions (from research)

- `self_update` v0.44.0 (not v0.27 referenced in PROJECT.md — 17 releases behind)
- Native ARM runners (not cross-compilation) — GA since Jan 2026
- `rusqlite::backup::Backup` for online SQLite backup (<100ms typical)
- Custom Homebrew tap (not core) — faster release velocity
- Backup as Storage trait methods — no new crate

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

---

*Last updated: 2026-04-08 — Initial roadmap creation*
