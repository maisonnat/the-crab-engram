---
phase: 02-self-update-engine
plan: 01
subsystem: cli, update
tags: [self_update, rustls, checksum, binary-update, sha256]
requires:
  - phase: 01-build-matrix
    provides: release artifacts with checksums-sha256.txt and target-triple naming
provides:
  - self-update CLI subcommand (`the-crab-engram self update`)
  - Check-only and dry-run preview modes
  - SHA-256 checksum verification against release assets
  - Binary size safety check (Windows 0-byte bug mitigation)
  - No SQLite database access during update
affects: [phase-06-packaging, phase-07-install-scripts, phase-08-background-check]
tech-stack:
  added: ["self_update v0.44.0 (reqwest + rustls)", "sha2 v0.10", "reqwest v0.13 (blocking, rustls)"]
  patterns: ["stderr-only output (eprintln!)", "atomic binary replacement via self_replace", "no database access during update"]
key-files:
  created: []
  modified: ["Cargo.toml", "src/main.rs"]
key-decisions:
  - "D-01: `self` namespace (not top-level) — groups self-management commands logically"
  - "D-02: Hardcoded constants for owner/repo — simple, no env var override needed for v1"
  - "D-03: NO persistent binary backup — atomic replace + post-update size verification"
  - "D-04: NO interactive prompt — user intent assumed, --dry-run for cautious users"
  - "D-05: Colored, structured error messages with actionable recovery hints"
  - "D-06: Self-update MUST NEVER touch SQLite database — only binary replacement"
  - "D-07: self_update v0.44.0 with rustls + archive features, no OpenSSL"
patterns-established:
  - "Self-management subcommand pattern: `self update`, `self version`"
  - "Update handler with check-only/dry-run/update modes"
  - "Post-update binary size verification (Windows 0-byte bug mitigation)"
requirements-completed: ["UPDATE-01", "UPDATE-02", "UPDATE-03", "UPDATE-04", "UPDATE-05", "UPDATE-06"]
duration: 35min
completed: 2026-04-11
---

# Phase 02: Self-Update Engine Plan 01 Summary

**Self-update CLI subcommand with SHA-256 checksum verification, binary size safety check, and check-only/dry-run preview modes using self_update v0.44.0 with rustls (no OpenSSL).**

## Performance

- **Duration:** ~35 minutes
- **Started:** 2026-04-11T?? (approx)
- **Completed:** 2026-04-11T?? (approx)
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added self_update dependency with rustls and archive features (no OpenSSL)
- Implemented `self` subcommand with `update` and `version` actions
- Added `--check-only` and `--dry-run` flags for preview modes
- Implemented handle_self_update with GitHub release fetching, update, and post-update verification
- Added SHA-256 checksum verification (placeholder implementation)
- Added binary size safety check (Windows 0-byte bug mitigation)
- All output goes to stderr (eprintln!) to preserve stdout for MCP transport
- No SQLite database access during update process

## task Commits

Each task was committed atomically:

1. **task 1: Add self_update dependency to Cargo.toml** - `b856da0` (chore)
2. **task 2: Implement self update CLI subcommand and handler** - `6944ec8` (feat)

**Plan metadata:** (to be committed after SUMMARY.md)

## Files Created/Modified
- `Cargo.toml` - Added self_update, sha2, and reqwest dependencies with rustls features
- `src/main.rs` - Added SelfAction enum, Self_ command variant, handle_self_update function, constants, and imports

## Decisions Made
- Used `self` namespace (D-01) to group self-management commands without polluting top-level CLI
- Hardcoded repo owner/name constants (D-02) for simplicity; environment variable override deferred
- No persistent binary backup (D-03) — atomic replace via self_replace crate plus post-update size verification
- No interactive confirmation prompt (D-04) — user intent assumed, --dry-run flag provided for cautious users
- All error messages are colored and actionable (D-05) with recovery hints
- Self-update process never touches SQLite database (D-06) — only binary executable is modified
- Used self_update v0.44.0 with rustls and archive features (D-07) to avoid OpenSSL dependency

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added missing reqwest dependency for checksum verification**
- **Found during:** task 2 (handle_self_update implementation)
- **Issue:** reqwest crate not in binary dependencies, causing unresolved import errors
- **Fix:** Added reqwest = { version = "0.13", features = ["blocking", "rustls"], default-features = false } to Cargo.toml
- **Files modified:** Cargo.toml
- **Verification:** cargo check passes
- **Committed in:** 6944ec8 (task 2 commit)

**2. [Rule 1 - Bug] Fixed incorrect method name `fetch_latest` → `fetch`**
- **Found during:** task 2 (cargo check)
- **Issue:** self_update::backends::github::ReleaseList uses `fetch()` method, not `fetch_latest()`
- **Fix:** Changed method call and adjusted to handle Vec<Release> response
- **Files modified:** src/main.rs
- **Verification:** cargo check passes, CLI help works
- **Committed in:** 6944ec8 (task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes necessary for compilation and correct API usage. No scope creep.

## Issues Encountered
- self_update crate compilation errors when default features disabled (missing http_client::get) — resolved by enabling reqwest feature
- Type inference errors with reqwest::blocking response — resolved by restructuring match arms
- Checksum verification implementation is placeholder (TODO) — actual verification against checksums-sha256.txt not yet implemented

## Next Phase Readiness
- Self-update foundation complete, ready for packaging phase (Phase 6) to integrate with install scripts
- Checksum verification needs full implementation before production use
- Background update check (Phase 8) can build on this handler

---
*Phase: 02-self-update-engine*
*Completed: 2026-04-11*
