# Learn Daemon Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Add an autonomous Learn Daemon that periodically runs Engram’s existing learning engines in the background, both as a standalone CLI command and optionally alongside `serve`.

**Architecture:** Reuse the already-existing `engram-learn` building blocks instead of inventing new algorithms. Add a new orchestration layer (`LearnDaemon`) in the root binary crate that wires together `MemoryStream`, `ConsolidationEngine`, `GraphEvolver`, `CapsuleBuilder`, `SpacedRepetition`, `AntiPatternDetector`, and `SmartInjector`, then executes them on a configurable interval.

**Tech Stack:** Rust 2024, `clap`, `tokio`, `Arc<dyn Storage>`, existing crates `engram-learn`, `engram-store`, `engram-core`, `engram-api`.

---

## 1. What exists already

### `crates/learn/src/lib.rs`
Re-exports the full learning surface:
- `MemoryStream`
- `ConsolidationEngine`, `ConsolidationResult`
- `GraphEvolver`, `EvolutionResult`, `NewEdge`
- `CapsuleBuilder`, `CapsuleSynthesizer`, `HeuristicSynthesizer`, `ChainedSynthesizer`
- `AntiPatternDetector`, `AntiPattern`, `AntiPatternType`, `Severity`
- `SpacedRepetition`, `ReviewResult`, `bootstrap_reviews`
- `SmartInjector`, `InjectionContext`
- `InferenceEngine`, `CacheKey`
- `ExtractionPipeline`, `ExtractionResult`, `KnowledgeExtraction`, etc.

### `MemoryStream`
**Fields**
- `store: Arc<dyn Storage>`
- `embedder: Option<Arc<Embedder>>`

**Public API**
- `new(store, embedder) -> Self`
- `detect_file_context(project, file_path) -> Result<Vec<MemoryEvent>, EngramError>`
- `detect_deja_vu(project, task_description) -> Result<Vec<MemoryEvent>, EngramError>`
- `detect_anti_pattern_warnings(project, current_content) -> Result<Vec<MemoryEvent>, EngramError>`
- `detect_pending_reviews(project) -> Result<Vec<MemoryEvent>, EngramError>`
- `extract_entities(text) -> Vec<ExtractedEntity>`
- `observe_topics(text) -> HashMap<String, f64>`
- `detect_entities(&self, event_text) -> Result<Vec<MemoryEvent>, EngramError>`

**Needs embedder?**
- `detect_deja_vu` yes
- everything else no

**Important behavior**
- File context is heuristic string matching over title+content.
- Pending review detection currently uses decision observations + age + access count.
- Entity extraction is heuristic only.

### `GraphEvolver`
**Fields**
- `store: Arc<dyn Storage>`
- `embedder: Option<Arc<Embedder>>`

**Public structs**
- `NewEdge { source_id, target_id, relation, weight, reason }`
- `EvolutionResult { edges_created, temporal_patterns, file_correlations, semantic_clusters }`

**Public API**
- `new(store, embedder) -> Self`
- `evolve(project) -> Result<EvolutionResult, EngramError>`

**Needs embedder?**
- temporal patterns: no
- file correlations: no
- semantic clusters: yes

**Important behavior**
- Temporal detector infers `CausedBy` when A precedes B in 3+ sessions.
- File detector infers `RelatedTo` when observations mention the same file.
- Semantic clusters use cosine similarity > 0.85 and < 0.99.
- Inserts edges through `Storage::add_edge`.

### `CapsuleBuilder`
**Public types**
- `CapsuleSynthesizer` trait:
  - `synthesize(observations, topic) -> Result<KnowledgeCapsule, EngramError>`
  - `can_synthesize() -> bool`
  - `name() -> &str`
- `HeuristicSynthesizer`
- `ChainedSynthesizer { primary, fallback }`
- `CapsuleBuilder { store, synthesizer }`

**Public API**
- `ChainedSynthesizer::new(primary, fallback) -> Self`
- `CapsuleBuilder::new(store, synthesizer) -> Self`
- `CapsuleBuilder::build_capsule(project, topic) -> Result<KnowledgeCapsule, EngramError>`

**Needs embedder/model?**
- heuristic path: no
- future LLM synthesizer: potentially yes, but not required now

**Important behavior**
- Current heuristic synthesizer fills:
  - `source_observations`
  - `key_decisions`
  - `known_issues`
  - `anti_patterns`
  - `best_practices`
  - `summary`
  - `confidence`
- `build_capsule()` currently returns a capsule but does **not** persist it; caller must call `store.upsert_capsule()`.

