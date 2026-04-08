# Specification: engram-wiring

## Requirements

### WRQ-1: Decay Scoring Applied in Search Results
- **WHEN** a search is performed via MCP `mem_search` or API `GET /search`
- **THEN** results SHALL be re-ranked using `decay_score_with_lifecycle()` combining FTS5 rank with recency, frequency, and lifecycle decay
- **AND** pinned observations SHALL always appear first
- **AND** the scoring SHALL respect `LifecyclePolicy::decay_multiplier` per observation type

### WRQ-2: Hybrid Search Integration
- **WHEN** the MCP search handler is invoked with a text query
- **THEN** the system SHALL attempt FTS5 search AND embedding-based vector search (if embeddings available)
- **AND** results SHALL be combined using Reciprocal Rank Fusion (RRF) from `engram_search::reciprocal_rank_fusion()`
- **AND** if no embeddings are available, SHALL fall back to FTS5-only with decay scoring

### WRQ-3: Crypto Module Wired to Encrypt Command
- **WHEN** user runs `engram encrypt <file>` from CLI
- **THEN** the command SHALL use `engram_core::crypto::encrypt()` with a derived key from passphrase
- **AND** `engram decrypt <file>` SHALL use `engram_core::crypto::decrypt()`
- **AND** the current direct file I/O approach SHALL be replaced

### WRQ-4: Episodic/Semantic Memory Population
- **WHEN** an observation is saved via `mem_save` or `insert_observation()`
- **THEN** the system SHALL classify the query type using `classify_query_type()`
- **AND** IF episodic, SHALL insert into `episodic_memories` table with session context
- **AND** IF semantic, SHALL insert into `semantic_memories` table with domain classification

### WRQ-5: Beliefs Tool Wired to MCP
- **WHEN** `mem_save` is called with content containing factual claims
- **THEN** the system SHALL extract potential beliefs and call `upsert_belief()`
- **AND** a new MCP tool `mem_beliefs` SHALL expose `get_beliefs()` for querying belief state
- **AND** belief state machine transitions SHALL be applied via `process_evidence()` + `execute_operation()`

### WRQ-6: MemoryStream Wired to Stream Handler
- **WHEN** `mem_stream` MCP tool is invoked
- **THEN** the handler SHALL instantiate `MemoryStream::new(store, embedder)` 
- **AND** SHALL call `detect_file_context()`, `detect_anti_pattern_warnings()`, `detect_pending_reviews()`, or `detect_entities()` based on parameters
- **AND** the current inline pattern detection logic SHALL be removed

### WRQ-7: GraphEvolver Wired to Consolidation
- **WHEN** `mem_consolidate` MCP tool is invoked
- **THEN** after consolidation completes, SHALL call `GraphEvolver::new(store, embedder).evolve(&project)`
- **AND** newly detected edges SHALL be reported in the consolidation result

### WRQ-8: Permissions Check in MCP Handlers
- **WHEN** a destructive MCP tool is called (`mem_delete`, `mem_update`)
- **THEN** the handler SHALL verify agent permissions via `PermissionEngine::check()`
- **AND** IF insufficient permissions, SHALL return an error with access level required
- **AND** read-only operations SHALL not require permission checks

### WRQ-9: Transactional Observation Updates
- **WHEN** `update_observation()` modifies multiple fields
- **THEN** all SQL operations SHALL be wrapped in a single `BEGIN`/`COMMIT` transaction
- **AND** on error, SHALL `ROLLBACK` and return the error

### WRQ-10: SpacedRepetition Logic in Reviews Handler
- **WHEN** `mem_reviews` MCP tool is called
- **THEN** the handler SHALL call `bootstrap_reviews()` to initialize SM-2 schedules for observations without reviews
- **AND** SHALL display next review date computed from `SpacedRepetition.interval_days`
- **AND** review results SHALL feed back into `BoundaryTracker` confidence

### WRQ-11: Missing API Routes
- **GIVEN** the HTTP API is running
- **THEN** these routes SHALL exist and return correct data:
  - `POST /consolidate` — runs ConsolidationEngine
  - `GET /capsules` — lists KnowledgeCapsules
  - `GET /capsules/:topic` — gets capsule by topic
  - `GET /graph/:id` — gets edges for observation
  - `POST /inject` — runs SmartInjector
  - `GET /antipatterns` — runs AntiPatternDetector

### WRQ-12: API Search Uses Hybrid Search
- **WHEN** `POST /search` or `GET /observations` with query param is called
- **THEN** results SHALL use the same hybrid search (FTS5 + RRF + decay scoring) as MCP

### WRQ-13: Compaction Level in SmartInjector
- **WHEN** `SmartInjector::build_context()` constructs injection context
- **THEN** it SHALL call `compaction::determine_level(task)` to select abstraction level
- **AND** observations at the appropriate compaction level SHALL be prioritized

### WRQ-14: Vector Store Migration
- **THEN** migration `003_vectors.sql` SHALL exist creating embeddings table
- **AND** `store_embedding()` / `search_vector()` SHALL return `Ok(...)` with stub data instead of `Err`

### WRQ-15: All Compiler Warnings Resolved
- **THEN** `cargo check --workspace` SHALL produce zero warnings
- **AND** all unused imports SHALL be removed
- **AND** all unused variables SHALL be prefixed with `_` or removed

### WRQ-16: TUI Shows Real Learn Data
- **THEN** the TUI dashboard SHALL show anti-pattern count from `AntiPatternDetector`
- **AND** the capsules view SHALL display real capsules from `store.list_capsules()`
- **AND** the boundaries view SHALL show computed boundaries from `BoundaryTracker`

## Scenarios

### S1: Decay scoring affects search results
```gherkin
Given observation A was created 30 days ago with access_count=50
And observation B was created yesterday with access_count=1
And both match "auth" query
When mem_search("auth") is called
Then B should rank higher than A due to recency
Unless A is pinned
```

### S2: Beliefs extracted on save
```gherkin
Given mem_save is called with content "JWT uses RS256 algorithm"
When the observation is saved
Then a belief is upserted with subject="JWT", predicate="uses", value="RS256"
And belief state is Active with confidence 0.5
```

### S3: Consolidation evolves graph
```gherkin
Given 3 observations about "auth" exist
And 2 of them have supersedes relationships
When mem_consolidate is called
Then consolidation merges duplicates
And GraphEvolver detects temporal patterns
And new edges are reported in result
```

### S4: API capsule endpoint
```gherkin
Given a capsule for topic "auth" exists in store
When GET /capsules/auth is called
Then response contains capsule with summary, decisions, and confidence
And status is 200
```
