# Tasks: engram-rust v2 — Persistent Memory for AI Agents

**Proyecto:** engram-rust  
**Lenguaje:** Rust (edition 2024)  
**Stack:** rmcp 1.3+, rusqlite bundled, fastembed, sqlite-vec, axum, ratatui  
**Fases:** 3 (Core → Search+Learning → Production)  
**Innovaciones:** 10 (Episodic-Semantic, Salience, Spaced Repetition, Multi-Agent, Multimodal, Metacognitive, Temporal Graph, MCP Resources, Personality, Streaming)

---

## Dependencia entre fases

```
F1.1 ─→ F1.2 ─→ F1.3 ─┬─→ F1.4 ─→ F1.5 ─┬─→ F1.6
                       │                   └─→ F1.7
                       └─→ F2.1 ─→ F2.2 ─→ F2.3 ─→ F2.4
                                                   │
                                                   ▼
                                              F2.5 (Auto-Learning)
                                                   │
                                              F2.75 (Smart Context)
                                                   │
                                              F3.1 (HTTP API)
                                                   │
                                          ┌────────┼────────┐
                                          ▼        ▼        ▼
                                        F3.2    F3.3    F3.5
                                        (TUI)   ──→ F3.4 (Cifrado)
                                                (Sync ──→ F3.6 Multi-Agent)
                                                ──→ F3.7 Multimodal)
                                                ──→ F3.8 Streaming)
```

---

## FASE 1 — Core + Store + Paridad Funcional

_Objetivo: Binario que hace TODO lo que Engram Go hace, con tests._

---

### F1.1 — Workspace Setup + Core Types

- [x] F1.1.1 — Crear `Cargo.toml` workspace con `members = ["crates/*"]`, `resolver = "3"`, shared deps en `[workspace.dependencies]`
- [x] F1.1.2 — Crear `crates/core/src/observation.rs`: `Observation` struct (todos los campos incluidos `access_count`, `last_accessed`, `pinned`), `ObservationType` enum estricto (Bugfix, Decision, Architecture, Pattern, Discovery, Learning, Config, Convention, ToolUse, FileChange, Command, FileRead, Search, Manual), `Scope` enum (Project, Personal), Display impl snake_case compatible con Go
- [x] F1.1.3 — Crear `crates/core/src/session.rs`: `Session`, `SessionSummary` structs
- [x] F1.1.4 — Crear `crates/core/src/topic.rs`: `fn suggest_topic_key(obs_type, title) -> String`, `fn slugify(text) -> String`, family heuristics
- [x] F1.1.5 — Crear `crates/core/src/error.rs`: `EngramError` enum (Database, NotFound, Duplicate, TooLong, InvalidTopicKey, Sync, Embedding, Config)
- [x] F1.1.6 — Crear `crates/core/src/graph.rs`: `RelationType` enum (CausedBy, RelatedTo, Supersedes, Blocks, PartOf), `Edge` struct con `weight`, `valid_from`, `valid_until`, `superseded_by` (columnas temporales desde el inicio)
- [x] F1.1.7 — Crear `crates/core/src/score.rs`: `fn decay_score(created_at, access_count, pinned) -> f64`, half-life 30 días, diseñar para extensión futura por salience
- [x] F1.1.8 — Crear `crates/core/src/lib.rs` con re-exports de todos los módulos

**DoD F1.1:** `cargo check` pasa, todos los tipos compilan, unit tests de scoring y topic_key

---

### F1.2 — Storage Trait

`→ F1.1`

- [x] F1.2.1 — Definir `crates/store/src/trait.rs`: `Storage` trait con operaciones CRUD observations/sessions/search/timeline/stats/prompts/graph/embeddings/export/import/capsules/cross-project/episodic-semantic/review/boundaries/lifecycle/beliefs/entities/attachments/personalities. **Audit crítico:** cada método retorna tipos de `crates/core`, parámetros como structs propios, `Result<T, EngramError>`. NADA de `raw_query` o `get_connection`.
- [x] F1.2.2 — Definir structs de parámetros: `AddObservationParams`, `UpdateObservationParams`, `SearchOptions`, `AddPromptParams`, `AddEdgeParams` con Default impl
- [x] F1.2.3 — Crear `ExportData`, `ImportResult` structs JSON-compatible con Engram Go

**DoD F1.2:** Trait compila, pasa audit de abstracción (zero rusqlite leaks)

---

### F1.3 — SQLite Implementation + Migrations

`→ F1.2`

- [x] F1.3.1 — Crear `crates/store/Cargo.toml`: rusqlite (bundled, serde_json), tokio, sha2
- [x] F1.3.2 — Implementar migration runner: `Migration` struct con version + SQL embebido via `include_str!`, tabla `_migrations` tracking
- [x] F1.3.3 — Migration 001: schema base (sessions, observations con `access_count`/`last_accessed`/`pinned`, prompts, índices)
- [x] F1.3.4 — Migration 002: FTS5 virtual table `observations_fts` con triggers ai/ad/au
- [x] F1.3.5 — Implementar `SqliteStore::new()`: WAL mode, busy_timeout=5000, synchronous=NORMAL, foreign_keys=ON, ejecutar migrations
- [x] F1.3.6 — Implementar CRUD observations: insert (SHA-256 dedup, upsert por topic_key, 15min window), get (increment access_count), update, delete (soft/hard)
- [x] F1.3.7 — Implementar CRUD sessions: create (UUID v4), end, context (últimas N observations)
- [x] F1.3.8 — Implementar prompts + stats operations
- [x] F1.3.9 — Implementar export/import con round-trip test contra fixture JSON de Go

