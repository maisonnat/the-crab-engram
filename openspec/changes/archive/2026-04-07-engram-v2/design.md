# Design: engram-rust v2

## Arquitectura General

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI (src/main.rs)                        │
│                     clap derive subcommands                     │
└──────────┬──────────────┬──────────────┬───────────────────────┘
           │              │              │
     ┌─────▼─────┐  ┌─────▼─────┐  ┌────▼──────┐
     │  MCP Server│  │ HTTP API  │  │    TUI    │
     │  crates/mcp│  │crates/api │  │crates/tui │
     │  rmcp 1.3+ │  │  axum 0.8 │  │ ratatui   │
     └─────┬──────┘  └─────┬─────┘  └─────┬─────┘
           │               │               │
     ┌─────▼───────────────▼───────────────▼─────┐
     │              Application Layer             │
     │  ┌─────────────┐  ┌──────────────────┐    │
     │  │  crates/learn│  │  crates/search   │    │
     │  │  Consolidation│  │  FTS5 + Vector  │    │
     │  │  Capsules    │  │  Hybrid (RRF)   │    │
     │  │  AntiPatterns│  │  Type-aware     │    │
     │  │  Streaming   │  │  Entity-aware   │    │
     │  └──────┬───────┘  └────────┬────────┘    │
     │         │                   │              │
     │  ┌──────▼───────────────────▼──────────┐  │
     │  │         crates/core (tipos puros)    │  │
     │  │  Observation, Session, Edge,         │  │
     │  │  KnowledgeCapsule, Belief, Entity,   │  │
     │  │  MemoryType, Salience, Lifecycle     │  │
     │  └──────────────────┬──────────────────┘  │
     └─────────────────────┼─────────────────────┘
                           │
     ┌─────────────────────▼─────────────────────┐
     │          Storage Trait (firewall)           │
     │      crates/store/src/trait.rs             │
     │   Result<T, EngramError> — zero leaks      │
     └─────────────────────┬─────────────────────┘
                           │
     ┌─────────────────────▼─────────────────────┐
     │          SQLite Implementation              │
     │   rusqlite (bundled) + FTS5 + sqlite-vec   │
     │   WAL mode, migrations versionadas         │
     └───────────────────────────────────────────┘
                           │
     ┌─────────────────────▼─────────────────────┐
     │        crates/sync (Chunk + CRDT)          │
     │   JSONL gzip chunks, LWW-Element-Set       │
     └───────────────────────────────────────────┘