### `ConsolidationEngine`
**Fields**
- `store: Arc<dyn Storage>`
- `embedder: Option<Arc<Embedder>>`

**Public structs**
- `ConsolidationResult { duplicates_merged, obsolete_marked, conflicts_found, patterns_extracted, time_taken_ms }`

**Public API**
- `new(store, embedder) -> Self`
- `run_consolidation(project) -> Result<ConsolidationResult, EngramError>`

**Needs embedder?**
- duplicate merging uses embedder if available, otherwise hash dedup
- other phases do not require embedder

**Important behavior**
- Merges duplicates
- Marks obsolete observations as `stale`
- Finds contradictions
- Extracts patterns and stores them as new observations using `AddObservationParams`

### `infer_salience`
**Public API**
- `infer_salience(content, session_length_minutes) -> MemorySalience`

**Needs embedder/model?**
- no

**Important behavior**
- Pure heuristic inference of emotional valence, surprise, effort.
- Very good fit for enriching newly created daemon-generated observations.

### `AntiPatternDetector`
**Fields**
- `store: Arc<dyn Storage>`
- `embedder: Option<Arc<Embedder>>`

**Public types**
- `Severity { Low, Medium, High, Critical }`
- `AntiPatternType { RecurringBug, RevertPattern, HotspotFile, UnverifiedDecision }`
- `AntiPattern { type, description, evidence, severity, suggestion }`

**Public API**
- `new(store, embedder) -> Self`
- `detect_all(project) -> Result<Vec<AntiPattern>, EngramError>`
- `detect_recurring_bugs(project, min_occurrences, similarity_threshold)`
- `detect_revert_patterns(project)`
- `detect_hotspot_files(project, min_mentions)`
- `detect_unverified_decisions(project, confidence_threshold)`

**Needs embedder?**
- recurring bugs yes
- revert patterns no
- hotspot files no
- unverified decisions no

### `SpacedRepetition`
**Public types**
- `ReviewResult { Perfect, Good, Hard, Forgotten }`
- `SpacedRepetition { memory_id, interval_days, ease_factor, review_count, last_result }`

**Public API**
- `SpacedRepetition::new(memory_id) -> Self`
- `process_review(result)`
- `bootstrap_reviews(observation_ids_with_access, count) -> Vec<SpacedRepetition>`

**Needs embedder/model?**
- no

**Important behavior**
- Pure algorithmic scheduler.
- Persistence must be done by caller through `Storage::upsert_review()` and `Storage::get_pending_reviews()`.

### `SmartInjector`
**Public types**
- `InjectionContext { relevant_memories, knowledge_capsules, warnings, knowledge_boundaries, review_reminders, total_tokens }`

**Public API**
- `InjectionContext::is_empty()`
- `InjectionContext::to_markdown()`
- `InjectionContext::estimate_tokens()`
- `SmartInjector::new(store) -> Self`
- `SmartInjector::build_context(project, task, max_tokens) -> Result<InjectionContext, EngramError>`

**Needs embedder/model?**
- no (uses store search + heuristics)

**Important behavior**
- Returns markdown, but does not currently persist the context anywhere.
- Good candidate for daemon-generated “context snapshot” observations or API exposure.

### `InferenceEngine`
**Public API**
- `new(model_path) -> Self`
- `is_loaded() -> bool`
- `load() -> Result<()>`
- `unload()`
- `infer(prompt) -> Result<String>`
- `cache_system_prompt(system, schema) -> CacheKey`
- `infer_with_cache(cache_key, user_prompt) -> Result<String>`

**Needs model?**
- In non-`inference` builds it is stubbed and still works.
- In `feature = "inference"` real `llama_cpp_rs` wiring is still TODO.

### `ExtractionPipeline`
**Public API**
- `new(engine) -> Self`
- `extract(input) -> Result<ExtractionResult, ExtractionError>`
- `grammar() -> &'static str`

**Needs model?**
- yes, via `InferenceEngine`
- but current engine can run in stub mode for tests only

## 2. Storage APIs the daemon will use

From `Storage`:
- `search()`
- `insert_observation()`
- `update_observation()`
- `delete_observation()`
- `get_edges()` / `add_edge()`
- `upsert_capsule()` / `list_capsules()`
- `upsert_review()` / `get_pending_reviews()`
- `upsert_boundary()` / `get_boundaries()`
- `upsert_entity()` / `link_entity_observation()` / `get_entity()`
- `get_stats()`

## 3. Observation fields relevant to daemon-generated memories

