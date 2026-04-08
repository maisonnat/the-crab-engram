# Engram-Rust — Changelog

## v2.0.0 (2026-04-07)

### Summary

Full SDD (Spec-Driven Development) implementation of Engram-Rust v2.0.0. Complete rewrite from the Go-based v1 with expanded capabilities.

- **277 SDD tasks** completed
- **199 tests** passing (0 failures)
- **0 warnings** (clippy clean)
- **13 database migrations**
- **31 MCP tools**, **14 HTTP routes**, **15 CLI commands**

### Major Features

#### Core (engram-core)
- `Observation` with 14 types, provenance tracking, lifecycle states, salience scoring
- `Edge` temporal knowledge graph with 5 relation types and validity windows
- `Session` management with UUID v4 IDs
- `KnowledgeCapsule` dense topic synthesis
- `Belief` state machine (Active → Confirmed/Contested/Superseded/Retracted)
- `Attachment` multimodal support (CodeDiff, TerminalOutput, ErrorTrace, GitCommit)
- `PermissionEngine` multi-agent access control (Read/Write/Admin)
- ChaCha20-Poly1305 encryption with passphrase key derivation
- Entity extraction and resolution
- Compaction levels (Raw/Fact/Pattern/Principle)
- Episodic and Semantic memory types

#### Storage (engram-store)
- `Storage` trait — 35-method abstraction layer (hexagonal architecture)
- `SqliteStore` — SQLite + FTS5 full implementation
- WAL mode, busy timeout, synchronous=NORMAL
- 13 idempotent migrations
- Deduplication by normalized_hash (SHA-256)
- Hybrid search: FTS5 + vector similarity
- Export/Import JSON round-trip
- Spaced repetition review schedule
- Cross-project knowledge transfers
- Agent personality profiles

#### MCP Server (engram-mcp)
- 31 MCP tools covering full lifecycle
- 3 MCP resources (context, capsules, anti-patterns)
- Tool profiles: Agent (27 tools), Admin (4 tools), All (31 tools)
- Auto-consolidation background task (every 30 minutes)
- Stream event delivery via tokio::sync::mpsc with 25ms throttle
- Anti-pattern detection on bugfix saves
- Belief auto-extraction from content
- stdio transport via rmcp v1.3

#### HTTP API (engram-api)
- 14 Axum routes with CORS
- Full CRUD for observations and sessions
- Search, stats, context, export/import
- Knowledge capsules, graph edges, consolidation
- Smart context injection, anti-pattern detection

#### Auto-Learning (engram-learn)
- **ConsolidationEngine** — duplicate merging, obsolete marking, conflict detection, pattern extraction
- **CapsuleBuilder** — synthesis of observations into KnowledgeCapsules
- **HeuristicSynthesizer** — rule-based capsule generation
- **SmartInjector** — context injection with relevant memories, warnings, boundaries
- **AntiPatternDetector** — recurring bugs, unverified decisions, hotspot files
- **MemoryStream** — file context, deja-vu detection, anti-pattern warnings, pending reviews
- **GraphEvolver** — auto-detect edges between observations
- **BoundaryTracker** — knowledge gap tracking with confidence levels
- **SpacedRepetition** — SM-2 algorithm for memory review scheduling
- **SalienceInference** — emotional valence, surprise factor, effort invested

#### Search (engram-search)
- Embedder interface for text embeddings
- Hybrid scoring: FTS5 rank + vector similarity
- Reciprocal rank fusion for result merging

#### Sync (engram-sync)
- CRDT state management with vector clocks
- Chunk export/import for cross-machine sync
- Conflict resolution strategies

#### TUI (engram-tui)
- Dashboard view with project stats
- Search with live typing
- Capsules browser
- Boundaries viewer
- Ratatui + Crossterm rendering

#### CLI
- 15 subcommands: mcp, search, save, context, stats, timeline, export, import, export-context, session-start, session-end, serve, tui, consolidate, sync, encrypt, setup
- Agent setup for Claude Code, Cursor, Gemini CLI, Opencode
- Database encryption/decryption
- Markdown system prompt export (killer feature)

### Technology Stack
- Rust 2024 edition (resolver v3)
- SQLite 3 (bundled via rusqlite) with FTS5
- rmcp v1.3 (MCP protocol)
- Axum 0.8 (HTTP)
- Ratatui + Crossterm (TUI)
- ChaCha20-Poly1305 (encryption)
- Tokio (async runtime)
- Clap 4 (CLI)
- Serde + JSON (serialization)
- thiserror + anyhow (error handling)