**DoD F1.3:** Unit tests CRUD pass, export/import round-trip con Go format

---

### F1.4 — FTS5 Search (Baseline)

`→ F1.3`

- [x] F1.4.1 — Implementar `search()` con FTS5 MATCH + filtros type/project/scope + rank ordering
- [x] F1.4.2 — Implementar query normalization: escape FTS5 operators especiales
- [x] F1.4.3 — Tests de búsqueda (5+ test cases: keyword exact, partial, multi-word, filtered, empty)

**DoD F1.4:** Búsqueda relevante sobre datos de prueba

---

### F1.5 — MCP Server (Paridad con Go — 15 tools)

`→ F1.3, → F1.4`

- [x] F1.5.1 — Setup `crates/mcp/Cargo.toml` con rmcp 1.3+ (features: server, macros, transport-io, schemars)
- [x] F1.5.2 — Definir `EngramServer` struct (store Arc<dyn Storage>, config, tool_allowlist)
- [x] F1.5.3 — Implementar `mem_save` (dedup hash, topic_key upsert)
- [x] F1.5.4 — Implementar `mem_search` (FTS5 baseline, hybrid en F2.2)
- [x] F1.5.5 — Implementar `mem_context` (últimas N observations del proyecto)
- [x] F1.5.6 — Implementar `mem_session_summary` (resumen de sesión)
- [x] F1.5.7 — Implementar `mem_session_start` + `mem_session_end`
- [x] F1.5.8 — Implementar `mem_get_observation` (incrementa access_count)
- [x] F1.5.9 — Implementar `mem_update` (actualización parcial)
- [x] F1.5.10 — Implementar `mem_delete` (soft/hard)
- [x] F1.5.11 — Implementar `mem_suggest_topic_key`
- [x] F1.5.12 — Implementar `mem_timeline`
- [x] F1.5.13 — Implementar `mem_save_prompt`
- [x] F1.5.14 — Implementar `mem_stats`
- [x] F1.5.15 — Implementar `mem_merge_projects` (admin only)
- [x] F1.5.16 — Implementar `mem_capture_passive` (heurística de patrones)
- [x] F1.5.17 — Implementar tool profiles (agent=11, admin=4, all=15)
- [x] F1.5.18 — MCP annotations (title, read_only_hint, destructive_hint)

**DoD F1.5:** 15 tools registrados y funcionales, profiles operativos

---

### F1.6 — CLI Entrypoint + Integration Tests

`→ F1.5`

- [x] F1.6.1 — Setup `src/main.rs` con clap derive: mcp, search, save, timeline, context, stats, export, import, export-context, session-start, session-end, version
- [x] F1.6.2 — Subcommand `mcp` (SqliteStore → EngramServer → rmcp stdio via serve_stdio)
- [x] F1.6.3 — Subcommand `search` (FTS5 search con filtros type/limit)
- [x] F1.6.4 — Subcommand `save` (insert observation con todos los campos)
- [x] F1.6.5 — Subcommands `stats`, `context`, `timeline`
- [x] F1.6.6 — Subcommands `export`/`import` (JSON compatible con Go)
- [x] F1.6.7 — Subcommand `export-context` (genera Markdown ~2000 tokens: top observations accedidas, stats overview, by-type breakdown). Killer feature — funciona solo con SQLite, no requiere F2+
- [x] F1.6.8 — Integration tests: flujo completo store (session → obs → search → timeline → export → import → export-context)
- [x] F1.6.9 — Integration tests: dedup, soft_delete, prompts, graph_edges, type_filter

**DoD F1.6:** Todos los subcommands funcionan, integration tests pass

---

### F1.7 — Plugin Installers

`→ F1.5`

- [x] F1.7.1 — `engram setup [agent]` para claude-code, cursor, gemini-cli, opencode
- [x] F1.7.2 — SKILL.md template con Memory Protocol

**DoD F1.7:** `engram-rust setup claude-code` configura el agente correctamente

---

## FASE 2 — Búsqueda Semántica + Grafo + Auto-Aprendizaje

_Objetivo: Búsqueda que entiende significado + relaciones + sistema que consolida, sintetiza, detecta patrones._

---

### F2.1 — Embedder Integration

`→ F1.3`

- [x] F2.1.1 — Setup `crates/search/Cargo.toml` con fastembed (all-MiniLM-L6-v2, 384d)
- [x] F2.1.2 — Implementar `Embedder` con versionado de modelo: `EmbeddingMeta` (model_name, model_version, dimensions, created_at). Model drift detection al inicializar: si hay embeddings con version diferente → warning + fallback a FTS5-only. NO auto-reembed (costoso).
- [x] F2.1.3 — Auto-embedding en `mem_save`: embed `title + "\n" + content`, guardar con `model_name` + `model_version`
- [x] F2.1.4 — Tool `mem_reembed`: detecta automáticamente embeddings stale, progreso streaming, fallback FTS5 durante reembed
- [x] F2.1.5 — Tests de similaridad (cosine: identical, orthogonal, opposite, empty + model drift detection)

**DoD F2.1:** Embedder genera vectores 384d, detecta drift, advierte correctamente

---

### F2.2 — Vector Store + Hybrid Search

`→ F2.1, → F1.3`

- [x] F2.2.1 — Migration 003: `observation_vecs` (vec0 float[384]) + `embedding_meta` table (model_name, model_version)
- [x] F2.2.2 — Implementar `store_embedding()` y `search_vector()` en SqliteStore
- [x] F2.2.3 — Implementar `HybridSearch` con Reciprocal Rank Fusion (k=60, fts_weight=0.4, vector_weight=0.6)
- [x] F2.2.4 — Integrar en `mem_search`: hybrid por default, fallback FTS5 si embedder unavailable
- [x] F2.2.5 — Tests de hybrid search ("performance issue" encuentra "N+1 query fix")