`Observation` includes:
- identity: `id`, `type`, `scope`, `title`, `content`, `session_id`, `project`, `topic_key`
- timing: `created_at`, `updated_at`, `recorded_at`
- usage: `access_count`, `last_accessed`, `pinned`
- dedup: `normalized_hash`
- provenance: `provenance_source`, `provenance_confidence`, `provenance_evidence`
- lifecycle: `lifecycle_state`
- salience: `emotional_valence`, `surprise_factor`, `effort_invested`

The daemon should set provenance explicitly for generated observations, likely:
- `provenance_source = Inferred`
- confidence depending on subsystem
- evidence containing input observation IDs

---

## 4. Proposed feature: Learn Daemon

### New user-facing commands

Add a new top-level command:

```rust
Learn {
    #[arg(long)]
    daemon: bool,
    #[arg(long, default_value = "60")]
    interval: u64,
    #[arg(long)]
    once: bool,
    #[arg(long)]
    with_serve: bool,
}
```

Recommended final UX:
- `engram learn --once`
- `engram learn --daemon --interval 60`
- `engram serve --port 7437 --learn-daemon --learn-interval 60`

### New orchestration module

Create a new binary-side module:
- `src/learn_daemon.rs`

Define:

```rust
pub struct LearnDaemonConfig {
    pub project: String,
    pub interval_seconds: u64,
    pub max_search_observations: usize,
    pub max_capsules_per_tick: usize,
    pub max_reviews_bootstrap: usize,
    pub max_injection_tokens: usize,
    pub write_summary_observations: bool,
    pub enable_consolidation: bool,
    pub enable_evolution: bool,
    pub enable_capsules: bool,
    pub enable_reviews: bool,
    pub enable_anti_patterns: bool,
    pub enable_injection_snapshots: bool,
}
```

### Sensible defaults

```rust
impl Default for LearnDaemonConfig {
    fn default() -> Self {
        Self {
            project: "default".into(),
            interval_seconds: 60,
            max_search_observations: 1000,
            max_capsules_per_tick: 5,
            max_reviews_bootstrap: 25,
            max_injection_tokens: 1200,
            write_summary_observations: true,
            enable_consolidation: true,
            enable_evolution: true,
            enable_capsules: true,
            enable_reviews: true,
            enable_anti_patterns: true,
            enable_injection_snapshots: false,
        }
    }
}
```

### Core orchestration type

```rust
pub struct LearnDaemon {
    store: Arc<dyn Storage>,
    config: LearnDaemonConfig,
    embedder: Option<Arc<Embedder>>,
}
```

Public methods:
- `new(store, config, embedder) -> Self`
- `run_once(&self) -> Result<LearnTickResult>`
- `run_loop(&self, shutdown: impl Fn() -> bool) -> Result<()>`

And a per-tick report:

```rust
pub struct LearnTickResult {
    pub consolidation: Option<ConsolidationResult>,
    pub evolution: Option<EvolutionResult>,
    pub capsules_upserted: usize,
    pub reviews_upserted: usize,
    pub anti_patterns_found: usize,
    pub entities_linked: usize,
    pub snapshots_written: usize,
}
```

---

## 5. Main loop phases

Order per tick:

### Phase 1 — Observe
**Goal:** derive structured hints from fresh observations.

Actions:
1. Read recent project observations with `search(query="", limit=max_search_observations)`.
2. For each new/recent observation:
   - `MemoryStream::extract_entities(title + content)`
   - `MemoryStream::observe_topics(title + content)`
   - `infer_salience(content, None)`
3. Persist entities using:
   - `upsert_entity()`
   - `link_entity_observation()`
4. Optional: update salience-related fields on observations using `update_observation()` if params already support them; otherwise skip in first iteration.

**Does not require embedder** except for optional deja-vu.

### Phase 2 — Consolidate
**Goal:** clean duplicates, stale items, contradictions, extract patterns.

Actions:
- `ConsolidationEngine::run_consolidation(project)`
- Persist result only via the engine’s own DB operations.
- Optionally write one daemon summary observation such as:
  - title: `Learn tick: consolidation`
  - type: `Learning`
  - topic_key: `learn/consolidation`

### Phase 3 — Evolve
**Goal:** grow the graph edges automatically.

Actions:
- `GraphEvolver::evolve(project)`
- The engine already inserts edges.
- Optional summary observation:
  - title: `Learn tick: graph evolution`
  - type: `Learning`
  - topic_key: `learn/graph-evolution`

