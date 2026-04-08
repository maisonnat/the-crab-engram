# Phase 2: Self-Update Engine - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Add in-binary self-update via `the-crab-engram self update` command. Uses `self_update` v0.44.0 crate with `rustls` to download the latest release from GitHub, verify checksums, and atomically replace the binary. NO database interaction — only the executable is touched.

</domain>

<decisions>
## Implementation Decisions

### CLI Structure
- **D-01:** `self` namespace (not top-level). `the-crab-engram self update` and `the-crab-engram self version`. Follows uv's pattern. Groups self-management commands logically without polluting top-level space.

### Repo Config
- **D-02:** Hardcoded constants for owner/repo. `const UPDATE_REPO_OWNER: &str = "maisonnat"`, `const UPDATE_REPO_NAME: &str = "the-crab-engram"`. Environment variable override `CRAB_ENGRAM_UPDATE_REPO=owner/repo` for dev/testing. No install receipt system (not needed without custom installer).

### Binary Backup
- **D-03:** NO persistent binary backup. `self_replace` crate handles atomic replacement safely:
  - Unix: atomic `rename()` — filesystem guarantees consistency
  - Windows: move aside → copy → spawn cleanup — old binary is moved, not deleted
  - Replaces requirement UPDATE-04 ("automatic backup before replacing")
  - Safety comes from `--dry-run` flag + post-update size verification (UPDATE-05)

### Confirmation UX
- **D-04:** NO interactive prompt. If user ran `the-crab-engram self update`, they expressed intent. Modern CLI convention (uv pattern). `--dry-run` flag for cautious users shows what would happen without downloading.

### Error Handling
- **D-05:** Colored, structured error messages with actionable recovery hints (uv pattern):
  - Network timeout → "Check your internet connection and try again"
  - HTTP 403 → "Set GITHUB_TOKEN env var for higher rate limits"
  - HTTP 404 → "No release found for your platform"
  - Checksum mismatch → "Download failed integrity check. Try again or download manually"
  - Already latest → success message, not error
  - All output to stderr (stdout sacred for MCP)

### Database Safety (CRITICAL)
- **D-06:** Self-update MUST NEVER touch the SQLite database. The update process only downloads, extracts, and replaces the binary executable. No reads, writes, or access to any files in the data directory (`~/.engram/`). Database lives at separate filesystem location — completely isolated from update process.

### self_update Configuration
- **D-07:** Use `self_update` v0.44.0 with these features:
  ```toml
  self_update = { version = "0.44.0", features = ["archive-tar", "archive-zip", "compression-flate2", "compression-zip-deflate", "rustls"], default-features = false }
  ```
  - `rustls` avoids OpenSSL dependency (critical for musl builds)
  - Archive features for tar.gz (Unix) and zip (Windows) extraction
  - Version from `cargo_crate_version!()` macro

### Target Matrix

| Aspect | Decision |
|--------|----------|
| CLI command | `the-crab-engram self update [--check-only] [--dry-run]` |
| Repo owner | `maisonnat` (hardcoded) |
| Repo name | `the-crab-engram` (hardcoded) |
| Version detection | `cargo_crate_version!()` from Cargo.toml |
| Asset matching | Target-triple in archive name (Phase 1 naming) |
| Binary replace | `self_replace` (atomic, cross-platform) |
| Checksum | SHA-256 against `checksums-sha256.txt` |
| Output | stderr only |

### Requirements Modified
- **UPDATE-04** changed: from "automatic backup before replacing" to "update uses --dry-run flag for preview without downloading" (aligned with industry standard, atomic replace is sufficient for safety)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Source Code
- `src/main.rs` — Current CLI structure (Commands enum, clap derive). Add `Self { action: SelfAction }` variant.
- `Cargo.toml` — Add `self_update` dependency.

### Research
- `.planning/research/SELF-UPDATE.md` — Comparative research of uv, starship, self_update patterns
- `.planning/research/STACK.md` §Self-Update — `self_update` v0.44.0 dependency config
- `.planning/research/PITFALLS.md` §Pitfall 2 — Windows 0-byte executable issue and mitigation
- `.planning/research/ARCHITECTURE.md` §Self-Update Flow — exact flow diagram

### Requirements
- `.planning/REQUIREMENTS.md` §Self-Update — UPDATE-01 through UPDATE-06

### Project Constraints
- `.planning/PROJECT.md` §Constraints — stdout sacred, zero system deps
- `.planning/phases/01-build-matrix/01-CONTEXT.md` — Archive naming (target-triple) that self_update expects

</canonical_refs>

<code_context>
## Existing Code Insights

### CLI Structure (src/main.rs)
- Uses `#[derive(Subcommand)]` on `Commands` enum
- Current commands: Mcp, Search, Save, Context, Stats, Timeline, Export, Import, ExportContext, SessionStart, SessionEnd, Version, Serve, Tui, Consolidate, Sync, Encrypt, Setup
- Version command is simple: prints version string + GitHub URL
- Pattern: each variant maps to handler in `match` block

### Reusable Assets
- `Cargo.toml` workspace structure — single `[package]` with `[[bin]]`
- `self_update::backends::github` — builder pattern for GitHub releases
- `self_update::get_target()` — returns current target triple string
- `cargo_crate_version!()` macro — pulls version from Cargo.toml

### Integration Points
- Phase 1 archive naming: `the-crab-engram-{version}-{target}.{ext}` — self_update matches against this
- `checksums-sha256.txt` generated in release job — self_update can verify against this
- stderr output — existing commands use `eprintln!` for user-facing messages

</code_context>

<specifics>
## Specific Ideas

### CLI Implementation
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Self-management commands
    Self {
        #[command(subcommand)]
        action: SelfAction,
    },
}

#[derive(Subcommand)]
enum SelfAction {
    /// Update to the latest version
    Update {
        /// Check for updates without downloading
        #[arg(long)]
        check_only: bool,
        /// Show what would happen without updating
        #[arg(long)]
        dry_run: bool,
    },
    /// Show current version and check for updates
    Version,
}
```

### Update Handler Flow
```rust
fn handle_self_update(check_only: bool, dry_run: bool) -> Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner(UPDATE_REPO_OWNER)
        .repo_name(UPDATE_REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(self_update::cargo_crate_version!())
        .build()?
        .update()?;
    
    match status {
        self_update::Status::UpToDate(v) => {
            eprintln!("Already on the latest version (v{v})");
        }
        self_update::Status::Updated(v) => {
            eprintln!("Updated to v{v}");
        }
    }
    Ok(())
}
```

### Post-Update Verification (Windows 0-byte fix)
```rust
// After self_update completes, verify binary integrity
let exe = std::env::current_exe()?;
let meta = std::fs::metadata(&exe)?;
if meta.len() == 0 {
    return Err("Update produced 0-byte binary. Please reinstall manually.".into());
}
```

</specifics>

<deferred>
## Deferred Ideas

- Install receipt system (like uv) — not needed without custom installer
- `--token` flag for private repos — public repo only for now
- Rollback to previous version — requires keeping old binary, add if users request
- Async update check (background) — Phase 8 territory
- Changelog display during update — `self_update` doesn't support natively
- Signature verification (zipsign) — add after initial implementation works

</deferred>

---

*Phase: 02-self-update-engine*
*Context gathered: 2026-04-08*
