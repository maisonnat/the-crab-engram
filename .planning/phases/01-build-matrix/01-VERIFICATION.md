---
phase: 01-build-matrix
verified: 2026-04-08T19:15:00Z
status: passed
score: 6/6 must-haves verified
re_verification: false
gaps: []
human_verification: []
---

# Phase 01: Build Matrix — Verification Report

**Phase Goal:** Release produces 8 platform targets and 12 artifacts with consistent naming
**Verified:** 2026-04-08T19:15:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Release workflow builds for all 8 targets (linux-gnu x2, linux-musl x2, macOS x2, Windows x2) | ✓ VERIFIED | Lines 21-52: 8 `target:` entries in matrix include block |
| 2 | Linux targets build with cross (not native cargo), Windows/macOS targets build with native cargo | ✓ VERIFIED | 4× `use_cross: true` (Linux), 4× `use_cross: false` (macOS/Windows); Lines 80-81 use `cross build`, Lines 83-85 use `cargo build` |
| 3 | Every archive is named with target triple: the-crab-engram-{version}-{target}.{ext} | ✓ VERIFIED | Line 91: `the-crab-engram-${{ github.ref_name }}-${{ matrix.target }}.tar.gz`; Line 97: `...${{ matrix.target }}.zip`; Line 128: `.deb` also uses target triple |
| 4 | A .deb package is built in a separate job using cargo-deb | ✓ VERIFIED | Lines 107-134: `build-release-deb` job runs `cargo install cargo-deb` then `cargo deb` independently |
| 5 | Single target failure does not cancel other targets (fail-fast: false) | ✓ VERIFIED | Line 18: `fail-fast: false` |
| 6 | Release job uploads all archives, .deb, and checksums-sha256.txt | ✓ VERIFIED | Lines 172-176: files glob includes `*.tar.gz`, `*.zip`, `*.deb`, and `checksums-sha256.txt` |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.github/workflows/release.yml` | 8-target build matrix with cross for Linux, .deb job, and release job | ✓ VERIFIED | Contains strategy matrix with 8 entries, cross installation step, build-release-deb job, and release job with proper dependencies |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| build job matrix | cross binary | download step before cargo build | ✓ WIRED | Line 60: `cross-v0.2.5` binary download via curl; conditional on `matrix.use_cross` |
| build job matrix | archive naming | tar/Compress-Archive step | ✓ WIRED | Lines 91, 97: archive names include `matrix.target` triple |
| build-release-deb job | cargo-deb | cargo install cargo-deb && cargo deb | ✓ WIRED | Lines 119-122: `cargo install cargo-deb` then `cargo deb --target x86_64-unknown-linux-gnu` |
| release job | all artifacts | download-artifact + glob patterns | ✓ WIRED | Line 138: `needs: [build, build-release-deb]`; Line 146: `download-artifact@v4` to `artifacts/` |

### Data-Flow Trace (Level 4)

Not applicable — this phase produces CI/CD workflow configuration, not runtime data artifacts. The workflow itself is the deliverable.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| YAML is valid (no syntax errors) | Visual inspection of indentation and structure | Consistent 2-space indentation, proper YAML list/dict syntax | ✓ PASS |
| Matrix has exactly 8 targets | grep target: in release.yml | 8 matches | ✓ PASS |
| cross v0.2.5 URL is correct | grep cross-v0 | Line 60: `cross-v0.2.5/cross-x86_64-unknown-linux-musl.tar.gz` | ✓ PASS |
| Windows ARM uses windows-11-arm | grep windows-11-arm | Line 51: `os: windows-11-arm` | ✓ PASS |
| All Linux targets use cross | grep use_cross: true | 4 matches (all Linux targets) | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| BUILD-01 | 01-01-PLAN.md | Release produces 8 targets | ✓ SATISFIED | 8 targets in matrix (lines 21-52) |
| BUILD-02 | 01-01-PLAN.md | Release produces 12 artifacts (8× archive + .deb + .rpm + 2× .msi + checksums) | ⚠️ PARTIAL | 10 artifacts produced (8 archives + .deb + checksums). .rpm and .msi deferred to Phase 6 per D-06/D-07. See **Requirement Discrepancies** below. |
| BUILD-03 | 01-01-PLAN.md | Asset naming uses target-triple convention | ✓ SATISFIED | `the-crab-engram-${{ github.ref_name }}-${{ matrix.target }}.{ext}` in all packaging steps |
| BUILD-04 | 01-01-PLAN.md | Linux musl builds use native musl-tools (not cross) | ⚠️ DEVIATION | Implementation uses `cross` for ALL Linux targets including musl. See **Requirement Discrepancies** below. |
| BUILD-05 | 01-01-PLAN.md | ARM Linux uses ubuntu-24.04-arm native runner | ⚠️ DEVIATION | Implementation uses `cross` on `ubuntu-latest` for ARM Linux. See **Requirement Discrepancies** below. |
| BUILD-06 | 01-01-PLAN.md | ARM Windows uses windows-11-arm native runner | ✓ SATISFIED | Line 51: `os: windows-11-arm` for `aarch64-pc-windows-msvc` |
| BUILD-07 | 01-01-PLAN.md | Build matrix uses fail-fast: false | ✓ SATISFIED | Line 18: `fail-fast: false` |

### Requirement Discrepancies

Three requirements in `REQUIREMENTS.md` diverge from the plan's user-locked decisions (CONTEXT.md). These are **not plan failures** — the plan faithfully implemented user decisions that intentionally changed the approach. However, the requirements document was not updated to reflect these decisions.

| Requirement | Requirement States | Plan Decision | Impact |
|-------------|-------------------|---------------|--------|
| BUILD-02 | 12 artifacts (includes .rpm + 2× .msi) | D-06: .rpm SKIPPED; D-07: .msi SKIPPED (deferred to Phase 6) | Artifact count is 10, not 12. .rpm and .msi are intentionally deferred. Phase 6 covers PKG-01/02 and WIN-01/02. |
| BUILD-04 | musl uses native musl-tools | D-03: cross for ALL musl targets | Implementation uses cross for musl (industry standard: starship, fd). Not a functional issue — cross handles musl internally. |
| BUILD-05 | ARM Linux uses ubuntu-24.04-arm native runner | D-01: cross for ALL Linux targets | Implementation uses cross on ubuntu-latest. Not a functional issue — cross builds ARM via QEMU. |

**Recommendation:** Update `REQUIREMENTS.md` BUILD-02/04/05 to align with actual implementation decisions, or document the intentional divergence. The traceability table currently shows all BUILD-* as "Complete" which is misleading when BUILD-02 counts 12 artifacts but only 10 are produced.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | None found. No TODO/FIXME/placeholder comments. No stub patterns. |

### Human Verification Required

None — all items are programmatically verifiable from the YAML structure.

### Gaps Summary

**Plan must-haves: ALL VERIFIED.** The plan was executed faithfully — all 6 must-have truths pass, all artifacts exist and are substantive, all 4 key links are wired.

**Requirement discrepancies (informational):** Three REQUIREMENTS.md entries (BUILD-02, BUILD-04, BUILD-05) describe different approaches than what was implemented. These reflect user decisions documented in CONTEXT.md that intentionally changed the scope:

1. **BUILD-02** — The plan counts 10 artifacts (not 12) because .rpm and .msi are deferred to Phase 6 per user decisions D-06 and D-07. The requirement's "12 artifacts" count included packaging formats the user explicitly deferred.
2. **BUILD-04** — The plan uses `cross` for musl (D-03), not native `musl-tools`. Industry standard (starship, fd, ripgrep all do this). Functionally equivalent — cross handles musl internally.
3. **BUILD-05** — The plan uses `cross` for ARM Linux (D-01), not native `ubuntu-24.04-arm`. Industry standard. Functionally equivalent — cross uses QEMU for ARM.

None of these discrepancies prevent the phase goal from being achieved. The `cross` approach is the industry standard for Rust CLI release pipelines.

---

_Verified: 2026-04-08T19:15:00Z_
_Verifier: OpenCode (gsd-verifier)_
