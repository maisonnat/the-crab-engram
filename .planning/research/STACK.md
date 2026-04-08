# Stack Research: Rust CLI Distribution & Packaging

**Domain:** Rust CLI cross-platform distribution, self-update, native packaging
**Researched:** 2026-04-08
**Confidence:** HIGH

## Industry Audit — What Do Starship, ripgrep, fd, and uv Do?

| Tool | Self-Update | Targets | Packaging Channels | Distribution Pattern |
|------|-------------|---------|-------------------|---------------------|
| **starship** | None (manual only) | 13 (linux-gnu, linux-musl x5, macOS x2, Windows x3, FreeBSD) | Homebrew (core), winget, Chocolatey, MSI, Snap, pkg (macOS) | cargo-wix for MSI, wingetcreate for winget, `mislav/bump-homebrew-formula-action` for Homebrew, notarized macOS PKG |
| **ripgrep** | None | 13 (linux-gnu x2, linux-musl, macOS x2, Windows x3, cross-compile arm/s390x) | Homebrew (core), cargo-deb for .deb | `cargo-deb` for .deb packages, Homebrew auto-bump via formula action, SHA256 checksums on all assets |
| **fd** | None | 14 (linux-gnu x3, linux-musl x3, macOS x2, Windows x3, arm Linux) | Homebrew (core), cargo-deb for .deb, winget | `cross` for cross-compilation, custom `scripts/create-deb.sh`, `winget-releaser` for winget |
| **uv** | None (curl \| sh) | 8+ (linux-gnu x2, linux-musl x2, macOS x2, Windows x2) | curl \| sh, Homebrew, PyPI (via uv-installer) | Custom install scripts (not a Rust binary pattern — ships Python installer too) |

**Key insight:** NONE of these tools ship built-in self-update. They all rely on package managers to handle updates. The `the-crab-engram` approach of built-in self-update is a differentiator, not standard practice.

## Recommended Stack

### Core Technologies

#### Self-Update: `self_update` 0.44.0

| Property | Value |
|----------|-------|
| **Version** | 0.44.0 (released 2026-04-05) |
| **Downloads** | 7.9M total, 1.3M recent |
| **Repository** | `jaemk/self_update` |
| **Confidence** | HIGH — verified via crates.io API |

**Why `self_update` and NOT `axoupdater` (0.10.0) or `cargo-dist` (0.31.0):**

| Criterion | `self_update` 0.44 | `axoupdater` 0.10 | `cargo-dist` 0.31 |
|-----------|--------------------|--------------------|---------------------|
| **Coupling** | None — works with any GitHub Release | Tied to cargo-dist install receipts | Requires replacing entire CI/release workflow |
| **Asset naming** | Flexible — any naming convention | Requires cargo-dist receipt JSON | Own installer system |
| **Rustls support** | `rustls` feature flag | Via `axoasset/tls-native-roots` | Via `axoasset/tls-native-roots` |
| **Signature verification** | `signatures` feature (zipsign) | No | No |
| **Archive support** | tar.gz, zip (with compression) | Limited | Limited |
| **Windows binary replace** | `self_replace` crate (proven) | Unknown | Via installer |
| **No-openssl builds** | `default-features = false, features = ["rustls"]` | Requires native-tls override | Default OpenSSL |

**The project already chose `self_update` v0.27 in PROJECT.md — that version is 17 releases behind. Use v0.44.0.**

**Dependency configuration:**
```toml
# Cargo.toml [dependencies]
self_update = { version = "0.44.0", features = ["archive-tar", "archive-zip", "compression-flate2", "compression-zip-deflate", "rustls"], default-features = false }
```

The `rustls` feature is critical — it avoids linking against system OpenSSL, which preserves the zero-dependency property on musl builds.

#### Packaging: `cargo-deb` 3.6.3

| Property | Value |
|----------|-------|
| **Version** | 3.6.3 (released 2026-02-04) |
| **Downloads** | 1.8M total, 139K recent |
| **Repository** | `kornelski/cargo-deb` |
| **Confidence** | HIGH — verified via crates.io API |

**Used by:** ripgrep, fd, bat, delta, and virtually every Rust CLI that ships .deb packages.

**Why cargo-deb:** It's the only tool. There's no serious alternative. The `[package.metadata.deb]` section in `Cargo.toml` is the de facto standard for Rust → Debian packaging.

#### Packaging: `cargo-generate-rpm` 0.20.0

