# Phase 1: Build Matrix - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Expand the release CI pipeline from 3 targets / 3 artifacts to 8 targets / 12 artifacts with target-triple naming, cross-compilation via `cross`, and .deb packaging in a separate job. No code changes to the Rust binary itself — this is purely CI/CD workflow work.

</domain>

<decisions>
## Implementation Decisions

### ARM Runner Strategy
- **D-01:** Use `cross` (cross-rs) for ALL Linux targets including ARM — NOT native ARM runners (`ubuntu-24.04-arm`). Industry standard: starship, ripgrep, and fd all use `cross` for Linux ARM. Simpler CI, less maintenance.
- **D-02:** Use `windows-11-arm` native runner for Windows ARM64 (`aarch64-pc-windows-msvc`). ripgrep and fd do this — `cross` doesn't support Windows targets.

### musl Build Approach
- **D-03:** Use `cross` for ALL musl targets (x86_64 + aarch64). No manual `musl-tools` installation. `cross` handles musl internally. Follows starship and fd patterns.

### Archive Naming
- **D-04:** Target-triple format: `the-crab-engram-{version}-{target}.tar.gz` (e.g., `the-crab-engram-2.1.0-x86_64-unknown-linux-gnu.tar.gz`). Required by `self_update` crate for asset discovery. Matches starship, ripgrep, and fd conventions.

### Packaging Jobs
- **D-05:** `.deb` package in a SEPARATE job (not inline in build matrix). Pattern: ripgrep's `build-release-deb`. Uses `cargo-deb` against `x86_64-unknown-linux-gnu` target.
- **D-06:** `.rpm` package SKIPPED. No major Rust CLI tool generates .rpm (starship, ripgrep, fd — none do it). Deferred to Phase 6 (Packaging) if needed.
- **D-07:** `.msi` installer SKIPPED in this phase. Deferred to Phase 6 (Packaging) — `cargo-wix` setup is packaging work, not build matrix work.

### CI Scope
- **D-08:** CI workflow stays at 3 OS targets (ubuntu, macos, windows). No expansion to test musl/ARM in CI. Release builds with `cross` provide sufficient coverage. If it compiles, it works.

### Cross Tool Version
- **D-09:** Pin `cross` to a specific release version (download binary, not `cargo install`). ripgrep and fd both pin `cross` to avoid breakage from upstream releases.

### Target Matrix (8 targets)
| Target | Runner | Build Tool |
|--------|--------|------------|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | `cross` |
| `x86_64-unknown-linux-musl` | `ubuntu-latest` | `cross` |
| `aarch64-unknown-linux-gnu` | `ubuntu-latest` | `cross` |
| `aarch64-unknown-linux-musl` | `ubuntu-latest` | `cross` |
| `x86_64-apple-darwin` | `macos-latest` | `cargo` |
| `aarch64-apple-darwin` | `macos-latest` | `cargo` |
| `x86_64-pc-windows-msvc` | `windows-latest` | `cargo` |
| `aarch64-pc-windows-msvc` | `windows-11-arm` | `cargo` |

### Artifact Count (12 total)
- 8× archives (.tar.gz for Unix, .zip for Windows)
- 1× .deb package (linux x86_64 gnu)
- 2× .msi (placeholder, deferred to Phase 6)
- 1× checksums-sha256.txt

### OpenCode's Discretion
- `cross` exact version pin — OpenCode decides based on latest stable
- .deb metadata (description, section, priority) — OpenCode follows cargo-deb defaults
- Checksum generation method (sha256sum vs openssl) — OpenCode decides

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### CI/CD Reference Workflows
- `.github/workflows/release.yml` — Current 3-target release workflow to expand
- `.github/workflows/ci.yml` — Current CI workflow (stays as-is)

### Industry Reference (researched)
- [starship/release.yml](https://github.com/starship/starship/blob/master/.github/workflows/release.yml) — 12 targets, `cross` for all Linux, `cargo-wix` for MSI
- [ripgrep/release.yml](https://github.com/BurntSushi/ripgrep/blob/master/.github/workflows/release.yml) — `cross` for Linux ARM, separate .deb job, `windows-11-arm` for Windows ARM
- [fd/CICD.yml](https://github.com/sharkdp/fd/blob/master/.github/workflows/CICD.yml) — `cross` for all Linux, inline .deb, `windows-11-arm`

### Requirements
- `.planning/REQUIREMENTS.md` §Build Matrix — BUILD-01 through BUILD-07

### Project Constraints
- `.planning/PROJECT.md` §Constraints — zero system deps, stdio sacred, single binary

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `.github/workflows/release.yml` — Existing 3-target workflow, `fail-fast: false` already set
- `.github/workflows/ci.yml` — CI workflow, no changes needed
- `Cargo.toml` — Workspace root with `[package]` section for cargo-deb metadata

### Established Patterns
- Release triggered by `v*` tags — keep this trigger
- `softprops/action-gh-release@v2` for GitHub Release creation — keep
- SHA-256 checksum generation in release job — keep and expand
- `actions/upload-artifact@v4` + `actions/download-artifact@v4` — artifact flow pattern

### Integration Points
- `self_update` crate (Phase 2) expects `the-crab-engram-{version}-{target}.{ext}` asset naming — D-04 satisfies this
- Phase 6 (Packaging) will add MSI/winget/Homebrew on top of this matrix — D-07 defers MSI

</code_context>

<specifics>
## Specific Ideas

- Follow ripgrep's pattern: separate `build-release-deb` job that runs `cargo-deb` independently
- `cross` binary download pattern from ripgrep/fd: `curl` from GitHub releases, pin version, add to PATH
- Archive packaging: standard `tar czf` for Unix, `7z a` or `Compress-Archive` for Windows
- macOS builds use native `cargo` (no cross needed — `macos-latest` supports both x86_64 and aarch64 via Xcode)

</specifics>

<deferred>
## Deferred Ideas

- `.rpm` package generation — deferred to Phase 6 or later if user demand exists
- `.msi` installer via `cargo-wix` — deferred to Phase 6 (Packaging)
- Native ARM runners for Linux (`ubuntu-24.04-arm`) — rejected in favor of `cross`, revisit if cross proves unreliable
- CI expansion to test musl/ARM targets — not needed, release builds provide coverage
- `cargo-zigbuild` as alternative to `cross` — not needed, cross is industry standard

</deferred>

---

*Phase: 01-build-matrix*
*Context gathered: 2026-04-08*
