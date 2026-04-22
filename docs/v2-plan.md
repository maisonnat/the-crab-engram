# The Crab Engram v2.0 — Plan de Implementación Atómico

> Generado: 2026-04-22 | Estado: En ejecución

## Resumen de Estado Actual (v2.1.0)

- 8 crates: core, store, search, learn, mcp, sync, api, tui
- 93 archivos .rs, ~123K líneas
- Edge ya tiene `superseded_by`, `valid_from/valid_until`
- RRF con k=60 implementado
- CRDT básico LWW por-device
- `engram-learn` con 9 módulos heurísticos (sin LLM)
- Embeddings: f32 384-dim con fastembed (AllMiniLML6V2)

---

## Fase 1: Persistencia Bitemporal y Sincronización

### 1.1 — Migración SQL: Bitemporalidad
**Archivos:** `crates/core/src/graph.rs`, `crates/core/src/observation.rs`, `crates/store/src/migration.rs`, `crates/store/src/sqlite.rs`, `crates/store/src/params.rs`

- [ ] 1.1.1 Agregar `recorded_at: DateTime<Utc>` al struct `Edge` (después de `valid_until`)
- [ ] 1.1.2 Agregar `recorded_at: DateTime<Utc>` al struct `Observation` (después de `updated_at`)
- [ ] 1.1.3 Inicializar `recorded_at: Utc::now()` en `Edge::new()` y `Observation::new()`
- [ ] 1.1.4 Crear migración `017_bitemporal.sql`: `ALTER TABLE edges ADD COLUMN recorded_at TEXT;` + `ALTER TABLE observations ADD COLUMN recorded_at TEXT;`
- [ ] 1.1.5 Registrar migración en `migration.rs`
- [ ] 1.1.6 Actualizar `fix_schema_compat()` para manejar DBs sin `recorded_at`
- [ ] 1.1.7 Actualizar SQL INSERT/SELECT en `sqlite.rs` para incluir `recorded_at`
- [ ] 1.1.8 Agregar `recorded_at` a `AddEdgeParams` y `AddObservationParams`
- [ ] 1.1.9 `cargo check` pasa

### 1.2 — Lógica de Supersedes
**Archivos:** `crates/store/src/sqlite.rs`, `crates/store/src/trait.rs`

- [ ] 1.2.1 Modificar `add_edge()`: al cerrar arista vieja (set `valid_until`), TAMBIÉN set `superseded_by = new_edge_id` en la fila vieja
- [ ] 1.2.2 El SQL UPDATE actual: `UPDATE edges SET valid_until = ? WHERE id = ?` → agregar `superseded_by = ?`
- [ ] 1.2.3 Test: verificar que `add_edge` con relación conflictiente marca la vieja como superseded
- [ ] 1.2.4 `cargo check` + `cargo test -p engram-store` pasa

### 1.3 — Integración CRDT por-columna
**Archivos:** `crates/sync/src/crdt.rs`, `crates/store/src/trait.rs`

- [ ] 1.3.1 Cambiar `CrdtState.vector_clock: u64` → `column_clocks: HashMap<String, u64>` (por-columna)
- [ ] 1.3.2 Agregar `get_clock(&self, column: &str) -> u64` e `increment_clock(&mut self, column: &str)`
- [ ] 1.3.3 Actualizar `SyncDelta` para incluir `column: String` (campo-level granularity)
- [ ] 1.3.4 Agregar tabla CRDT metadata: migración `018_crdt_columns.sql` con `(observation_id, column_name, lamport_clock, device_id, updated_at)`
- [ ] 1.3.5 Implementar `resolve_column_conflict()` que compara lamport clocks por columna
- [ ] 1.3.6 `cargo check` + `cargo test -p engram-sync` pasa

**Checkpoint Fase 1:** `cargo check --workspace` + `cargo test --workspace` verde

---

## Fase 2: Motor de Inferencia Local Embebido

### 2.1 — Crate engram-learn: Bindings llama_cpp_rs
**Archivos:** `crates/learn/Cargo.toml`, `crates/learn/build.rs` (nuevo)

- [ ] 2.1.1 Agregar `llama-cpp-rs` como dependencia opcional en `crates/learn/Cargo.toml`
- [ ] 2.1.2 Crear `crates/learn/build.rs` con feature flags: `feature "metal"` para macOS, `feature "cuda"` para Windows/Linux
- [ ] 2.1.3 Agregar features al workspace `Cargo.toml`: `engram-learn = { path = "crates/learn", features = [...] }`
- [ ] 2.1.4 `cargo check -p engram-learn` compila (aún sin usar el binding)

### 2.2 — Gestor de Modelos (Singleton Lazy Load)
**Archivos:** `crates/learn/src/lib.rs`, `crates/learn/src/inference.rs` (nuevo)

- [ ] 2.2.1 Crear `inference.rs` con struct `InferenceEngine`
- [ ] 2.2.2 Implementar `InferenceEngine::new(model_path: &Path)` — lazy init con `OnceCell`
- [ ] 2.2.3 Implementar `InferenceEngine::infer(prompt: &str) -> Result<String>` — carga modelo en primer uso, libera después
- [ ] 2.2.4 Implementar `InferenceEngine::is_loaded() -> bool` y `InferenceEngine::unload()`
- [ ] 2.2.5 Registrar módulo en `lib.rs`: `#[cfg(feature = "inference")] pub mod inference;`
- [ ] 2.2.6 Test unitario con mock (sin modelo real): verificar lazy load pattern
- [ ] 2.2.7 `cargo check -p engram-learn` pasa

### 2.3 — Host-Memory Prompt Caching
**Archivos:** `crates/learn/src/inference.rs`