**DoD F2.2:** Hybrid search funciona, >20% improvement sobre FTS5-only

---

### F2.3 — Graph Relations (Temporal)

`→ F1.3, → F1.5`

- [x] F2.3.1 — Migration 004: tabla `edges` (source_id, target_id, relation, weight, valid_from, valid_until, superseded_by, auto_detected, UNIQUE constraint, índice en valid_from)
- [x] F2.3.2 — Graph operations en SqliteStore: `add_edge` (auto-cierra anterior), `get_edges` (solo vigentes), `get_related` (BFS/DFS), `get_edges_at` (temporal)
- [x] F2.3.3 — Tool `mem_relate` (crea relación entre observations)
- [x] F2.3.4 — Tool `mem_graph` (JSON de grafo, solo vigentes por default, flag `include_historical`)
- [x] F2.3.5 — Enriquecer `mem_search` con graph context (1-2 relaciones por resultado)
- [x] F2.3.6 — Auto-detección de relaciones (referencia a IDs, bugfix→decision correlation)
- [x] F2.3.7 — Tests de grafo (integration tests graph_edges_temporal pass)

**DoD F2.3:** Grafo temporal funciona, edges se cierran automáticamente

---

### F2.4 — Relevance Scoring

`→ F2.2, → F1.1`

- [x] F2.4.1 — `compute_final_score`: 0.3*fts + 0.3*vector + 0.2*recency + 0.2*frequency (diseñar para extensión por salience)
- [x] F2.4.2 — Tool `mem_pin` (toggle pinned column, pinned = score 1.0 recency)
- [x] F2.4.3 — `access_count` increment en reads
- [x] F2.4.4 — Tests de scoring (pinned > unpinned, frequent > rare, recent > old)

**DoD F2.4:** Scoring compuesto funciona, pinned tiene prioridad

---

### F2.5.1 — Confidence + Provenance Tracking

`→ F1.3`

- [x] F2.5.1.1 — Crear `crates/core/src/provenance.rs`: `ProvenanceInfo` (source, confidence, evidence), `ProvenanceSource` enum (TestVerified=0.95, CodeAnalysis=0.85, UserStated=0.70, External=0.65, LlmReasoning=0.60, Inferred=0.40)
- [x] F2.5.1.2 — Migration 005: columnas `provenance_source`, `provenance_confidence`, `provenance_evidence` en observations
- [x] F2.5.1.3 — Extender `mem_save` con parámetros de provenance (input opcional, default LlmReasoning)
- [x] F2.5.1.4 — Extender `mem_search` con filtro `min_confidence`
- [x] F2.5.1.5 — Auto-provenance en `mem_capture_passive` (detectar "test passed"→TestVerified, "changed in src/"→CodeAnalysis)
- [x] F2.5.1.6 — Tests de provenance (3+ cases)

**DoD F2.5.1:** Provenance se guarda, filtra, e infiere automáticamente

---

### F2.5.2 — Consolidation Engine

`→ F2.5.1, → F2.1, → F2.3`

- [x] F2.5.2.1 — Crear `crates/learn/src/consolidation.rs`: `ConsolidationEngine<S: Storage>`, `ConsolidationResult` struct con métricas (duplicates_merged, obsolete_marked, conflicts_found, patterns_extracted)
- [x] F2.5.2.2 — Implementar detección de duplicados semánticos: pairs con cosine_similarity > 0.92, soft-delete la redundante
- [x] F2.5.2.3 — Implementar detección de obsolescencia: observations con edge `supersedes` → marca como stale
- [x] F2.5.2.4 — Implementar detección de contradicciones: mismo topic_key con decisiones sin supersedes → conflict flag
- [x] F2.5.2.5 — Implementar extracción de patrones: 3+ bugfixes similares (cosine > 0.8) → crear observation tipo "pattern" con provenance Inferred
- [x] F2.5.2.6 — Implementar `run_consolidation()` orquestador: duplicates → obsolete → contradictions → patterns
- [x] F2.5.2.7 — Implementar `mem_consolidate` MCP tool (input: project?, dry_run?)
- [x] F2.5.2.8 — Implementar `engram-rust consolidate` CLI command
- [x] F2.5.2.9 — Implementar auto-consolidación: `tokio::spawn` con intervalo, trigger cuando observations > 500 o cada N horas
- [x] F2.5.2.10 — Tests de consolidación (5 cases: empty_project, marks_obsolete, finds_conflicts, no_embedder_skips, result_display)

**DoD F2.5.2:** Consolidation engine limpia la base automáticamente

---

### F2.5.3 — Knowledge Capsules

`→ F2.5.2`