| Property | Value |
|----------|-------|
| **Version** | 0.20.0 (released 2025-12-06) |
| **Downloads** | 220K total, 37K recent |
| **Repository** | `cat-in-136/cargo-generate-rpm` |
| **Confidence** | HIGH — verified via crates.io API |

**Used by:** fd, and other Rust CLIs that ship RPM packages. Lower download numbers than cargo-deb because RPM distribution is less common in the Rust ecosystem.

#### MSI Installer: `cargo-wix` 0.3.9

| Property | Value |
|----------|-------|
| **Version** | 0.3.9 (released 2025-03-13) |
| **Downloads** | 293K total, 37K recent |
| **Repository** | `volks73/cargo-wix` |
| **Confidence** | HIGH — verified via crates.io API |

**Used by:** starship (pins v0.3.8), ripgrep. The standard for WiX-based MSI generation from Rust.

**Note:** cargo-wix requires WiX Toolset v3 installed on the Windows CI runner. Starship installs it via `cargo install --version 0.3.8 cargo-wix`. Use v0.3.9 (latest).

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `self_replace` | 1.x | Re-extracted from self_update — used internally | self_update handles this automatically; don't add separately |
| `semver` | 1.x | Version comparison for update checks | If comparing versions outside self_update |

### Development Tools (CI)

| Tool | Purpose | Notes |
|------|---------|-------|
| `cross` v0.2.5 | Cross-compilation for Linux targets | Starship, ripgrep, fd all use it for Linux builds |
| `taiki-e/install-action` | Install cargo tools in CI | Faster than `cargo install`, uses prebuilt binaries |
| `signpath` (optional) | Code signing on Windows | Starship uses it for MSI signing; not required at v2.0 |

## Build Matrix — The 8 Targets

Based on what starship/ripgrep/fd ship, the 8-target matrix for `the-crab-engram`:

| Target | OS Runner | Archive Format | Package |
|--------|-----------|---------------|---------|
| `x86_64-unknown-linux-gnu` | ubuntu-latest | tar.gz | .deb |
| `x86_64-unknown-linux-musl` | ubuntu-latest | tar.gz | — |
| `aarch64-unknown-linux-gnu` | ubuntu-latest (cross) | tar.gz | — |
| `aarch64-unknown-linux-musl` | ubuntu-latest (cross) | tar.gz | — |
| `x86_64-apple-darwin` | macos-latest | tar.gz | — |
| `aarch64-apple-darwin` | macos-latest | tar.gz | — |
| `x86_64-pc-windows-msvc` | windows-latest | zip | .msi |
| `aarch64-pc-windows-msvc` | windows-latest | zip | — |

**Why ARM Linux is included despite PROJECT.md saying "deferred":** The PROJECT.md says ".deb/.rpm on ARM deferred" — the ARM tar.gz archives are a separate concern. Cross-compilation via `cross` is trivial for tar.gz. The .deb/.rpm for ARM is the deferred part.

**Why musl alongside gnu:** Musl builds are fully static — zero system dependencies. The install script should prefer musl for Linux (like starship does) because it works everywhere without glibc version issues.

## Asset Naming Convention

The standard pattern across all Rust CLI tools:

```
{binary}-{version}-{target-triple}.tar.gz    (Linux/macOS)
{binary}-{version}-{target-triple}.zip        (Windows)
{binary}-{version}-{target-triple}.deb        (Debian)
{binary}-{version}-{target-triple}.msi        (Windows MSI)
{binary}-{version}-{target-triple}.tar.gz.sha256  (checksums)
```

**For self_update:** The GitHub asset name must contain the target triple. self_update uses `asset_for(&self_update::get_target(), None)` which matches assets containing the target triple string.

## Homebrew Distribution

**Custom tap pattern** (what starship does before reaching Homebrew core):

```yaml
# .github/workflows/release.yml
update_brew_formula:
  runs-on: ubuntu-latest
  steps:
    - uses: mislav/bump-homebrew-formula-action@v4.1
      with:
        formula-name: the-crab-engram
        tag-name: ${{ needs.release_please.outputs.tag_name }}
```

This auto-creates a PR to your tap repo (`USERNAME/homebrew-tap`) with the updated formula on each release.

## Winget Distribution

**Pattern from fd/starship:**

```yaml
winget_update:
  runs-on: windows-latest
  steps:
    - uses: vedantmgoyal9/winget-releaser@v2
      with:
        identifier: your-identifier-here
        installers-regex: '-pc-windows-msvc\.zip$'
        token: ${{ secrets.WINGET_TOKEN }}
```

## Scoop Distribution

