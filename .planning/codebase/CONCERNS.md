# Codebase Concerns

**Analysis Date:** 2026-04-08

## Security Concerns

### CRITICAL: Insecure CSPRNG Implementation
- **Issue:** Custom `getrandom()` in `crypto.rs` uses `SystemTime::now().as_nanos()` as sole entropy source. This is NOT cryptographically secure — an attacker who knows the approximate time of encryption can brute-force the nonce, breaking ChaCha20Poly1305 security guarantees.
- **Files:** `crates/core/src/crypto.rs` (lines 56-68)
- **Impact:** Encryption of database is fundamentally weakened. Nonce reuse or predictable nonces can expose plaintext.
- **Fix approach:** Replace with the `getrandom` crate (already a transitive dependency via `uuid`). Code even has a comment: "In production, use getrandom crate or OS RNG" — but it's using the insecure fallback.

### CRITICAL: Weak Key Derivation (SHA-256 instead of Argon2id)
- **Issue:** `derive_key()` uses a single SHA-256 hash with a static salt (`"engram-salt-v1"`) to derive encryption keys from passphrases. No key stretching, no memory hardness, no iteration count.
- **Files:** `crates/core/src/crypto.rs` (lines 85-93)
- **Impact:** Brute-force attacks on passphrases are trivial — millions of guesses per second. The code comment says "production: use Argon2id" but it's using SHA-256.
- **Fix approach:** Use `argon2` crate with random per-database salt. Store salt alongside encrypted data.

### HIGH: HTTP API Binds to 0.0.0.0 with No Authentication
- **Issue:** `Commands::Serve` binds to `0.0.0.0:{port}` (all interfaces) with no authentication or authorization. Anyone on the network can read/write/delete observations.
- **Files:** `src/main.rs` (line 436), `crates/api/src/lib.rs` (line 50 — `CorsLayer::permissive()`)
- **Impact:** Full database access from any network-connected device. CORS is also fully permissive.
- **Fix approach:** Bind to `127.0.0.1` by default. Add optional API key auth. Restrict CORS.

### MEDIUM: API Has No Rate Limiting or Input Size Limits
- **Issue:** No limits on request body size, observation content length, or request rate.
- **Files:** `crates/api/src/lib.rs`
- **Impact:** Memory exhaustion via oversized payloads. DoS via rapid requests.

## Tech Debt

### Version Mismatch Between Cargo.toml and CLI Output
- **Issue:** `Cargo.toml` declares version `2.1.0` but `main.rs` hardcodes `2.0.0` in three places (CLI parser `version` attribute line 14, `Commands::Version` line 424, and server banner line 437).
- **Files:** `Cargo.toml` (line 6), `src/main.rs` (lines 14, 424, 437)
- **Impact:** Users see wrong version. Automated version checks break.
- **Fix approach:** Use `env!("CARGO_PKG_VERSION")` instead of hardcoded strings.

### Migration Version Gap (Missing 5, 10, 14)
- **Issue:** Migration versions jump from 4→6, 9→11, 13→15. Versions 5, 10, and 14 are missing from the `MIGRATIONS` array.
- **Files:** `crates/store/src/migration.rs` (lines 12-65)
- **Impact:** No functional issue today (applied by version number), but confusing for maintainers. Future migrations must avoid these version numbers or risk collision.
- **Fix approach:** Document why versions were skipped, or renumber to sequential.

### SQLite Single-Connection Mutex Bottleneck
- **Issue:** `SqliteStore` wraps a single `rusqlite::Connection` in a `std::sync::Mutex`. Every database operation serializes through this lock. The `conn()` method uses `.expect()` which panics on poison.
- **Files:** `crates/store/src/sqlite.rs` (lines 20, 69-71)
- **Impact:** Under concurrent load (MCP server with multiple tool calls), all operations contend on one mutex. A poisoned lock crashes the entire server.
- **Fix approach:** Use `r2d2` or `deadpool` connection pool. Or switch to `tokio::sync::Mutex` if keeping single connection.

### Embedder Mutex Panic Risk
- **Issue:** `Embedder::embed_one()` and `embed_batch()` call `.lock().unwrap()` on the model mutex. If the lock is poisoned (e.g., a panic during embedding), the server crashes.
- **Files:** `crates/search/src/embedder.rs` (lines 89, 96)
- **Impact:** Any panic during embedding poisons the lock and crashes subsequent calls.
- **Fix approach:** Use `.lock().map_err(...)` to convert to a proper error.