- [x] F2.5.3.1 — Crear `crates/core/src/capsule.rs`: `KnowledgeCapsule` struct (topic, project, summary, key_decisions, known_issues, anti_patterns, best_practices, source_observations, confidence, version, to_markdown())
- [x] F2.5.3.2 — Migration 006: tabla `knowledge_capsules` con UNIQUE(topic, project)
- [x] F2.5.3.3 — Definir trait `CapsuleSynthesizer` (synthesize, can_synthesize, name). Implementar `HeuristicSynthesizer` (MVP, siempre disponible), `ChainedSynthesizer` (primary → fallback). `CapsuleBuilder` usa synthesizer + Storage.
- [x] F2.5.3.4 — Implementar re-consolidación de capsules: `fn rebuild(capsule_id)` incrementa version, recalcula con nuevas observations
- [x] F2.5.3.5 — Tool `mem_synthesize` (input: topic, project? → genera/actualiza capsule)
- [x] F2.5.3.6 — Tool `mem_capsule_list` (input: project? → lista con topic + confidence + version)
- [x] F2.5.3.7 — Tool `mem_capsule_get` (input: topic, project? → capsule completa formateada)
- [x] F2.5.3.8 — Integrar en `mem_search`: si >5 matches mismo topic → sugerir capsule
- [x] F2.5.3.9 — Integrar auto-síntesis en consolidación
- [x] F2.5.3.10 — Tests de capsules (4 cases: produces_capsule, always_available, chained_fallback, no_observations_errors)
- [x] F2.5.3.9 — Integrar en consolidación: auto-crear si topic tiene >10 observations sin capsule; auto-rebuild si >5 nuevas desde last_consolidated
- [x] F2.5.3.10 — Tests de capsules (4+ cases: build, rebuild, confidence, search suggestion)

**DoD F2.5.3:** Capsules se crean, evolucionan, y se sugieren automáticamente

---

### F2.5.4 — Anti-Pattern Detection

`→ F2.5.2, → F2.3`

- [x] F2.5.4.1 — Crear `crates/core/src/antipattern.rs` → moved to `crates/learn/src/anti_pattern.rs`: `AntiPattern` struct, `AntiPatternType` enum (RecurringBug, RevertPattern, HotspotFile, UnverifiedDecision), `Severity` enum
- [x] F2.5.4.2 — Implementar `AntiPatternDetector`: detectores para cada tipo (recurring bugs por cosine similarity, revert patterns por cycles en grafo, unverified decisions por provenance baja, hotspot files por count)
- [x] F2.5.4.3 — Tool `mem_antipatterns` (input: project?, severity? → lista con evidencia y sugerencia)
- [x] F2.5.4.4 — Integrar anti-patterns en `mem_context`: warnings al inicio de sesión
- [x] F2.5.4.5 — Integrar anti-patterns en `mem_save`: advertir al guardar bugfix recurrente
- [x] F2.5.4.6 — Tests de anti-pattern detection (4 cases: no_embedder, unverified_decisions, hotspot_files, display)

**DoD F2.5.4:** Anti-patterns se detectan y advierten automáticamente

---

### F2.5.5 — Auto Graph Evolution

`→ F2.3, → F2.1`

- [x] F2.5.5.1 — Crear `crates/learn/src/graph_evolver.rs`: `GraphEvolver<S: Storage>`, `NewEdge` struct, `EvolutionResult`
- [x] F2.5.5.2 — Implementar detección de correlación temporal: A precede a B en 3+ sesiones → edge `CausedBy`
- [x] F2.5.5.3 — Implementar detección de co-ocurrencia en búsquedas: X y Y aparecen juntas en 3+ searches → edge `RelatedTo` (requiere tabla `search_log`)
- [x] F2.5.5.4 — Implementar detección de correlación por archivos: dos observations afectan mismo archivo → `RelatedTo`
- [x] F2.5.5.5 — Implementar detección de clusters semánticos: cosine_similarity > 0.85 → `RelatedTo` (requiere embedder)
- [x] F2.5.5.6 — Implementar `evolve()` orquestador: ejecutar 3 detectores (temporal + file + semantic), insertar edges con `auto_detected=true`
- [x] F2.5.5.7 — Integrar en consolidación: después de consolidate → evolve
- [x] F2.5.5.8 — Tests de auto graph evolution (5 cases: evolver_no_embedder, detect_temporal, detect_file, display, is_file_path)

**DoD F2.5.5:** Grafo evoluciona solo con relaciones detectadas automáticamente

---

### F2.5.6 — Episodic-Semantic Separation [Innovación 1]

`→ F1.3, → F2.5.2`

- [x] F2.5.6.1 — Crear `crates/core/src/memory.rs`: `MemoryType` enum, `EpisodicMemory` (session_id, timestamp, what_happened, context, emotional_valence, surprise_factor), `SemanticMemory` (knowledge, domain, confidence, source_episodes, last_validated), `EpisodicContext`, `QueryTarget`, `classify_query_type()`
- [x] F2.5.6.2 — Migration 008: tablas `episodic_memories` y `semantic_memories` (FK a observations)
- [x] F2.5.6.3 — Implementar conversión episódico→semántico en ConsolidationEngine
- [x] F2.5.6.4 — `classify_query_type()`: heurística episodic/semantic/both con word boundary matching
- [x] F2.5.6.5 — Integrar en `mem_save`: flag opcional `memory_type`
- [x] F2.5.6.6 — Tests de episodic-semantic (7 cases: roundtrip, invalid, episodic queries, semantic queries, generic queries, semantic_from_episode)

**DoD F2.5.6:** Separación funciona, búsqueda es type-aware, episodios se consolidan a semánticos

---

### F2.5.7 — Emotional Salience [Innovación 2]

`→ F2.5.6, → F1.1`

- [x] F2.5.7.1 — Crear `crates/core/src/salience.rs`: `MemorySalience` (emotional_valence -1.0..1.0, surprise_factor 0.0..1.0, effort_invested 0.0..1.0), `decay_multiplier()` formula
- [x] F2.5.7.2 — Implementar `crates/learn/src/salience_infer.rs`: `fn infer_salience(content, session_length) -> MemorySalience` (keyword heuristic: frustration/achievement/surprise/effort)
- [x] F2.5.7.3 — `decay_multiplier()`: `(1.0 + valence * 0.3) * (1.0 + surprise * 0.5)`, clamped to min 0.1
- [x] F2.5.7.4 — Extender `mem_capture_passive` para inferir salience automáticamente
- [x] F2.5.7.5 — Columnas emotional_valence, surprise_factor, effort_invested ya existen en observations (migration 001)
- [x] F2.5.7.6 — Tests de salience (11 cases: 6 core + 5 learn)