```

## Decisiones de Diseño Clave

### D1: Storage Trait como Firewall

**Decisión:** Todo el código depende del trait `Storage`, nunca de `rusqlite` directamente.

**Razón:** Upgrade path a libSQL (Turso) solo requiere reemplazar `crates/store/src/sqlite.rs`. El resto del código no cambia. libSQL tiene vector search nativo, replicación, y async I/O.

**Reglas de audit:**
- ✅ Cada método retorna tipos definidos en `crates/core`
- ✅ Parámetros como structs propios (no SQL strings)
- ✅ `Result<T, EngramError>` (no `rusqlite::Error`)
- ❌ NADA de `fn raw_query(&self, sql: &str)`
- ❌ NADA de `fn get_connection(&self) -> &rusqlite::Connection`

### D2: CapsuleSynthesizer Trait

**Decisión:** `KnowledgeCapsule` synthesis es un trait con tres implementaciones: `HeuristicSynthesizer` (MVP, siempre disponible), `LlmSynthesizer` (API externa), `ChainedSynthesizer` (LLM → fallback).

**Razón:** Calidad de capsules depende de la calidad del synthesizer. Heurístico es baseline, LLM es premium. Chained intenta LLM primero, cae a heurístico si no hay API.

**Config:** `synthesizer: "heuristic" | "llm" | "chained"` (default: chained).

### D3: Episodic-Semantic Separation

**Decisión:** Dos sistemas de memoria explícitamente separados. Episódico = qué pasó (contexto rico, temporal). Semántico = qué se sabe (denso, general).

**Motor de conversión:** ConsolidationEngine convierte episodios accedidos 3+ veces con `surprise_factor > 0.5` en memorias semánticas.

### D4: Temporal Knowledge Graph

**Decisión:** Edges tienen `valid_from`, `valid_until`, `superseded_by` desde el inicio (Migration 004).

**Razón:** El grafo evoluciona en el tiempo. Queries temporales ("qué sabíamos cuando hicimos este cambio?") son críticas.

**Auto-cierre:** Cuando se crea un nuevo edge entre los mismos nodos con mismo relation type, el anterior cierra automáticamente.

### D5: Embedding Model Versioning

**Decisión:** Cada embedding se almacena con `model_name` + `model_version`. Si hay mismatch, búsqueda cae a FTS5-only hasta que el usuario corra `the-crab-engram reembed`.

**Razón:** Cada modelo crea un espacio geométrico diferente. Nunca mezclar vectores de modelos diferentes. (Ref: Gary Stafford, Dic 2025).

### D6: Lifecycle Policies por Tipo

**Decisión:** No todas las observations son permanentes. `Decision` y `Architecture` son forever. `Bugfix` se archiva a los 6 meses. `Command`/`FileRead`/`Search` se purgan automáticamente.

**Razón:** 5000 observations de `Command` son ruido. Las memorias valiosas necesitan ser distinguibles de las efímeras.

### D7: Belief State Machine

**Decisión:** Las contradicciones no se señalan — se resuelven. `BeliefState` es una máquina de estados (Active → Confirmed → Contested → Superseded/Retracted), no un flag binario.

**Regla de resolución:**
- Nueva evidencia con confidence >0.2 por encima del belief actual → Update automático
- Confidence similar → Contest (esperar más evidencia)
- Usuario corrigió → Retract (el usuario manda)
- 3+ fuentes concuerdan → auto-resolver Contested → Confirmed

### D8: Memory Compaction por Niveles

**Decisión:** Observations se compactan en niveles de abstracción: Raw → Facts → Patterns → Principles. Cada nivel es más denso y abstracto.

**SmartInjector elige nivel según query:**
- "¿qué config tiene auth?" → Fact (específico)
- "¿cómo solemos manejar errores?" → Pattern (tendencia)
- "¿qué tipo de proyecto es esto?" → Principle (visión general)

### D9: Spaced Repetition con Cold Start

**Decisión:** Sistema SM-2 básico para revisión periódica. La parte difícil es la detección implícita de ReviewResult (¿el agente usó o no usó el conocimiento?).

**Cold start:** Si review_schedule está vacío, bootstrap automático seleccionando top 50 observations más accedidas con intervals distribuidos.

### D10: MCP Resources coexisten con Tools

**Decisión:** MCP Resources (push) NO reemplazan MCP Tools (pull). Resources = contexto ambiental. Tools = queries específicas. Los dos conviven.

## Estructura de Crates

```
engram-rust/
├── Cargo.toml                    # Workspace root, resolver 3, shared deps
├── crates/
│   ├── core/                     # Tipos puros, cero dependencias pesadas
│   │   ├── observation.rs        # Observation, ObservationType, Scope
│   │   ├── session.rs            # Session, SessionSummary
│   │   ├── topic.rs              # TopicKey suggestion, slugify
│   │   ├── graph.rs              # Edge, RelationType, temporal columns
│   │   ├── score.rs              # Decay scoring (extensible por salience)
│   │   ├── capsule.rs            # KnowledgeCapsule
│   │   ├── provenance.rs         # ProvenanceInfo, ProvenanceSource
│   │   ├── memory.rs             # MemoryType (Episodic/Semantic)
│   │   ├── salience.rs           # MemorySalience
│   │   ├── spaced.rs             # SpacedRepetition, ReviewResult
│   │   ├── attachment.rs         # Attachment enum
│   │   ├── boundary.rs           # KnowledgeBoundary, ConfidenceLevel
│   │   ├── lifecycle.rs          # LifecyclePolicy, LifecycleState
│   │   ├── belief.rs             # Belief, BeliefState, BeliefOperation
│   │   ├── compaction.rs         # CompactionLevel, NewPattern, NewPrinciple
│   │   ├── entity.rs             # Entity, EntityType, EntityRegistry
│   │   ├── personality.rs        # AgentPersonality, WorkingStyle
│   │   ├── permissions.rs        # PermissionEngine, AccessLevel
│   │   └── error.rs              # EngramError enum
│   │
│   ├── store/                    # Storage trait + SQLite impl
│   │   ├── trait.rs              # Storage trait (firewall)
│   │   ├── sqlite.rs             # SqliteStore implementation
│   │   ├── migration.rs          # Migration runner
│   │   └── migrations/
│   │       ├── 001_initial.sql
│   │       ├── 002_fts.sql
│   │       ├── 003_vectors.sql
│   │       ├── 004_graph.sql
│   │       ├── 005_provenance.sql
│   │       ├── 006_capsules.sql
│   │       ├── 007_cross_project.sql
│   │       ├── 008_episodic_semantic.sql
│   │       ├── 009_review_schedule.sql
│   │       ├── 010_agent_permissions.sql
│   │       ├── 011_attachments.sql
│   │       ├── 012_knowledge_boundaries.sql
│   │       ├── 013_agent_personalities.sql
│   │       ├── 014_lifecycle_state.sql
│   │       ├── 015_beliefs.sql
│   │       └── 016_entities.sql
│   │
│   ├── search/                   # Search engine
│   │   ├── fts.rs                # FTS5 query builder
│   │   ├── vector.rs             # Vector similarity (sqlite-vec)
│   │   ├── hybrid.rs             # Reciprocal Rank Fusion
│   │   ├── embedder.rs           # fastembed wrapper + model versioning
│   │   ├── type_aware.rs         # Búsqueda diferenciada episódico/semántico
│   │   └── entity_aware.rs       # Búsqueda por entidad
│   │
│   ├── learn/                    # Auto-aprendizaje (crate más experimental)
│   │   ├── consolidation.rs      # ConsolidationEngine
│   │   ├── capsule_builder.rs    # KnowledgeCapsule synthesis
│   │   ├── graph_evolver.rs      # Auto-detección de relaciones
│   │   ├── anti_pattern.rs       # Anti-pattern detection
│   │   ├── cross_project.rs      # Cross-project learning
│   │   ├── smart_injector.rs     # Context-aware injection
│   │   ├── salience_infer.rs     # Salience inference
│   │   ├── spaced_review.rs      # Spaced repetition
│   │   ├── boundary_tracker.rs   # KnowledgeBoundary updater
│   │   ├── belief_manager.rs     # Belief resolution engine
│   │   ├── compaction_pipeline.rs # Memory compaction por niveles
│   │   ├── entity_registry.rs    # Entity extraction + resolution
│   │   ├── personality_analyzer.rs # AgentPersonality inference
│   │   └── stream.rs             # Memory streaming events
│   │
│   ├── sync/                     # Sync engines
│   │   ├── chunk.rs              # Git-friendly chunks (compat Go)
│   │   └── crdt.rs               # CRDT P2P sync (LWW)
│   │
│   ├── mcp/                      # MCP server
│   │   ├── server.rs             # EngramServer + ServerHandler
│   │   ├── profiles.rs           # Tool profiles (agent/admin/all)
│   │   ├── resources.rs          # MCP Resources (list/read)
│   │   └── tools/                # ~35 MCP tools
│   │       ├── mod.rs
│   │       ├── save.rs, search.rs, context.rs, session.rs
│   │       ├── timeline.rs, graph.rs, capsule.rs, consolidate.rs
│   │       ├── antipatterns.rs, transfer.rs, admin.rs
│   │       ├── share.rs, team_capsule.rs, agent_status.rs
│   │       ├── synthesize.rs, capsule_list.rs, capsule_get.rs
│   │       ├── pin.rs, reembed.rs, relate.rs, inject.rs
│   │       ├── knowledge_boundary.rs, graph_timeline.rs
│   │       ├── beliefs.rs, entities.rs, principles.rs
│   │       └── ...
│   │
│   ├── api/                      # HTTP REST API
│   │   ├── lib.rs
│   │   └── routes.rs
│   │
│   └── tui/                      # Terminal UI
│       ├── lib.rs
│       ├── app.rs
│       ├── views/
│       └── widgets/
│
├── src/
│   └── main.rs                   # CLI entrypoint (clap derive)
│
├── plugins/
│   ├── claude-code/
│   └── setup/
│
└── tests/
    ├── common/
    ├── integration/
    └── fixtures/