### MCP Server Mutex Panic Risk
- **Issue:** `EngramServer` stores peer and event_tx in `std::sync::Mutex` and calls `.lock().unwrap()` in multiple places.
- **Files:** `crates/mcp/src/server.rs` (lines 126, 200, 212)
- **Impact:** Panic on poisoned lock kills the MCP server.
- **Fix approach:** Use proper error handling or `parking_lot::Mutex` (which doesn't poison).

### Consolidation Hardcoded 1000 Limit
- **Issue:** `ConsolidationEngine` fetches observations with `limit: Some(1000)` in multiple methods. Projects with >1000 observations will silently skip older entries during consolidation.
- **Files:** `crates/learn/src/consolidation.rs` (lines 83, 179, 213, 275)
- **Impact:** Incomplete dedup, stale marking, and pattern extraction for large projects.
- **Fix approach:** Use cursor-based pagination or remove limit for internal operations.

### search_vector Returns Empty (Vector Search Disabled)
- **Issue:** `search_vector()` always returns `vec![]` — vector similarity search is not implemented. Requires `sqlite-vec` extension.
- **Files:** `crates/store/src/sqlite.rs` (lines 935-939)
- **Impact:** Hybrid search falls back to FTS5-only. Embeddings are stored but never used for retrieval. The `engram-search` embedder is effectively write-only.
- **Fix approach:** Integrate `sqlite-vec` or use an in-memory vector index.

### Sync Module is a Stub
- **Issue:** `CrdtState` is regenerated fresh on every call (`CrdtState::new()` creates new UUID). `get_sync_status()` always reports `pending_deltas: 0`. No actual sync protocol implemented.
- **Files:** `crates/sync/src/crdt.rs` (lines 83-90), `crates/mcp/src/tools/mod.rs` (line 1843)
- **Impact:** Sync status is meaningless. Cross-device sync doesn't work. Export/import works but is manual.
- **Fix approach:** Persist device state. Implement delta tracking.

### Anti-Pattern Detection is Naive Text Matching
- **Issue:** Anti-pattern detection finds "hotspot files" by splitting titles/content on whitespace and checking if words contain `.rs`, `.ts`, or `.go`. This is fragile — a file mentioned in passing counts as a bug location.
- **Files:** `crates/mcp/src/tools/mod.rs` (lines 715-721, 835-841), `crates/mcp/src/server.rs` (lines 364-370)
- **Impact:** False positives. A discussion about `main.rs` in content counts as a bug in `main.rs`.
- **Fix approach:** Parse file paths structurally (e.g., from attachments, structured metadata).

### Belief Extraction is Heuristic-Only
- **Issue:** `extract_and_upsert_beliefs()` uses simple string pattern matching ("uses", "is", "requires") to extract subject-predicate-value triples. Very noisy — any sentence with "is" creates a belief.
- **Files:** `crates/mcp/src/tools/mod.rs` (lines 1902-1928)
- **Impact:** Garbage beliefs accumulate. E.g., "This is a test" → belief: "This" is "a test".
- **Fix approach:** Use NLP-based extraction or require structured input format.

## Performance Concerns

### O(n²) Semantic Dedup
- **Issue:** `merge_semantic_duplicates()` compares every pair of observations with cosine similarity. With 1000 observations, this is ~500K comparisons.
- **Files:** `crates/learn/src/consolidation.rs` (lines 138-169)
- **Impact:** Consolidation time grows quadratically. Will be slow (>10s) at scale.
- **Fix approach:** Use approximate nearest neighbor (ANN) index or LSH for dedup.

### O(n²) Conflict Detection
- **Issue:** `find_contradictions()` also does pairwise comparison within topic groups, with an additional DB query per pair to check edges.
- **Files:** `crates/learn/src/consolidation.rs` (lines 234-256)
- **Impact:** Many DB round-trips in nested loop. Slow for topics with many decisions.

### Row Mapper Silent Data Loss
- **Issue:** Multiple row mappers use `.unwrap_or_default()` for date parsing, provenance source, lifecycle state, and JSON deserialization. If a row has corrupted data, the observation loads with default values instead of failing.
- **Files:** `crates/store/src/sqlite.rs` (lines 99, 103, 114, 115, 130, 146, 168-178, 183, 187)
- **Impact:** Silent data corruption. Observations appear valid but have wrong dates, types, or empty evidence.
- **Fix approach:** Log warnings when defaults are used. Consider failing hard for critical fields.

### Character-Boundary Safe Truncation Bug
- **Issue:** `truncate_str()` uses byte indexing `&s[..max]` which will panic on multi-byte UTF-8 characters if `max` falls in the middle of a character.
- **Files:** `crates/mcp/src/tools/mod.rs` (lines 1931-1936)
- **Impact:** Panic on non-ASCII content (e.g., CJK, emoji, accented characters).
- **Fix approach:** Use `s.char_indices().take(max)` or `unicode-segmentation` crate.

## Fragile Areas

### `main.rs` Consolidate Command Has Potential Panic
- **Issue:** `group.iter().max_by_key(|o| o.id).unwrap()` on line 469 — if `group` is somehow empty (shouldn't be due to `len() > 1` check, but fragile).
- **Files:** `src/main.rs` (line 469)
- **Impact:** Very low probability panic, but the pattern is repeated in consolidation.rs line 109.
- **Fix approach:** Use `if let Some(newest) = ...` pattern.

### `json_schema()` Panics on Non-Object JSON
- **Issue:** `json_schema()` calls `.as_object().unwrap()` — panics if the schema JSON is not an object.
- **Files:** `crates/mcp/src/tools/mod.rs` (line 132)
- **Impact:** Currently all calls pass valid JSON objects, but adding a new tool with bad schema would panic at startup.
- **Fix approach:** Return `Result` or use `.expect("schema must be an object")` with context.

### Schema Compatibility Migration Silently Ignores Errors
- **Issue:** `fix_schema_compat()` runs `conn.execute_batch(cmd)` inside `let _ = ...`, ignoring all errors.
- **Files:** `crates/store/src/migration.rs` (line 167)
- **Impact:** If an ALTER TABLE fails for a real reason (not just "column exists"), it's silently swallowed.
- **Fix approach:** Check error message for "duplicate column" before ignoring.

### `get_edges` Stats Query Ignores Project Scope
- **Issue:** `get_stats()` counts edges with `SELECT COUNT(*) FROM edges` without filtering by project. Cross-project edges inflate stats.
- **Files:** `crates/store/src/sqlite.rs` (lines 752-756)
- **Impact:** Edge count in stats is global, not per-project. Misleading.

### Import Doesn't Import Edges
- **Issue:** `import()` method imports observations, sessions, and prompts, but ignores `data.edges` entirely. The `ImportResult` reports `edges_imported` as `data.edges.len()` without actually importing them.
- **Files:** `crates/store/src/sqlite.rs` (lines 1048-1135, specifically 1132)
- **Impact:** Export/import round-trip loses graph edges. `edges_imported` count is a lie.
- **Fix approach:** Add edge INSERT logic similar to observations.

### Beliefs Upsert is INSERT-Only (No ON CONFLICT)
- **Issue:** `upsert_belief()` does a plain INSERT without `ON CONFLICT` clause, despite the name suggesting upsert behavior. Duplicate subjects with same predicate+value will create duplicate rows.
- **Files:** `crates/store/src/sqlite.rs` (lines 1397-1413)
- **Impact:** Duplicate belief entries accumulate over time.
- **Fix approach:** Add `ON CONFLICT(subject, predicate, value) DO UPDATE SET confidence=excluded.confidence, state=excluded.state`.

## Test Coverage Gaps

### No Integration Tests for MCP Server
- **Issue:** No test exercises the MCP tool dispatch, server initialization, or notification delivery.
- **Files:** `crates/mcp/src/`
- **Impact:** MCP regressions are caught only in production.
- **Fix approach:** Add integration tests with mock transport.

### Embedding Tests Are `#[ignore]`
- **Issue:** All embedding tests (embed_one, semantic_similarity, hydrate) require an 80MB model download and are marked `#[ignore]`.
- **Files:** `crates/search/src/embedder.rs` (lines 228, 236, 255)
- **Impact:** Embedding regressions not caught in CI.
- **Fix approach:** Add CI job with model cache. Or mock the embedder.

### No Tests for API Routes
- **Issue:** Only one test in `api/src/lib.rs` (serialization). No HTTP handler tests.
- **Files:** `crates/api/src/lib.rs`
- **Impact:** API regressions uncaught.

### No Tests for Crypto Encrypt/Decrypt Under Load
- **Issue:** Crypto tests verify roundtrip but not concurrent safety, large payloads, or edge cases with the insecure RNG.
- **Files:** `crates/core/src/crypto.rs`

## Missing Critical Features

### No Input Validation on Observation Content
- **Issue:** No length limits on title, content, or topic_key at the store level. The `EngramError::TooLong` variant exists but is never used.
- **Files:** `crates/core/src/error.rs` (line 13), `crates/store/src/sqlite.rs`
- **Impact:** Users can insert arbitrarily large observations, consuming disk and memory.
- **Fix approach:** Add validation in `insert_observation()` — reject titles > 500 chars, content > 100K chars.

### No Database Backup/Restore Mechanism
- **Issue:** No way to create consistent backups while the server is running. The encrypt command reads raw file bytes which may be inconsistent with WAL mode.
- **Files:** `src/main.rs` (lines 526-546)
- **Impact:** Encryption of a live SQLite DB with WAL may produce corrupt output.
- **Fix approach:** Use `VACUUM INTO` or SQLite Online Backup API before encrypting.

---

*Concerns audit: 2026-04-08*