**DoD F2.5.7:** Salience se infiere y modifica decay correctamente

---

### F2.5.8 — Metacognitive Boundaries [Innovación 6]

`→ F2.5.2, → F2.5.4`

- [x] F2.5.8.1 — Crear `crates/core/src/boundary.rs`: `KnowledgeBoundary` (domain, confidence_level Expert/Proficient/Familiar/Aware/Unknown, evidence, record_success/failure, recalculate, format_for_context)
- [x] F2.5.8.2 — Migration 012: tabla `knowledge_boundaries` (domain PK, confidence_level, evidence JSON)
- [x] F2.5.8.3 — Implementar `crates/learn/src/boundary_tracker.rs`: `BoundaryTracker` con `compute_boundaries()` y `get_relevant_boundaries()`
- [x] F2.5.8.4 — Integrar en SmartInjector
- [x] F2.5.8.5 — Tool `mem_knowledge_boundary`
- [x] F2.5.8.6 — Spaced repetition alimenta boundaries
- [x] F2.5.8.7 — Tests de metacognitive boundaries (9 cases: 5 core + 4 learn)

**DoD F2.5.8:** Sistema sabe en qué es experto y en qué no

---

### F2.5.9 — Observation Lifecycle

`→ F2.5.2, → F1.1`

- [x] F2.5.9.1 — Crear `crates/core/src/lifecycle.rs`: `LifecyclePolicy` (por ObservationType con todos los campos), `for_type()`, `all_defaults()`
- [x] F2.5.9.2 — Políticas default: Decision (permanente, decay 0.5x), Bugfix (90d stale, no auto-delete), Command (30d/180d auto-purga), Architecture (permanente, decay 0.3x), FileRead/Search (14d/90d ephemeral, decay 2.0x)
- [x] F2.5.9.3 — lifecycle_state ya existe en migration 001 (columna + índice)
- [x] F2.5.9.4 — Implementar transiciones en ConsolidationEngine
- [x] F2.5.9.5 — Integrar en search con include_stale flag
- [x] F2.5.9.6 — Integrar decay multiplier en score
- [x] F2.5.9.7 — Configurable via engram.toml
- [x] F2.5.9.8 — Tests de lifecycle (6 cases)

**DoD F2.5.9:** Lifecycle transiciones automáticas, configurable por usuario

---

### F2.5.10 — Belief Resolution

`→ F2.5.1, → F2.5.2`

- [x] F2.5.10.1 — Crear `crates/core/src/belief.rs`: `Belief`, `HistoricalBelief`, `BeliefState` enum, `BeliefOperation` enum, `process_evidence()`, `execute_operation()`
- [x] F2.5.10.2 — Migration 015: tabla `beliefs`
- [x] F2.5.10.3 — Belief state machine: Active→Confirmed (>0.9, 3+ sources), Contest (similar confidence), Update (stronger evidence, preserves history), Retract (user correction)
- [x] F2.5.10.4-8 — Integrations (mem_save, mem_search, consolidación, tool)
- [x] F2.5.10.9 — Tests de belief resolution (8 cases: new, confirms, updates, contests, weaker_ignored, preserves_history, becomes_confirmed, contested_state)

**DoD F2.5.10:** Beliefs se resuelven automáticamente, historial preservado

---

### F2.5.11 — Memory Compaction

`→ F2.5.2, → F2.5.3, → F2.1`

- [x] F2.5.11.1 — Crear `crates/core/src/compaction.rs`: `CompactionLevel` enum (Raw/Fact/Pattern/Principle), `determine_level()` query classifier
- [x] F2.5.11.2-5 — Stages Raw→Fact→Pattern→Principle pipeline
- [x] F2.5.11.6-7 — SmartInjector integration + mem_principles tool
- [x] F2.5.11.8 — Tests de memory compaction (4 cases: specific→fact, trend→pattern, big_picture→principle, default→fact)

**DoD F2.5.11:** Compaction por niveles de abstracción funciona

---

### F2.5.12 — Entity Resolution

`→ F2.3, → F2.1`

- [x] F2.5.12.1 — Crear `crates/core/src/entity.rs`: `Entity` (canonical_name, aliases, entity_type, properties, `matches()`, `add_alias()`), `EntityType` enum, `extract_entities()` NER heuristic
- [x] F2.5.12.2 — Migration 016: tablas entities, entity_mentions
- [x] F2.5.12.3-7 — EntityRegistry, auto-extraction, entity-aware search, mem_entities tool
- [x] F2.5.12.8 — Tests de entity resolution (7 cases: matches_canonical, matches_alias, no_match, add_alias_no_dup, add_alias_skips_canonical, extract_file, extract_pascal)
- [x] F2.5.12.8 — Tests de entity resolution (5+ cases: Person alias matching, File resolution, Vendor alias, entity-aware search, auto-extraction)

**DoD F2.5.12:** Entities se extraen y resuelven automáticamente, búsqueda robusta a variaciones

---

## FASE 2.75 — Contexto Inteligente

_Objetivo: El sistema inyecta el conocimiento correcto en el momento correcto._

---

### F2.75.1 — Smart Context Injection

`→ F2.2, → F2.5.3, → F2.5.4, → F2.5.8`