### Phase 4 — Capsule
**Goal:** synthesize capsules for strong topics.

Actions:
1. Scan observations and count `topic_key` prefixes/domains.
2. Select top N topics lacking capsules or with stale capsules.
3. Build with:
   - `CapsuleBuilder::build_capsule(project, topic)`
4. Persist with:
   - `store.upsert_capsule(&capsule)`

**Important:** the builder returns but does not persist.

**No embedder required** with `HeuristicSynthesizer`.

### Phase 5 — Review
**Goal:** maintain spaced repetition schedules.

Actions:
1. `store.get_pending_reviews(Some(project), limit)` to inspect due reviews.
2. If there are none or too few scheduled items, bootstrap from top-accessed observations:
   - collect `(id, access_count)` from observations
   - `bootstrap_reviews(&pairs, max_reviews_bootstrap)`
3. Persist schedules with `upsert_review(observation_id, interval_days, ease_factor, next_review)`.

This first version does **not** auto-grade reviews. It only ensures schedules exist.

### Phase 6 — AntiPattern
**Goal:** detect systemic problems.

Actions:
- `AntiPatternDetector::detect_all(project)`
- Optionally persist each finding as a `Pattern` or `Bugfix`-adjacent `Learning` observation:
  - title: `Anti-pattern detected: Hotspot File`
  - content: description + suggestion + evidence ids
  - topic_key: `learn/anti-pattern/<slug>`

### Phase 7 — Inject
**Goal:** produce a smart summary of what the agent should remember right now.

Actions:
- `SmartInjector::build_context(project, "current project state", max_injection_tokens)`
- First version should **not** auto-inject into live sessions.
- Safer first step: optionally persist a snapshot observation:
  - type: `Learning`
  - title: `Injection snapshot`
  - content: `ctx.to_markdown()`
  - topic_key: `learn/injection-snapshot`

---

## 6. What runs without embedder vs with embedder

### Works without embedder
- `infer_salience`
- `MemoryStream::detect_file_context`
- `MemoryStream::detect_anti_pattern_warnings`
- `MemoryStream::detect_pending_reviews`
- `MemoryStream::extract_entities`
- `MemoryStream::observe_topics`
- `GraphEvolver` temporal patterns
- `GraphEvolver` file correlations
- `CapsuleBuilder` + `HeuristicSynthesizer`
- `ConsolidationEngine` hash dedup / obsolete / contradictions / pattern extraction
- `SpacedRepetition`
- `SmartInjector`
- `AntiPatternDetector` except recurring-bug clustering

### Needs embedder
- `MemoryStream::detect_deja_vu`
- `GraphEvolver` semantic clusters
- `ConsolidationEngine` semantic dedup
- `AntiPatternDetector::detect_recurring_bugs`

### Needs inference engine / model
- `ExtractionPipeline`
- Any future LLM-based capsule synthesizer

**Conclusion:** the daemon is worth implementing immediately even with `embedder = None`.

---

## 7. Integration into CLI

### A. Standalone mode
Modify `src/main.rs`:
1. Add `use engram_learn::{...}` imports.
2. Add `mod learn_daemon;`
3. Add `Commands::Learn { daemon, interval, once }`.
4. Match arm behavior:
   - `--once` → run one tick and print summary
   - `--daemon` → endless loop sleeping `interval` seconds
   - if neither provided, default to one tick or reject and ask for one mode

### B. Sidecar mode inside `serve`
Extend the command:

```rust
Serve {
    #[arg(long, default_value = "7437")]
    port: u16,
    #[arg(long)]
    learn_daemon: bool,
    #[arg(long, default_value = "60")]
    learn_interval: u64,
}
```

Implementation approach:
1. Open store once as `Arc<SqliteStore>`.
2. If `learn_daemon`, clone `Arc` and spawn a tokio task or std thread.
3. That task creates `LearnDaemon` and runs `run_loop()`.
4. HTTP server continues normally.
5. Log failures with `tracing::error!`, but don’t crash the API if one tick fails.

**Prefer std thread + sleep for first implementation** because all learning code is synchronous and store APIs are sync.

Pseudo:

```rust
if learn_daemon {
    let daemon_store = store.clone();
    let cfg = LearnDaemonConfig { project: cli.project.clone(), interval_seconds: learn_interval, ..Default::default() };
    std::thread::spawn(move || {
        let daemon = LearnDaemon::new(daemon_store, cfg, None);
        if let Err(err) = daemon.run_loop(|| false) {
            tracing::error!(?err, "learn daemon stopped");
        }
    });
}
```

