# Phase 3: Version Transparency - Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Enhance the `version` command to show version, git commit hash, commit date, and target triple. Create `build.rs` to embed git metadata at compile time. Add update hint to version output. No external dependencies — pure build-time metadata injection.

</domain>

<decisions>
## Implementation Decisions

### Version Output Format
- **D-01:** Simple line-per-field format (like `rustc --version --verbose`). One line per datum, readable and parseable.
  ```
  the-crab-engram 2.0.0 (abc1234 2026-04-11)
  target: x86_64-unknown-linux-gnu
  update: run `the-crab-engram self update` to check for updates
  ```

### Version Source
- **D-02:** Use `env!("CARGO_PKG_VERSION")` instead of hardcoded string. Single source of truth in `Cargo.toml`. No more manual "v2.0.0" strings scattered in code.

### Update Hint Location
- **D-03:** Update hint appears in BOTH `version` and `self version` commands. `version` shows it once at the end. `self version` is the "control panel" for self-management.

### build.rs Approach
- **D-04:** Create `build.rs` that runs `git log` to extract:
  - Short commit hash (`git rev-parse --short HEAD`)
  - Commit date (`git log -1 --format=%ci`)
  - Embed via `cargo:rustc-env=GIT_HASH=...` and `cargo:rustc-env=GIT_DATE=...`
  - Graceful fallback: if not a git repo (e.g., building from tarball), use "unknown"

### Target Triple
- **D-05:** Use `std::env::consts::ARCH` + `std::env::consts::OS` + `std::env::consts::FAMILY` at runtime, or `self_update::get_target()` which already returns the target triple string. Prefer `self_update::get_target()` since it's already a dependency and returns the canonical format.

</decisions>

<canonical_refs>
## Canonical References

### Source Code
- `src/main.rs` lines 455-459 — Current hardcoded Version handler
- `src/main.rs` line 166 — `SelfAction::Version` variant (Phase 2)
- `Cargo.toml` line 24 — `version = "2.0.0"` (single source of truth)

### Requirements
- `.planning/REQUIREMENTS.md` §Version Transparency — VERSION-01, VERSION-02, VERSION-03

</canonical_refs>

<code_context>
## Existing Code Insights

### Current Version Command
```rust
Commands::Version => {
    println!("The Crab Engram v2.0.0");
    println!("Persistent memory for AI coding agents");
    println!("https://github.com/maisonnat/the-crab-engram");
}
```

### Self Version (Phase 2)
```rust
SelfAction::Version => {
    println!("The Crab Engram v{}", env!("CARGO_PKG_VERSION"));
    println!("https://github.com/{UPDATE_REPO_OWNER}/{UPDATE_REPO_NAME}");
}
```

### Files to Create
- `build.rs` — new file at project root (Cargo auto-detects it)

### Files to Modify
- `src/main.rs` — update `Commands::Version` and `SelfAction::Version` handlers

</code_context>

<specifics>
## Specific Ideas

### build.rs
```rust
fn main() {
    // Git commit hash
    let hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    // Git commit date
    let date = std::process::Command::new("git")
        .args(["log", "-1", "--format=%ci"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    println!("cargo:rustc-env=GIT_HASH={}", hash);
    println!("cargo:rustc-env=GIT_DATE={}", date);
}
```

### Enhanced Version Output
```rust
Commands::Version => {
    let version = env!("CARGO_PKG_VERSION");
    let hash = env!("GIT_HASH");
    let date = env!("GIT_DATE");
    let target = self_update::get_target();
    
    eprintln!("the-crab-engram {version} ({hash} {date})");
    eprintln!("target: {target}");
    eprintln!("update: run `the-crab-engram self update` to check for updates");
}
```

### Updated Self Version
```rust
SelfAction::Version => {
    let version = env!("CARGO_PKG_VERSION");
    let hash = env!("GIT_HASH");
    let date = env!("GIT_DATE");
    
    eprintln!("the-crab-engram {version} ({hash} {date})");
    eprintln!("https://github.com/{UPDATE_REPO_OWNER}/{UPDATE_REPO_NAME}");
    eprintln!("update: run `the-crab-engram self update` to check for updates");
}
```

</specifics>

<deferred>
## Deferred Ideas

- Semantic version parsing for update comparisons — self_update handles this
- JSON output mode for version (`--json`) — not needed yet
- Build timestamp (in addition to commit date) — adds noise without value

</deferred>

---

*Phase: 03-version-transparency*
*Context gathered: 2026-04-11*