- [x] F2.75.1.1 — Crear `crates/learn/src/smart_injector.rs`: `SmartInjector<S: Storage>`, `InjectionContext` (relevant_memories max 5, file_history max 3, knowledge_capsules max 2, warnings, boundaries, review_reminders, total_tokens)
- [x] F2.75.1.2 — Implementar `build_context()`: embed task → vector search 5, file history 3, capsules 2, anti-patterns, boundaries, review reminders. Presupuesto tokens: warnings > boundaries > capsules > review > memories > file_history
- [x] F2.75.1.3 — Formato Markdown de output (legible, denso, estructurado)
- [x] F2.75.1.4 — Tool `mem_inject` (input: task_description, current_files?, project?)
- [x] F2.75.1.5 — Auto-injection en hooks de Claude Code (session-start hook)
- [x] F2.75.1.6 — Tests de smart injection (4+ cases)

**DoD F2.75.1:** Contexto relevante se inyecta automáticamente, dentro de presupuesto de tokens

---

### F2.75.2 — Cross-Project Learning

`→ F2.5.3, → F2.2, → F2.75.5`

- [x] F2.75.2.1 — Migration 007: índice cross-project en capsules + tabla `knowledge_transfers` (source_project, target_project, capsule_id, relevance_score, accepted)
- [x] F2.75.2.2 — Crear `crates/learn/src/cross_project.rs`: `CrossProjectLearner<S: Storage>`, `KnowledgeTransfer` (source_project, capsule, relevance, transfer_type DirectApplicable/Analogous/AntiPattern, style_compatibility)
- [x] F2.75.2.3 — Implementar `suggest_prior_knowledge()`: embed initial_context, buscar en TODAS las capsules de TODOS los proyectos, rank por cosine, filtrar confidence > 0.7, ajustar por style_compatibility
- [x] F2.75.2.4 — Tool `mem_transfer` (input: project, context → transfers sugeridos)
- [x] F2.75.2.5 — Auto-transfer en `mem_session_start`: si proyecto tiene <10 observations → sugerir transfers
- [x] F2.75.2.6 — Transfer acceptance tracking en `knowledge_transfers`
- [x] F2.75.2.7 — Tests de cross-project (3+ cases)

**DoD F2.75.2:** Conocimiento de proyectos viejos se sugiere en nuevos

---

### F2.75.3 — Spaced Repetition [Innovación 3]

`→ F2.5.6, → F2.75.1`

- [x] F2.75.3.1 — Crear `crates/core/src/spaced.rs`: `SpacedRepetition` (memory_id, interval_days, ease_factor 2.5 default, next_review, review_count, last_result), `ReviewResult` enum (Perfect/Good/Hard/Forgotten)
- [x] F2.75.3.2 — Migration 009: tabla `review_schedule` (memory_id FK, interval, ease, next_review, índice en next_review)
- [x] F2.75.3.3 — Implementar `crates/learn/src/spaced_review.rs`: SM-2 lógica (Perfect: interval*=ease, Good: interval*=1.2, Hard/Forgotten: interval=1d, Forgotten: ease-=0.2), `get_pending_reviews`, `schedule_review`
- [x] F2.75.3.4 — Cold start bootstrap: si review_schedule vacío, seleccionar top 50 observations más accedidas, programar con intervals distribuidos (top 10: 3d, 11-30: 1d, 31-50: 0.5d)
- [x] F2.75.3.5 — Detección implícita de ReviewResult: agente busca memory y la usa correctamente → Good/Perfect; busca pero no la usa → Hard/Forgotten
- [x] F2.75.3.6 — Integrar en SmartInjector: pending reviews como "refresh reminders"
- [x] F2.75.3.7 — Integrar con KnowledgeBoundary: ReviewResult alimenta successful/failed applications
- [x] F2.75.3.8 — Tests de spaced repetition (5+ cases: schedule, review, cold start, implicit detection, smart injection)

**DoD F2.75.3:** Sistema de revisión periódica funciona, cold start bootstrapea automáticamente

---

### F2.75.4 — MCP Resources [Innovación 8]

`→ F2.75.1, → F2.5.3, → F2.5.4`

- [x] F2.75.4.1 — Implementar `crates/mcp/src/resources.rs`: `list_resources` (3 resources estándar), `read_resource` (resuelve URI → contenido formateado)
- [x] F2.75.4.2 — Sistema de notificaciones granulares: current-context batch cada N min, knowledge-capsules notify al consolidar, anti-patterns notify al detectar
- [x] F2.75.4.3 — Implementar `ServerHandler` para `list_resources` y `read_resource`
- [x] F2.75.4.4 — URIs: `engram://project/current-context`, `engram://project/knowledge-capsules`, `engram://project/anti-patterns`
- [x] F2.75.4.5 — Tests de MCP resources (3+ cases)

**DoD F2.75.4:** Resources listables y legibles, notificaciones no ruidosas

---

### F2.75.5 — Agent Personality [Innovación 9]

`→ F2.5.2, → F2.75.2`

- [x] F2.75.5.1 — Crear `crates/core/src/personality.rs`: `AgentPersonality` (agent_id, project, working_style, preferences, strengths, weaknesses), `WorkingStyle`, `Preferences`. Weaknesses por fracaso (anti-patterns), NO por ausencia.
- [x] F2.75.5.2 — Migration 013: tabla `agent_personalities` (agent_id + project PK compuesto, campos JSON)
- [x] F2.75.5.3 — Implementar `crates/learn/src/personality_analyzer.rs`: `fn analyze(agent_id, project) -> AgentPersonality`, análisis de patrones en observations
- [x] F2.75.5.4 — Integrar en CrossProjectLearner: `style_compatibility` afecta ranking de transfers
- [x] F2.75.5.5 — Tests de agent personality (3+ cases)

**DoD F2.75.5:** Perfil de agente se genera y afecta cross-project transfers