```

## Flujo de Datos

### mem_save
```
CLI/MCP Tool → AddObservationParams
  → dedup check (SHA-256 hash)
  → INSERT observation
  → auto-embed (title + "\n" + content) via fastembed
  → store embedding + model metadata
  → infer salience (keyword heuristic)
  → extract entities (NER + alias matching)
  → process beliefs (subject extraction + evidence)
  → check anti-patterns (recurring bug warning)
  → return observation_id
```

### mem_search (hybrid)
```
CLI/MCP Tool → SearchOptions
  → classify query type (episodic/semantic/generic)
  → FTS5 MATCH (keyword results)
  → vector search via sqlite-vec (semantic results)
  → Reciprocal Rank Fusion (k=60, fts=0.4, vector=0.6)
  → entity-aware enrichment
  → graph context enrichment (1-2 relations per result)
  → compute final_score (0.3*fts + 0.3*vector + 0.2*recency + 0.2*frequency)
  → apply lifecycle filter (default: active only)
  → return ranked results
```

### Consolidation
```
auto-consolidate timer → run_consolidation()
  → find_semantic_duplicates (cosine > 0.92) → merge
  → find_obsolete (superseded edges) → mark
  → find_contradictions (opposite sentiment) → flag
  → extract_patterns (3+ similar bugfixes) → create pattern
  → episodic_to_semantic (accessed 3x, surprise > 0.5) → convert
  → apply_lifecycle (stale → archived → deleted)
  → run_graph_evolution (temporal, co-occurrence, file, semantic)
  → rebuild stale capsules
  → return ConsolidationResult
