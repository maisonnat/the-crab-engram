---
phase: 01-build-matrix
plan: 01
subsystem: infra
tags: [github-actions, cross-compilation, release, packaging]

# Dependency graph
requires:
  - phase: none
    provides: initial release workflow
provides:
  - 8-target build matrix with cross for Linux, native cargo for macOS/Windows
  - target-triple archive naming for self_update compatibility
  - separate .deb packaging job using cargo-deb
  - updated release job collecting all artifacts with checksums
affects: [02-self-update, 06-packaging]

# Tech tracking
tech-stack:
  added: [cross v0.2.5, cargo-deb]
  patterns: [matrix strategy with use_cross flag, separate packaging jobs]

key-files:
  created: []
  modified: [.github/workflows/release.yml]

key-decisions:
  - "Use cross for ALL Linux targets (not native ARM runners)"
  - "Use windows-11-arm native runner for Windows ARM64"
  - "Target-triple naming: the-crab-engram-{version}-{target}.{ext}"
  - ".deb in separate job using cargo-deb"
  - "Pin cross to v0.2.5"

patterns-established:
  - "Matrix include with use_cross flag for conditional cross/cargo builds"
  - "Separate packaging jobs for platform-specific formats"

requirements-completed: [BUILD-01, BUILD-02, BUILD-03, BUILD-04, BUILD-05, BUILD-06, BUILD-07]

# Metrics
duration: 5min
completed: 2026-04-08
---

# Phase 01: Build Matrix Summary

**Expanded release workflow from 3 targets to 8 targets with cross-compilation for Linux, target-triple naming, and .deb packaging**

## Performance

- **Duration:** 5 min
- **Started:** 2026-04-08T18:59:43Z
- **Completed:** 2026-04-08T19:04:00Z
- **Tasks:** 3 (completed in single commit)
- **Files modified:** 1

## Accomplishments
- 8-target build matrix covering Linux (gnu/musl x86_64/aarch64), macOS (x86_64/aarch64), Windows (x86_64/aarch64)
- Cross-compilation for all Linux targets using pinned cross v0.2.5
- Target-triple archive naming (the-crab-engram-{version}-{target}.{ext}) for self_update compatibility
- Separate .deb packaging job using cargo-deb for x86_64-unknown-linux-gnu
- Updated release job depends on both build and .deb jobs, includes .deb in checksums and uploads

## task Commits

Each task was committed atomically:

1. **task 1: Expand build matrix to 8 targets with cross-compilation** - `bbda8c2` (feat)
2. **task 2: Add separate .deb packaging job** - `bbda8c2` (included in same commit)
3. **task 3: Update release job to collect all artifacts and generate checksums** - `bbda8c2` (included in same commit)

**Plan metadata:** `bbda8c2` (docs: complete plan)

_Note: All three tasks were completed in a single commit because the changes were interdependent and modifying the same file._

## Files Created/Modified
- `.github/workflows/release.yml` - Expanded from 3 to 8 targets, added cross compilation, .deb job, updated release dependencies

## Decisions Made
- Used cross for ALL Linux targets (industry standard: starship, ripgrep, fd)
- Used windows-11-arm native runner for Windows ARM64 (cross doesn't support Windows)
- Target-triple naming required by self_update crate for asset discovery
- .deb in separate job following ripgrep pattern
- Pinned cross to v0.2.5 to avoid upstream breakage

## Deviations from Plan

None - plan executed exactly as written. All three tasks were completed successfully with no deviations.

## Issues Encountered
- None - plan executed smoothly

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Build matrix ready for Phase 2 (self-update) which requires target-triple named archives
- .deb packaging ready for Phase 6 (packaging) expansion
- Windows ARM64 runner configured for future Windows ARM support

---
*Phase: 01-build-matrix*
*Completed: 2026-04-08*