---

## FASE 3 — API, TUI, Sync, Cifrado, Innovaciones Avanzadas

_Objetivo: Production-ready + features de alto impacto._

---

### F3.1 — HTTP REST API

`→ F1.5`

- [x] F3.1.1 — Setup `crates/api/Cargo.toml` con axum 0.8 + tower-http (cors)
- [x] F3.1.2 — Routes: GET/POST/PUT/DELETE /observations, POST /search, GET /stats, POST /sessions, GET /graph/:id, GET /capsules, POST /consolidate, POST /inject
- [x] F3.1.3 — `engram serve [port]` (default 7437)
- [x] F3.1.4 — Tests de API (5+ endpoints)

**DoD F3.1:** API HTTP funcional, P95 latency <50ms

---

### F3.2 — TUI (Terminal UI)

`→ F1.3`

- [x] F3.2.1 — Setup con ratatui 0.29 + crossterm 0.28
- [x] F3.2.2 — App state machine (Dashboard, Search, Detail, Timeline, Capsules, AntiPatterns, Boundaries)
- [x] F3.2.3 — Dashboard view (observations recientes, stats, type badges)
- [x] F3.2.4 — Search view (input + results + highlight)
- [x] F3.2.5 — Detail view (contenido + metadata + graph relations + temporal history)
- [x] F3.2.6 — Capsules view (lista de capsules, drill-down)
- [x] F3.2.7 — AntiPatterns view (warnings activos)
- [x] F3.2.8 — Boundaries view (mapa de conocimiento del agente)
- [x] F3.2.9 — `engram tui` subcommand

**DoD F3.2:** TUI interactiva con todas las views funcionales

---

### F3.3 — Chunk Sync (Compatibilidad Go)

`→ F1.3`

- [x] F3.3.1 — Chunk export (JSONL gzip, SHA-256 ID, manifest.json)
- [x] F3.3.2 — Chunk import (descomprimir, INSERT OR IGNORE)
- [x] F3.3.3 — Test bidireccional de compatibilidad JSON con Go
- [x] F3.3.4 — `engram sync --status`

**DoD F3.3:** Chunk sync bidireccional con Engram Go

---

### F3.4 — CRDT Sync (P2P)

`→ F3.3`

- [x] F3.4.1 — LWW-Element-Set manual (observations son registros independientes)
- [x] F3.4.2 — CRDT state (device_id, vector_clock, persistido en `~/.engram/crdt_state.json`)
- [x] F3.4.3 — Sync via directorio compartido (Dropbox/iCloud — cada device escribe delta JSON.gz)
- [x] F3.4.4 — Conflict resolution (LWW por updated_at, loser en tabla `_conflicts`)
- [x] F3.4.5 — Tests de CRDT sync (4 scenarios: insert, update, sync, conflict)

**DoD F3.4:** CRDT sync converge en <5s entre 2 dispositivos

---

### F3.5 — Cifrado at Rest

`→ F1.3`

- [x] F3.5.1 — Chacha20Poly1305 sobre archivo (no SQLCipher — más simple, compatible con bundled SQLite)
- [x] F3.5.2 — `EncryptedStore` wrapper (Argon2id key derivation, passphrase de env var o OS keyring)
- [x] F3.5.3 — `engram encrypt` / `engram decrypt` para migration
- [x] F3.5.4 — Flag `--encrypt` en CLI, auto-detección de DB cifrada

**DoD F3.5:** Cifrado transparente, auto-detección funciona

---

### F3.6 — Multi-Agent Shared Memory [Innovación 4]

`→ F3.4, → F2.5.8`

- [x] F3.6.1 — Completar `crates/core/src/permissions.rs`: `SharedMemoryService`, `PermissionEngine`, `PermissionRule`, `AccessLevel` (Read/Write/Admin), `Scope` filter
- [x] F3.6.2 — Migration 010: tabla `agent_permissions` (agent_id, project, access_level, scope_filter)
- [x] F3.6.3 — Tool `mem_share` (marca observation como scope=project, verifica permisos)
- [x] F3.6.4 — Tool `mem_team_capsule` (knowledge capsule del equipo, agrega fuentes de múltiples agentes)
- [x] F3.6.5 — Tool `mem_agent_status` (qué sabe cada agente: boundaries + stats por agent_id)
- [x] F3.6.6 — Integrar con CRDT sync: permisos se respetan al merge
- [x] F3.6.7 — Tests de multi-agent (3+ cases: share, read permissions, team capsule)

**DoD F3.6:** Multi-agent sharing funciona con permisos

---

### F3.7 — Multimodal Memory [Innovación 5]

`→ F1.3, → F2.1`

- [x] F3.7.1 — Crear `crates/core/src/attachment.rs`: `Attachment` enum (CodeDiff, TerminalOutput, ErrorTrace, GitCommit), `MultimodalObservation`. TerminalOutput se trunca (últimas N líneas + hash).
- [x] F3.7.2 — Migration 011: tabla `observation_attachments` (FK observations, type, content JSON)
- [x] F3.7.2b — Storage: store_attachment() + get_attachments() en Storage trait + SqliteStore
- [x] F3.7.3 — Extender `mem_save` MCP tool para aceptar attachments opcionales
- [x] F3.7.4 — Auto-captura de git commits via hooks en `plugins/`
- [x] F3.7.5 — Auto-captura de error traces (parseo output cargo test/build)
- [x] F3.7.6 — Embeddings para attachments de texto
- [x] F3.7.7 — Tests multimodal (8 cases: 6 core + 2 store)

**DoD F3.7:** Attachments se almacenan, buscan, y auto-capturan

---

### F3.8 — Memory Streaming [Innovación 10]