- [ ] 2.3.1 Crear struct `PromptCache` con `HashMap<String, Vec<u8>>` para prefijos cacheados
- [ ] 2.3.2 Implementar `cache_system_prompt(system: &str, schema: &str) -> CacheKey`
- [ ] 2.3.3 Implementar `infer_with_cache(cache_key: &CacheKey, user_prompt: &str) -> Result<String>`
- [ ] 2.3.4 El cache reduce TTFT reutilizando el prefijo tokenizado del system prompt
- [ ] 2.3.5 `cargo check -p engram-learn` pasa

**Checkpoint Fase 2:** `cargo check --workspace` verde (sin modelo GGUF requerido)

---

## Fase 3: Pipeline de Extracción y Autocorrección

### 3.1 — Gramática GBNF
**Archivos:** `crates/learn/resources/kg_extraction.gbnf` (nuevo)

- [ ] 3.1.1 Definir gramática GBNF para `KnowledgeGraphEdges`: source_id, target_id, relation, weight
- [ ] 3.1.2 Definir gramática GBNF para `KnowledgeCapsule`: title, key_decisions, known_issues, anti_patterns
- [ ] 3.1.3 Validar gramática con parser GBNF (o test manual con llama.cpp)
- [ ] 3.1.4 Incluir archivo como `include_str!` en el módulo de inferencia

### 3.2 — Bucle de Autocorrección (Self-Healing)
**Archivos:** `crates/learn/src/extraction.rs` (nuevo), `crates/learn/src/capsule_builder.rs`

- [ ] 3.2.1 Crear `extraction.rs` con struct `ExtractionPipeline`
- [ ] 3.2.2 Implementar `extract_structured(prompt, grammar) -> Result<KnowledgeGraphEdges>`: inferencia guiada por GBNF
- [ ] 3.2.3 Implementar bucle: infer → `serde_json::from_str` → si falla, reintentar inyectando error del parser (máx 2 reintentos)
- [ ] 3.2.4 Implementar `LlmSynthesizer` que usa `ExtractionPipeline` (implementa trait `CapsuleSynthesizer`)
- [ ] 3.2.5 Conectar como primary en `ChainedSynthesizer` con `HeuristicSynthesizer` como fallback
- [ ] 3.2.6 `cargo check -p engram-learn` pasa

### 3.3 — Validación Semántica
**Archivos:** `crates/learn/src/extraction.rs`

- [ ] 3.3.1 Agregar validación: pesos en rango [0.0, 1.0]
- [ ] 3.3.2 Agregar validación: IDs de nodos consistentes (source/target existen)
- [ ] 3.3.3 Agregar validación: relation types válidos contra `RelationType` enum
- [ ] 3.3.4 Usar crate `validator` con derive macros en structs de extracción
- [ ] 3.3.5 Test: input inválido → self-healing corrige o rechaza gracefully
- [ ] 3.3.6 `cargo check -p engram-learn` + `cargo test -p engram-learn` pasa

**Checkpoint Fase 3:** `cargo check --workspace` + `cargo test --workspace` verde

---

## Fase 4: Optimización de Búsqueda (Binary Embeddings)

### 4.1 — Embeddings de 1-bit
**Archivos:** `crates/search/src/embedder.rs`, `crates/search/src/lib.rs`, `crates/store/src/migration.rs`, `crates/store/src/sqlite.rs`, `crates/store/src/trait.rs`

- [ ] 4.1.1 Implementar `binary_quantize(vec: &[f32]) -> Vec<u8>`: `(val > 0.0) ? 1 : 0`, pack 8 bits/u8
- [ ] 4.1.2 Para 384-dim: resultado = 48 bytes (384/8)
- [ ] 4.1.3 Implementar `hamming_distance(a: &[u8], b: &[u8]) -> u32`
- [ ] 4.1.4 Agregar `binary_hash: Vec<u8>` a `HydratedEmbedding`
- [ ] 4.1.5 Crear migración `019_binary_embeddings.sql`: `ALTER TABLE embeddings ADD COLUMN binary_hash BLOB;`
- [ ] 4.1.6 Extender `store_embedding()` para calcular y guardar binary hash automáticamente
- [ ] 4.1.7 Implementar `search_binary(query_hash: &[u8], limit: usize) -> Vec<(i64, u32)>` en Storage trait
- [ ] 4.1.8 SQL: scan de `binary_hash` con hamming distance via bit operations
- [ ] 4.1.9 `cargo check` + tests pasa

### 4.2 — Fusión RRF Update
**Archivos:** `crates/search/src/hybrid.rs`, `crates/search/src/lib.rs`

- [ ] 4.2.1 Agregar función `binary_prefilter(query: &[f32], limit: usize) -> Vec<i64>`: quantize → hamming search → top-K candidates
- [ ] 4.2.2 Modificar pipeline de búsqueda: binary prefilter → rerank con cosine completo → RRF fusion con BM25
- [ ] 4.2.3 Ajustar pesos RRF: binary search escala diferente (hamming 0-384 vs cosine 0-1)
- [ ] 4.2.4 Mantener k=60 pero agregar `binary_weight` configurable
- [ ] 4.2.5 Test: buscar con binary prefilter produce mismos top-5 que búsqueda completa
- [ ] 4.2.6 Benchmark: binary search debe ser >10x más rápido que full scan
- [ ] 4.2.7 `cargo check` + `cargo test -p engram-search` pasa

**Checkpoint Fase 4:** `cargo check --workspace` + `cargo test --workspace` verde

---

## Validación Final

- [ ] `cargo check --workspace` sin warnings
- [ ] `cargo test --workspace` verde
- [ ] `cargo clippy --workspace` sin errores
- [ ] `cargo fmt --check` pasa
- [ ] Integración: MCP server arranca con nuevas features
- [ ] Bump version: `2.1.0` → `2.2.0` en todos los Cargo.toml