---

## 8. New files and file changes

### New file
- `src/learn_daemon.rs`
  - `LearnDaemonConfig`
  - `LearnDaemon`
  - `LearnTickResult`
  - helper functions per phase
  - unit tests with `SqliteStore::in_memory()`

### Modify
- `src/main.rs`
  - new subcommand `Learn`
  - extend `Serve` flags
  - wire daemon startup
- possibly `Cargo.toml`
  - verify root crate already depends on `engram-learn`; if not, add it
- maybe `README.md`
  - document `engram learn --daemon`

### Optional future changes, not required in first pass
- `crates/api/src/lib.rs`
  - expose daemon status endpoint
- `crates/mcp/src/` 
  - expose “run learn tick now” tool

---

## 9. Persistence strategy for daemon findings

To avoid noisy duplicates, daemon-written summary observations should:
- use deterministic titles by subsystem + tick date bucket
- use `ObservationType::Learning`
- use scoped topic keys:
  - `learn/consolidation`
  - `learn/graph-evolution`
  - `learn/capsule/<topic>`
  - `learn/reviews`
  - `learn/anti-pattern/<kind>`
  - `learn/injection-snapshot`
- keep concise content so dedup by hash remains effective

Recommended first version:
- Persist only anti-pattern findings and injection snapshots optionally.
- Don’t spam summary observations for every tick by default.

---

## 10. Test strategy

### Unit tests in `src/learn_daemon.rs`
Use `SqliteStore::in_memory()` and create a session.

#### Test 1: `run_once_without_embedder_succeeds`
- seed observations
- create daemon with `embedder=None`
- assert no error
- assert result fields are reasonable

#### Test 2: `observe_phase_links_entities`
- insert observation mentioning `src/auth.rs` and `AuthService`
- run observe phase
- assert `get_entity("src/auth.rs")` exists

#### Test 3: `capsule_phase_upserts_capsule`
- seed several observations with same topic
- run capsule phase
- assert `list_capsules(Some(project))` is non-empty

#### Test 4: `review_phase_bootstraps_reviews`
- seed high-access observations
- run review phase
- assert `get_pending_reviews(Some(project), 100)` returns rows

#### Test 5: `anti_pattern_phase_persists_findings_if_enabled`
- seed hotspot/decision data
- run anti-pattern phase
- assert expected stored observation or count

#### Test 6: `serve_sidecar_can_start_without_crashing`
- maybe integration-light: construct daemon in thread with in-memory store and one-shot config
- don’t need full HTTP bind in unit test

### Local verification commands
Run in order before push:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

For faster inner loop during implementation:

```bash
cargo test -p engram-rust learn_daemon
cargo test -p engram-learn
cargo check -p engram-rust
```

---

## 11. Recommended implementation sequence

### Task 1
Create `src/learn_daemon.rs` with config/result structs and `LearnDaemon::new()`.

### Task 2
Implement `run_once()` skeleton that calls phase helpers and returns `LearnTickResult`.

### Task 3
Implement Observe phase:
- entity extraction
- topic observation helper
- optional salience computation

### Task 4
Implement Consolidate phase by delegating to `ConsolidationEngine`.

### Task 5
Implement Evolve phase by delegating to `GraphEvolver`.

### Task 6
Implement Capsule phase:
- discover candidate topics
- `build_capsule`
- `upsert_capsule`

### Task 7
Implement Review phase:
- bootstrap schedules if sparse
- persist via `upsert_review`

### Task 8
Implement AntiPattern phase:
- detect all
- optionally persist finding observations

### Task 9
Implement Inject phase:
- build `InjectionContext`
- optionally persist snapshot observation

### Task 10
Add `run_loop()`.

### Task 11
Wire new `Learn` subcommand into `src/main.rs`.

### Task 12
Extend `Serve` with `--learn-daemon` and `--learn-interval`.

### Task 13
Add tests.

### Task 14
Run trifecta, then push, then validate GitHub CI, then rebuild local binary and restart the running Hermes MCP/API service.

---

## 12. Definition of done

The feature is done when all of these are true:
- `engram learn --once` runs successfully on a real DB
- `engram learn --daemon --interval 60` loops without crashing
- `engram serve --learn-daemon --learn-interval 60` starts API + background learner
- capsules are actually persisted
- review schedules are actually persisted
- anti-pattern detection can run without bringing down the daemon
- local trifecta passes
- GitHub CI passes on ubuntu/windows/macos
- local binary used by Hermes is rebuilt and restarted