```

### Smart Context Injection
```
mem_inject(task, files)
  → embed task → vector search 5 memories
  → file history search (max 3)
  → find relevant capsules (max 2)
  → find active anti-patterns
  → find knowledge boundaries for current domains
  → find pending spaced repetition reviews
  → calculate total_tokens
  → trim by priority if exceeds budget
  → format Markdown output
  → return InjectionContext
```

## Stack de Dependencias

| Componente | Crate | Razón |
|-----------|-------|-------|
| CLI | `clap` (derive) | Estándar Rust |
| Async | `tokio` (full) | MCP lo requiere |
| MCP | `rmcp` (1.3+) | SDK oficial MCP |
| SQLite | `rusqlite` (bundled) | FTS5 incluido |
| Vector search | `sqlite-vec` | Extensión SQLite para vectores |
| Embeddings | `fastembed` | all-MiniLM-L6-v2 (384d), local |
| Serialization | `serde` + `serde_json` | Estándar |
| TUI | `ratatui` + `crossterm` | Sucesor de tui-rs |
| HTTP | `axum` | Basado en tokio/tower |
| Crypto | `chacha20poly1305` | Cifrado at rest |
| Error handling | `thiserror` + `anyhow` | Patrón estándar |
| Logging | `tracing` + `tracing-subscriber` | Structured logging |
| Config | `toml` + `dirs` | Config file |
| UUID | `uuid` (v4) | Session IDs |
| Hashing | `sha2` | Dedup hashes |
