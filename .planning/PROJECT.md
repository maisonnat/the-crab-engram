# The Crab Engram 🦀

## What This Is

The Crab Engram is a local-first persistent memory system for AI coding agents. It stores observations, builds a knowledge graph, and provides smart context injection via MCP, HTTP, and TUI. Ships as a single Rust binary with bundled SQLite — zero external dependencies at runtime.

**This phase:** Add zero-friction self-updates and zero-data-loss backup/restore across Windows, macOS, and Linux, plus native package manager distribution on every platform.

## Core Value

The agent's brain must never be lost. All update, distribution, and packaging work must preserve data safety as the non-negotiable priority.

## Requirements

### Validated

- ✓ MCP server with 31+ tools (mem_save, mem_search, mem_inject, etc.) — existing
- ✓ SQLite persistence with WAL mode, bundled — existing
- ✓ Hybrid search (FTS5 + vector cosine via Reciprocal Rank Fusion) — existing
- ✓ Knowledge graph with typed edges (CausedBy, RelatedTo, Supersedes, Blocks, PartOf) — existing
- ✓ Auto-consolidation pipeline (merge duplicates, mark obsolete, find contradictions, extract patterns) — existing
- ✓ HTTP REST API via Axum on port 7437 — existing
- ✓ TUI dashboard (ratatui/crossterm) — existing
- ✓ Cross-device sync via CRDT chunks — existing
- ✓ Optional ChaCha20Poly1305 database encryption — existing
- ✓ CLI with 18 subcommands (mcp, search, save, export, import, serve, tui, etc.) — existing
- ✓ Cargo workspace with 8 crates (core, store, search, learn, mcp, api, tui, sync) — existing
- ✓ Release workflow producing 3 targets (linux-gnu, macos-arm64, windows-msvc) — existing

### Active

- [ ] **SELF-UPDATE-01**: User can self-update binary with `the-crab-engram update`
- [ ] **SELF-UPDATE-02**: User can check for updates without downloading with `--check-only`
- [ ] **BUILD-MATRIX-01**: Release produces 8 targets (linux-gnu x2, linux-musl x2, macos x2, windows x2)
- [ ] **BUILD-MATRIX-02**: Release produces 12 artifacts (tar.gz, zip, .deb, .rpm, .msi)
- [ ] **VERSION-01**: User can see version, commit hash, target triple via enhanced `version` command
- [ ] **BACKUP-01**: User can create manual backups with `the-crab-engram backup`
- [ ] **BACKUP-02**: System creates automatic pre-update backups before self-update
- [ ] **BACKUP-03**: System creates automatic pre-migration backups before schema migrations
- [ ] **RESTORE-01**: User can list and restore backups with `the-crab-engram restore`
- [ ] **RESTORE-02**: Restore verifies backup integrity before applying
- [ ] **PACKAGING-01**: Users can install via `apt install` (.deb packages)
- [ ] **PACKAGING-02**: Users can install via `rpm -i` (.rpm packages)
- [ ] **PACKAGING-03**: Users can install via `brew install` (Homebrew tap)
- [ ] **PACKAGING-04**: Users can install via `winget install` (winget-pkgs)
- [ ] **PACKAGING-05**: Users can install via `scoop install` (Scoop bucket)
- [ ] **PACKAGING-06**: Users can install via `curl | sh` one-liner (install scripts)
- [ ] **PACKAGING-07**: Users can install via Windows MSI installer
- [ ] **UPDATE-CHECK-01**: System checks for updates in background (once per 24h, stderr only)
- [ ] **MUSL-01**: Linux builds include musl variants for zero-dependency portability

### Out of Scope

- cargo-dist adoption — requires replacing entire release workflow, unnecessary with self_update
- Cross-compilation via cross/cargo-zigbuild — native ARM runners available since Jan 2026
- Homebrew core submission — too early for v2.0.0, custom tap sufficient
- ARM Linux packaging (.deb/.rpm) — deferred, x86_64 only for now
- GUI installer beyond MSI — no Electron/web installer needed

## Context

- **Ecosystem**: Rust CLI tool, competes with uv/starship/ripgrep distribution patterns
- **Current state**: Working single-binary with 3-target release pipeline, 8 crates, SQLite + FTS5 + vector search
- **Key constraint**: MCP uses stdio for JSON-RPC — stdout is sacred, all update check output must go to stderr
- **Key constraint**: Windows cannot overwrite a running binary — self_update handles via rename→.old→copy pattern
- **Industry benchmark**: starship ships brew/winget/scoop + MSI; no major Rust CLI has automatic backup — differentiation opportunity

## Constraints

- **Zero system deps**: rusqlite/bundled + rustls — musl builds are fully static, must not break this
- **Stdio sacred**: MCP transport uses stdout for JSON-RPC — no user-facing output on stdout during mcp/serve
- **Single binary**: All features compiled in, no optional feature flags for distribution
- **SQLite backup API**: Use `rusqlite::backup::Backup::run_to_completion()` — non-blocking, works while MCP server is active

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| `self_update` v0.44.0 (not `axoupdater`) | No cargo-dist coupling, works with any GitHub Release, rustls. Master plan cited v0.27 — 17 releases behind. | — Pending |
| Native ARM runners (not cross/QEMU) | GitHub ARM runners GA since Jan 2026, faster, simpler CI | — Pending |
| Custom Homebrew tap (not core) | Faster release velocity at v2.0.0, can migrate later | — Pending |
| `cargo-deb` + `cargo-generate-rpm` | De facto standard (ripgrep, fd, bat use these) | — Pending |
| `rusqlite::backup::Backup` for backups | Official SQLite online backup API, non-blocking | — Pending |
| Target triple asset naming | Industry standard, self_update works out of the box | — Pending |
| musl builds alongside gnu | Zero-dependency Linux, install script prefers musl | — Pending |
| 10 auto-backup rotation limit | ~150MB disk for typical databases, manual backups never deleted | — Pending |
| Backup as Storage trait methods | No new crate, backup is a storage operation | — Pending |

---

*Last updated: 2026-04-08 after initialization*

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state