`→ F2.2, → F2.5.4, → F2.75.3`

- [x] F3.8.1 — Crear `crates/core/src/stream.rs`: `MemoryEvent` enum (5 tipos), `EventThrottle`
- [x] F3.8.1b — Crear `crates/learn/src/stream_engine.rs`: `MemoryStream` con detect_file_context, detect_anti_pattern_warnings, detect_deja_vu, detect_pending_reviews
- [x] F3.8.2 — Implementar interceptación de tool calls en MCP server
- [x] F3.8.3 — Implementar detección de DejaVu: similarity > 0.85 con solución previa → emitir event
- [x] F3.8.4 — Channel con `tokio::sync::mpsc` para delivery de events
- [x] F3.8.5 — Delivery como notificaciones MCP con throttling (máx 1 event cada 25ms, anti-spam por content hash)
- [x] F3.8.6 — Integrar con spaced repetition: detect_pending_reviews
- [x] F3.8.7 — Tests de memory streaming (5 cases: file_context, anti_pattern, clean_content, empty, pending_reviews)

**DoD F3.8:** Streaming emite events relevantes proactivamente, sin ruido

---

## Resumen MCP Tools

### Fase 1 (15 tools — paridad Go)
| Tool | Perfil |
|------|--------|
| mem_save | agent/core |
| mem_search | agent/core |
| mem_context | agent/core |
| mem_session_summary | agent/core |
| mem_session_start | agent |
| mem_session_end | agent |
| mem_get_observation | agent/core |
| mem_suggest_topic_key | agent |
| mem_capture_passive | agent |
| mem_save_prompt | agent |
| mem_update | agent |
| mem_delete | admin |
| mem_stats | admin |
| mem_timeline | admin |
| mem_merge_projects | admin |

### Fase 2-2.5 (17 tools nuevos)
| Tool | Perfil |
|------|--------|
| mem_relate | agent |
| mem_graph | agent |
| mem_graph_timeline | agent |
| mem_pin | agent |
| mem_reembed | agent |
| mem_consolidate | admin |
| mem_synthesize | agent |
| mem_capsule_list | agent |
| mem_capsule_get | agent |
| mem_antipatterns | agent |
| mem_knowledge_boundary | agent |
| mem_beliefs | agent |
| mem_entities | agent |
| mem_principles | agent |

### Fase 2.75 (3 tools nuevos)
| Tool | Perfil |
|------|--------|
| mem_inject | agent/core |
| mem_transfer | agent |

### Fase 3 (4 tools nuevos)
| Tool | Perfil |
|------|--------|
| mem_share | agent |
| mem_team_capsule | agent |
| mem_agent_status | agent |

### MCP Resources (3 resources)
| URI | Descripción |
|-----|-------------|
| engram://project/current-context | Contexto auto-inyectado |
| engram://project/knowledge-capsules | Capsules del proyecto |
| engram://project/anti-patterns | Anti-patterns activos |

**Total: ~35 tools + 3 resources**

---

## Migraciones SQLite (16 total)

| # | Nombre | Contenido |
|---|--------|-----------|
| 001 | initial | Schema base: sessions, observations, prompts |
| 002 | fts | FTS5 virtual table + triggers |
| 003 | vectors | sqlite-vec + embedding_meta |
| 004 | graph | edges con columnas temporales |
| 005 | provenance | Columnas provenance en observations |
| 006 | capsules | knowledge_capsules |
| 007 | cross_project | knowledge_transfers |
| 008 | episodic_semantic | episodic_memories, semantic_memories, salience columns |
| 009 | review_schedule | review_schedule |
| 010 | agent_permissions | agent_permissions |
| 011 | attachments | observation_attachments |
| 012 | knowledge_boundaries | knowledge_boundaries |
| 013 | agent_personalities | agent_personalities |
| 014 | lifecycle_state | lifecycle_state column + índice |
| 015 | beliefs | beliefs table |
| 016 | entities | entities, entity_mentions, entity_alias_embeddings |

---

## Notas Críticas para Implementación

1. **No saltar fases.** F1 obligatorio antes de F2. F2 antes de F2.5.
2. **export-context es la feature #1 de adopción** (F1.6.7) — funciona solo con SQLite, no requiere F2+.
3. **Storage trait es el firewall** — zero leaks de rusqlite, upgrade path documentado a libSQL.
4. **CapsuleSynthesizer trait es el centro del diseño** — ChainedSynthesizer (LLM→heuristic) como default.
5. **fastembed descarga ~80MB** — tener fallback FTS5 siempre. Documentar.
6. **sqlite-vec es pre-v1** — mantener vector search abstracta detrás del Storage trait.
7. **Embedding model drift es inevitable** — versionar cada embedding, caer a FTS5 si hay mismatch.
8. **Spaced Repetition necesita cold start** — bootstrap al detectar review_schedule vacío.
9. **Auto-graph evolution tiene riesgo de ruido** — empezar con thresholds altos (0.9 similarity, 3+ occurrences).
10. **Lifecycle es configurable** — Decision/Architecture son forever, Command/FileRead se purgan.
11. **Beliefs se resuelven, no se señalan** — state machine Active→Confirmed→Contested→Superseded→Retracted.
12. **Compaction != Consolidation** — compaction sube nivel de abstracción (Raw→Fact→Pattern→Principle).
13. **Entity resolution hace búsqueda robusta** — triple estrategia: vector + FTS + entity lookup.
14. **MCP Resources coexisten con Tools** — Resources = contexto ambiental, Tools = queries específicas.
15. **Streaming intercepta tool calls, no filesystem** — el agente usa herramientas, no edita directamente.
