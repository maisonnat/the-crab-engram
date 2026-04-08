# Technical Design: engram-wiring

## Architecture Decisions

### AD-1: Composite Relevance Score
**Decision**: Add `compute_relevance_score()` to `store/sqlite.rs` that combines FTS5 rank with decay scoring.

**Approach**: Post-process search results. After FTS5 returns results ordered by rank, re-sort using:
```
final_score = 0.4 * fts_normalized_rank + 0.4 * decay_score + 0.2 * lifecycle_score
```
Pinned observations bypass scoring and appear first.

**Why not modify FTS5**: FTS5 rank is computed by SQLite's BM25 internally. We can't inject decay scoring into the FTS5 query. Post-processing is simpler and more correct.

### AD-2: Hybrid Search Integration
**Decision**: Wire `engram_search::reciprocal_rank_fusion()` into the MCP search handler.

**Approach**: In `tool_search_handler`:
1. Run FTS5 search (existing)
2. If `engram_search::Embedder` is available, run vector search via `store.search_vector()`
3. Combine with `reciprocal_rank_fusion(fts_results, vec_results, k=60)`
4. If no embedder, fall back to FTS5 + decay scoring only

**Dependency**: `engram-mcp` needs `engram-search` in Cargo.toml. It currently doesn't have it.

### AD-3: Episodic/Semantic on Save
**Decision**: Classify at insert time in `SqliteStore::insert_observation()`.

**Approach**: After inserting into `observations`, call `classify_query_type()` on the content. Based on result:
- `QueryTarget::Episodic` → INSERT into `episodic_memories`
- `QueryTarget::Semantic` → INSERT into `semantic_memories`
- `QueryTarget::Both` → INSERT into both

**Why in store, not MCP handler**: Ensures classification always happens regardless of entry point (CLI, MCP, API).

### AD-4: Beliefs Extraction on Save
**Decision**: Lightweight heuristic extraction in `mem_save` handler, not in store.

**Approach**: After saving observation, extract subject-predicate-value triples from content using simple pattern matching (e.g., "X uses Y", "X is Y", "X requires Y"). Call `upsert_belief()` for each.

**Why not in store**: Belief extraction is an NLP-like task that doesn't belong in the storage layer. The MCP handler is the right place.

### AD-5: MemoryStream Replaces Inline Handler
**Decision**: Replace all inline logic in `tool_stream_handler` with `MemoryStream` method calls.

**Mapping**:
| Current inline code | MemoryStream method |
|---|---|
| File pattern matching | `detect_file_context(project, file_path)` |
| Anti-pattern inline check | `detect_anti_pattern_warnings(project, content)` |
| Pending review check | `detect_pending_reviews(project)` |
| Entity extraction regex | `detect_entities(event_text)` |

### AD-6: Permissions as Opt-In
**Decision**: Permission checks only on destructive operations. Read operations are always allowed.

**Rationale**: The permission engine is for multi-agent scenarios. In single-agent usage (the current default), all agents have Admin access. Checks degrade to a simple `true` when there's only one rule.

### AD-7: Transactional Updates
**Decision**: Wrap `update_observation()` in `conn.execute("BEGIN")` / `COMMIT`.

**Approach**: Use rusqlite's `conn.transaction()` closure pattern:
```rust
conn.transaction(|tx| {
    tx.execute("UPDATE observations SET ...", params)?;
    tx.execute("UPDATE ...", other_params)?;
    Ok(())
})?;
```

### AD-8: API Route Pattern
**Decision**: New routes follow existing pattern — extract state, call store, return JSON.

**New dependency**: `engram-api` needs `engram-learn` for ConsolidationEngine, AntiPatternDetector, SmartInjector, CapsuleBuilder.

### AD-9: Vector Store Migration Stub
**Decision**: Create migration 003_vectors.sql with embeddings table schema. Implement `store_embedding()` as INSERT and `search_vector()` as "return empty" (no actual vector search without sqlite-vec extension).

**Rationale**: Having the table schema ready enables future sqlite-vec integration without migration changes. The stub methods returning `Ok(empty)` instead of `Err` allow code paths that check for embeddings to work gracefully.

## File Changes Summary

| File | Change Type | Description |
|---|---|---|
| `store/src/sqlite.rs` | Modify | Decay scoring in search, transactional updates, episodic/semantic inserts |
| `store/src/migrations/003_vectors.sql` | Create | Embeddings table schema |
| `store/src/migration.rs` | Modify | Add migration 003 to MIGRATIONS array |
| `store/src/trait.rs` | Modify | No changes needed |
| `mcp/src/tools/mod.rs` | Modify | MemoryStream handler, beliefs tool, permissions check, hybrid search |
| `mcp/src/server.rs` | Modify | Add Embedder to EngramServer if available |
| `mcp/Cargo.toml` | Modify | Add engram-search dependency |
| `api/src/lib.rs` | Modify | Add 6 new routes |
| `api/Cargo.toml` | Modify | Add engram-learn dependency |
| `src/main.rs` | Modify | Wire crypto module |
| `core/src/compaction.rs` | No change | Already has determine_level() |
| `learn/src/smart_injector.rs` | Modify | Use determine_level() |
| `tui/src/app.rs` | Modify | Show real data from learn engines |
| Multiple files | Modify | Remove unused imports (11 warnings) |