**Manual bucket or community submission.** No auto-updater action exists (unlike Homebrew/winget). Options:
1. Submit to `scoop-extras` (community-maintained)
2. Create own bucket repo and update via CI
3. Defer — Homebrew + winget cover most users

## Install Script Pattern

The `curl | sh` one-liner pattern from starship:

```bash
curl -sS https://starship.sh/install.sh | sh
```

The install script should:
1. Detect OS + arch → download matching tar.gz from GitHub Releases
2. Extract binary to `/usr/local/bin` (or ask)
3. Prefer musl over gnu on Linux (more portable)
4. Verify SHA256 checksum if available
5. Print success message

## Version Strategy

**Cargo.toml version** drives everything. Use `clap`'s built-in version extraction:

```rust
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    // ...
}
```

For enhanced version output (commit hash, target triple), use `vergen` or `built` crate at build time — but this is optional polish, not blocking.

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `self_update` 0.44 | `axoupdater` 0.10 | If you adopt cargo-dist for CI generation |
| `cross` for Linux ARM | GitHub native ARM runners | Native ARM runners are faster but currently only for aarch64-unknown-linux-gnu |
| `cargo-deb` | Manual `dpkg-deb` script | Only if cargo-deb's `[package.metadata.deb]` config isn't sufficient |
| Custom install script | `eget` / `fetch` tools | If you don't want to maintain your own script |
| Manual CI matrix | `cargo-dist` CI generation | If starting from scratch with no existing release workflow |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `cargo-dist` 0.31 | Requires replacing entire release workflow. Generates its own installer system. 142K downloads (low adoption). Coupled to `axoupdater` via install receipts. | `self_update` + manual CI matrix |
| `axoupdater` 0.10 | Tied to cargo-dist receipts. Can't work with arbitrary GitHub Release assets. | `self_update` — works with any release |
| `cross-rs` cross-compilation for macOS/Windows | Only works for Linux targets. Overkill for native platforms. | Native runners (macos-latest, windows-latest) |
| `cargo-bundle` | Abandoned, doesn't support modern MSI/PKG workflows. | `cargo-wix` for MSI |
| `cargo-rpm` | Less maintained than `cargo-generate-rpm`. | `cargo-generate-rpm` 0.20 |

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `self_update` 0.44 | reqwest with rustls | Use `default-features = false, features = ["rustls"]` to avoid OpenSSL |
| `cargo-wix` 0.3.9 | WiX Toolset v3 | Requires `wix311-binaries` or WiX v3 installed on runner |
| `cargo-deb` 3.6.3 | Rust 1.85+ | Uses Edition 2024 |
| `self_update` + `self_replace` | Windows only | self_replace handles the rename→.old→copy pattern on Windows |

## CI Workflow Pattern

Based on analysis of starship, ripgrep, and fd release workflows:

1. **Trigger:** Tag push (`v*.*.*`) or release-please
2. **Build job:** Matrix build across 8 targets, using `cross` for Linux, native for macOS/Windows
3. **Package jobs:** Parallel — `cargo-deb` on Linux runner, `cargo-wix` on Windows runner
4. **Upload job:** Collect all artifacts, generate SHA256 checksums, attach to GitHub Release
5. **Homebrew job:** `mislav/bump-homebrew-formula-action` PR to custom tap
6. **Winget job:** `winget-releaser` PR to microsoft/winget-pkgs

**Starship uses `googleapis/release-please-action@v4` for release management.** Consider this for automated changelogs.

## Sources

- crates.io API — self_update v0.44.0 (2026-04-05), cargo-deb v3.6.3 (2026-02-04), cargo-generate-rpm v0.20.0 (2025-12-06), cargo-wix v0.3.9 (2025-03-13), axoupdater v0.10.0 (2026-02-20), cargo-dist v0.31.0 (2026-02-23)
- `github.com/starship/starship/.github/workflows/release.yml` — 13-target matrix, cargo-wix v0.3.8 for MSI, `signpath` for signing, `mislav/bump-homebrew-formula-action` for Homebrew, `wingetcreate` for winget
- `github.com/BurntSushi/ripgrep/.github/workflows/release.yml` — 13-target matrix, `cross` v0.2.5 for cross-compilation, `cargo-deb` for .deb
- `github.com/sharkdp/fd/.github/workflows/CICD.yml` — 14-target matrix, `scripts/create-deb.sh` wrapper, `winget-releaser@v2` for winget, `cross` v0.2.5
- `github.com/jaemk/self_update` — GitHub backend source, asset matching logic, `self_replace` re-export

---

*Stack research for: Rust CLI distribution & packaging*
*Researched: 2026-04-08*
