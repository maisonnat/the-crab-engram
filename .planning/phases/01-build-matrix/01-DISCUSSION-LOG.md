# Phase 1: Build Matrix - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-08
**Phase:** 01-build-matrix
**Areas discussed:** ARM runner strategy, musl build approach, archive naming, .deb/.rpm placement, CI coverage

---

## ARM Runner Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Native ARM runners | `ubuntu-24.04-arm` / `windows-11-arm` per requirements | |
| `cross` for Linux, native for Windows | `cross` for all Linux targets, `windows-11-arm` for Windows ARM | ✓ |
| `cross` for everything | `cross` for all targets including Windows (not possible — cross doesn't support Windows) | |

**User's choice:** Approved all recommendations (cross for Linux, native for Windows ARM)
**Notes:** Research showed starship, ripgrep, fd all use cross for Linux ARM. Nobody uses native Linux ARM runners.

---

## musl Build Approach

| Option | Description | Selected |
|--------|-------------|----------|
| Native musl-tools per requirements | Install musl-tools on each runner, build natively | |
| `cross` for all musl targets | cross handles musl internally, no manual install | ✓ |

**User's choice:** Approved recommendation — cross for all musl
**Notes:** Only ripgrep does native musl x86_64. starship and fd use cross for everything. Simpler to stay consistent.

---

## Archive Naming Convention

| Option | Description | Selected |
|--------|-------------|----------|
| Descriptive (current) | `the-crab-engram-linux-x86_64.tar.gz` | |
| Target-triple | `the-crab-engram-{version}-{target}.tar.gz` | ✓ |

**User's choice:** Approved target-triple naming
**Notes:** Required by self_update. All three reference projects use full target triple.

---

## .deb / .rpm Placement

| Option | Description | Selected |
|--------|-------------|----------|
| Inline in build matrix | Generate .deb/.rpm as steps in the build job | |
| Separate job | Dedicated `build-release-deb` job (ripgrep pattern) | ✓ |
| Skip entirely | No .deb/.rpm in this phase | |

**User's choice:** Approved separate job for .deb, skip .rpm
**Notes:** Nobody in the Rust ecosystem generates .rpm. ripgrep uses separate .deb job. .deb and .msi deferred/packaged accordingly.

---

## CI Coverage Expansion

| Option | Description | Selected |
|--------|-------------|----------|
| Keep 3 OS targets | Current CI stays as-is | ✓ |
| Expand to musl/ARM | Add musl and ARM targets to CI matrix | |

**User's choice:** Keep 3 OS targets — no CI expansion
**Notes:** Release builds with cross provide sufficient coverage.

---

## OpenCode's Discretion

- `cross` exact version pin
- .deb metadata defaults
- Checksum generation method

## Deferred Ideas

- .rpm package — deferred (no ecosystem precedent)
- .msi installer — deferred to Phase 6 (Packaging)
- Native ARM runners for Linux — rejected in favor of cross
- CI musl/ARM expansion — not needed
