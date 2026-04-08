# External Integrations

**Analysis Date:** 2026-04-08

## APIs & External Services

**Model Context Protocol (MCP) — Primary Integration:**
- Engram serves as an MCP server that AI coding agents connect to via stdio transport
- Protocol: JSON-RPC over stdin/stdout (not a network connection)
- SDK/Client: `rmcp` 1.3 (Rust MCP SDK)
- Auth: None (local stdio connection, trusted by default)
- Location: `crates/mcp/src/server.rs` (EngramServer implements ServerHandler)
- Location: `crates/mcp/src/tools/mod.rs` (50 tool definitions)

**Supported AI Agents (setup command generates SKILL.md):**
- Claude Code — writes to `~/.claude/skills/engram-memory.md`
- Cursor — writes to `~/.cursor/rules/engram-memory.md`
- Gemini CLI — writes to `~/.gemini/extensions/engram-memory.md`
- OpenCode — writes to `~/.config/opencode/skills/engram-memory.md`
- Location: `src/main.rs` lines 549-575 (Setup command)

## MCP Tools Exposed

**Agent Profile Tools (19 tools):**
- `mem_save` — Save an observation
- `mem_search` — Search memories by keyword
- `mem_context` — Get session context
- `mem_session_start` / `mem_session_end` — Session lifecycle
- `mem_get_observation` — Get full observation by ID
- `mem_suggest_topic_key` — Generate topic key suggestions
- `mem_capture_passive` — Auto-extract learnings from output
- `mem_save_prompt` — Save user prompt
- `mem_update` — Update observation
- `mem_capture_git` — Capture git commit as observation
- `mem_capture_error` — Capture compilation/test error
- `mem_stream` — Real-time memory event stream
- `mem_relate` — Add typed graph edges between observations
- `mem_graph` — Get knowledge graph around an observation
- `mem_pin` — Pin/unpin observation for max relevance
- `mem_inject` — Smart context injection for tasks
- `mem_synthesize` — Generate knowledge capsules
- `mem_capsule_list` / `mem_capsule_get` — Knowledge capsule access
- `mem_antipatterns` — Detect anti-patterns
- `mem_consolidate` — Run memory consolidation
- `mem_knowledge_boundary` — Query/update knowledge boundaries
- `mem_transfer` — Cross-project knowledge transfer
- `mem_reviews` — Spaced repetition reviews
- `mem_beliefs` — Query beliefs about a subject
- `mem_sync` — Sync operations (status/export/import)

**Admin Profile Tools (4 tools):**
- `mem_delete` — Delete an observation (hard delete)
- `mem_stats` — Project statistics
- `mem_timeline` — Timeline around observation
- `mem_merge_projects` — Merge project name variants

**MCP Resources (3 resources):**
- `engram://{project}/current-context` — Recent observations (text/markdown)
- `engram://{project}/knowledge-capsules` — Synthesized knowledge (text/markdown)
- `engram://{project}/anti-patterns` — Hotspot file warnings (text/markdown)

**MCP Notifications:**
- `notifications/stream/event` — Real-time stream events with 25ms throttle and content-hash dedup
- Location: `crates/mcp/src/server.rs` lines 132-173

## HTTP REST API

**Axum-based HTTP Server (port 7437):**
- Location: `crates/api/src/lib.rs`
- CORS: `tower_http::cors::CorsLayer::permissive()` (fully open CORS)

**Endpoints:**
| Method | Path | Purpose |
|--------|------|---------|
| GET | `/observations` | Search observations (query params) |
| POST | `/observations` | Create observation |
| GET | `/observations/:id` | Get observation by ID |
| PUT | `/observations/:id` | Update observation |
| DELETE | `/observations/:id` | Delete observation |
| POST | `/search` | Search (JSON body) |
| GET | `/stats` | Project statistics |
| POST | `/sessions` | Create session |
| GET | `/sessions/:id` | Get session |
| GET | `/context` | Session context |
| GET | `/export` | Export data to JSON |
| POST | `/import` | Import data from JSON |
| GET | `/capsules` | List knowledge capsules |
| GET | `/capsules/:topic` | Get capsule by topic |
| POST | `/consolidate` | Run memory consolidation |
| GET | `/graph/:id` | Get graph edges for observation |
| POST | `/inject` | Smart context injection |
| GET | `/antipatterns` | Detect anti-patterns |

## Data Storage

