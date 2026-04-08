# Technology Stack

**Analysis Date:** 2026-04-08

## Languages

**Primary:**
- Rust (Edition 2024) — All application code across 8 workspace crates

**Secondary:**
- SQL — 13 embedded migration files for schema evolution
- Shell (Bash) — Build/release scripts in `scripts/`

## Runtime

**Environment:**
- Native binary (no managed runtime)

**Package Manager:**
- Cargo — Rust package manager
- Lockfile: `Cargo.lock` (present)

**Workspace Version:** 2.1.0 (defined in `Cargo.toml`)

## Frameworks

**CLI Framework:**
- `clap` 4.x (derive macros) — Command-line argument parsing
  - Configured with `derive` feature
  - Location: `src/main.rs` (Cli struct + Commands enum)

**HTTP API Framework:**
- `axum` 0.8 — REST API server
  - Paired with `tower-http` 0.6 (CORS middleware)
  - Location: `crates/api/src/lib.rs`

**MCP (Model Context Protocol) Server:**
- `rmcp` 1.3 — AI agent tool integration server
  - Features: `server`, `macros`, `transport-io`, `schemars`
  - Stdio transport for communication with AI coding agents
  - Location: `crates/mcp/src/server.rs`

**TUI (Terminal User Interface):**
- `ratatui` 0.29 — Terminal UI rendering framework
- `crossterm` 0.29 — Terminal backend (cross-platform)
  - Location: `crates/tui/src/app.rs`

**Async Runtime:**
- `tokio` 1.x (full features) — Async runtime for MCP server and HTTP API

## Key Dependencies

**Database:**
- `rusqlite` 0.35 — SQLite embedded database
  - Features: `bundled` (ships SQLite with the binary), `serde_json`
  - WAL mode, busy_timeout=5000, synchronous=NORMAL, foreign_keys=ON
  - Location: `crates/store/src/sqlite.rs`

**Search & Embeddings:**
- `fastembed` 5.x — Local embedding model (all-MiniLM-L6-v2, 384 dimensions)
  - No external API calls — fully offline after model download (~80MB)
  - Hybrid search: FTS5 (full-text) + vector cosine similarity via Reciprocal Rank Fusion
  - Location: `crates/search/src/embedder.rs`, `crates/search/src/hybrid.rs`

**Serialization:**
- `serde` 1.x (with `derive`) — Serialization/deserialization framework
- `serde_json` 1.x — JSON serialization (used for import/export, API responses, MCP tools)

**Cryptography:**
- `chacha20poly1305` 0.10 — Authenticated encryption (ChaCha20-Poly1305 AEAD)
  - Used for optional database file encryption
  - Key derivation via SHA-256 (see `crates/core/src/crypto.rs`)
- `sha2` 0.10 — SHA-256 hashing (observation dedup hashes, key derivation)

**Error Handling:**
- `thiserror` 2.x — Custom error type derivation
- `anyhow` 1.x — Context-based error handling

**Logging:**
- `tracing` 0.1 — Structured logging framework
- `tracing-subscriber` 0.3 — Log output with `env-filter`

**Compression:**
- `flate2` 1.x — Gzip compression (used in sync chunk export/import)

**ID Generation:**
- `uuid` 1.x (v4, serde) — Session IDs and device IDs

**Date/Time:**
- `chrono` 0.4 (serde) — All timestamps (observations, sessions, edges)

**Base64 Encoding:**
- `base64` 0.22 — Data encoding (declared in workspace, used by core)

**Platform:**
- `dirs` 6.x — Cross-platform home directory detection

## Configuration

**Configuration Method:**
- CLI arguments (clap derive) — no config files
- Database path: `~/.engram/engram.db` (default), overridable via `--db`
- Project name: `--project` flag (default: "default")
- MCP tool profile: `--profile` flag (agent/admin/all)
- HTTP port: `--port` flag (default: 7437)

**Database Pragmas (at `crates/store/src/sqlite.rs`):**
- `journal_mode = WAL` (Write-Ahead Logging)
- `busy_timeout = 5000` (5 second timeout)
- `synchronous = NORMAL` (balanced durability/performance)
- `foreign_keys = ON`

**Logging:**
- Environment variable: `RUST_LOG` (via tracing-subscriber EnvFilter)
- Default level: `warn`

## Build Configuration

**Build Optimization (`Cargo.toml` [profile.release]):**
- `lto = true` — Link-Time Optimization enabled
- `codegen-units = 1` — Single codegen unit for maximum optimization
- `strip = true` — Strip debug symbols from release binary

**Binary Name:** `the-crab-engram`
**Entry Point:** `src/main.rs`

## Platform Requirements

**Development:**
- Rust compiler (Edition 2024 support required)
- SQLite headers (bundled via rusqlite `bundled` feature)
- Network access for first-run model download (fastembed, ~80MB)

**Production:**
- Single native binary (cross-platform: Windows/Linux/macOS)
- File system access for SQLite database
- No external services required (fully local/offline capable)

---

*Stack analysis: 2026-04-08*