**Database:**
- SQLite (embedded, file-based)
- Connection: `rusqlite` 0.35 with `bundled` feature (ships compiled SQLite)
- Default path: `~/.engram/engram.db`
- Thread safety: `std::sync::Mutex<rusqlite::Connection>` (single-writer)
- Location: `crates/store/src/sqlite.rs`
- Location: `crates/store/src/trait.rs` (Storage trait — abstraction layer)

**Schema Tables (13 migrations):**
- `observations` — Core memory records (with FTS5 virtual table)
- `sessions` — Agent sessions
- `prompts` — Saved user prompts
- `edges` — Knowledge graph relationships (temporal validity)
- `embeddings` — Vector embeddings (384-dimensional)
- `knowledge_capsules` — Synthesized knowledge by topic
- `review_schedule` — Spaced repetition schedule
- `knowledge_boundaries` — Confidence domain tracking
- `beliefs` — Subject-predicate-value beliefs
- `entities` — Named entity extraction
- `observation_attachments` — Code diffs, error traces, git commits
- `agent_personalities` — Agent profile data
- Location: `crates/store/src/migrations/*.sql` (13 files)

**File Storage:**
- Local filesystem only — SQLite database file + optional encrypted copy
- Sync export: `./engram-chunks/` directory (gzip-compressed JSON chunks)
- No cloud storage integration

**Caching:**
- No dedicated cache layer
- FTS5 index serves as implicit search cache within SQLite
- Observation dedup uses `normalized_hash` column with 15-minute window

## Embedding Model

**Local Model (no external API):**
- Model: all-MiniLM-L6-v2 (384-dimensional vectors)
- Library: `fastembed` 5.x (Rust ONNX runtime)
- Download: ~80MB on first use (cached locally)
- Model versioning: Tracks `model_name` + `model_version` to detect drift
- Location: `crates/search/src/embedder.rs`

**Hybrid Search:**
- FTS5 full-text search (SQLite native) + vector cosine similarity
- Reciprocal Rank Fusion (RRF) combines results (k=60 constant)
- Weighted relevance: FTS 30% + vector 30% + recency 20% + frequency 20%
- Location: `crates/search/src/hybrid.rs`

## Authentication & Identity

**Auth Provider:** None — fully local application
- MCP connections are stdio-based (trusted by process spawning)
- HTTP API has no authentication (intended for local use)
- No user accounts, no OAuth, no API keys

**Encryption:**
- Optional database file encryption via ChaCha20Poly1305
- Key derivation: SHA-256(passphrase + "engram-salt-v1")
- Command: `the-crab-engram encrypt --passphrase <pass>`
- Location: `crates/core/src/crypto.rs`

## Sync & Cross-Device

**CRDT-based Sync (chunk export/import):**
- Vector clock for ordering
- Device ID (UUID v4) for per-device state
- LWW (Last-Writer-Wins) conflict resolution
- Gzip-compressed JSON chunks exported to filesystem
- No network sync — user transfers chunks manually
- Location: `crates/sync/src/crdt.rs`
- Location: `crates/sync/src/chunk.rs`

## CI/CD & Deployment

**CI Pipeline:**
- GitHub Actions
- Location: `.github/workflows/ci.yml`
- Location: `.github/workflows/release.yml`

**Release Scripts:**
- `scripts/release.sh` — Release automation
- `scripts/generate-changelog.sh` — Changelog generation

## Environment Configuration

**Required env vars:** None (all configuration via CLI flags)

**Optional env vars:**
- `RUST_LOG` — Log level filter (e.g., `engram=debug`)
- `DB_PATH` — Not used (use `--db` CLI flag instead)

## Webhooks & Callbacks

**Incoming:** None
**Outgoing:** None

## Cross-Language Compatibility

**Go Engram DB Migration:**
- Schema compatibility layer in `crates/store/src/migration.rs` (function `fix_schema_compat`)
- Automatically adds missing columns when opening a database created by the Go version
- Handles: `lifecycle_state`, `emotional_valence`, `surprise_factor`, `effort_invested`, `provenance_source`, `provenance_confidence`, `provenance_evidence`, `pinned`, `normalized_hash`, `summary`
- Location: `crates/store/src/migration.rs` lines 114-170

## Data Export Format

**JSON Export Schema (compatible with Engram Go):**
```json
{
  "observations": [...],
  "sessions": [...],
  "prompts": [...],
  "edges": [...]
}
```
- Location: `crates/store/src/trait.rs` lines 13-19 (ExportData struct)

---

*Integration audit: 2026-04-08*
