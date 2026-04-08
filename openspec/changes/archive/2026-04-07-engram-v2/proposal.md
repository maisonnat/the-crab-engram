# Proposal: engram-rust v2 — Persistent Memory for AI Agents

## Intent

Reimplementar Engram (Go) en Rust como un sistema de memoria con auto-aprendizaje para agentes IA. No es solo un diario con buena búsqueda — es un sistema que **consolida, sintetiza, predice y advierte**. El proyecto agrega 10 innovaciones arquitectónicas sobre la base Go existente: búsqueda híbrida semántica, grafo de conocimiento temporal, auto-consolidación, knowledge capsules, cross-project learning, y más.

## Scope

### In Scope
- Core types puros en Rust (Observation, Session, Edge, KnowledgeCapsule, etc.)
- Storage trait abstracto (firewall contra vendor lock-in, upgrade path a libSQL)
- SQLite + FTS5 + sqlite-vec como backend MVP
- 15 MCP tools (paridad con Engram Go)
- Búsqueda híbrida FTS5 + vector embeddings (fastembed, all-MiniLM-L6-v2)
- Grafo de conocimiento temporal con auto-detección de relaciones
- Consolidation engine (dedup, merge, pattern extraction, episodic→semantic)
- Knowledge Capsules con trait CapsuleSynthesizer (LLM + heuristic fallback)
- Smart context injection con knowledge boundaries
- Spaced repetition con cold start bootstrap
- CLI completa (mcp, search, save, export-context, consolidate, setup)
- ~35 MCP tools + 3 MCP resources
- 16 SQLite migrations versionadas

### Out of Scope
- WebSocket sync (CRDT via directorio compartido es MVP)
- Screenshots con modelo de visión (requiere API externa, feature opcional)
- Multi-agent via A2A protocol (requiere CRDT como prerequisito)
- Memory streaming (requiere F2.2 + F2.5.4 como prerequisitos)
- Integración directa con notebooklm-rust-mcp (comunicación unidireccional opcional)

## Capabilities

### New Capabilities
- `core-types`: Tipos puros del dominio (Observation, Session, Edge, MemoryType, Salience, Belief, Entity, etc.)
- `storage-layer`: Storage trait abstracto + SQLite implementation + FTS5 + vector store + 16 migrations
- `search-engine`: FTS5 baseline → hybrid search (RRF) → type-aware → entity-aware
- `mcp-server-parity`: 15 MCP tools originales (paridad con Go) + profiles + annotations
- `auto-learning`: Consolidation engine, knowledge capsules, anti-patterns, graph evolution, provenance, lifecycle, beliefs, entities, memory compaction
- `smart-context`: Smart injection, cross-project learning, spaced repetition, agent personality
- `mcp-extensions`: MCP resources, new tools (relate, graph, pin, reembed, synthesize, etc.)
- `cli`: CLI completa con todos los subcommands (mcp, search, save, export-context, etc.)
- `api-http`: REST API con axum para integraciones externas
- `tui`: Terminal UI interactiva con ratatui
- `sync`: Chunk sync (compat Go) + CRDT sync P2P
- `encryption`: Cifrado at rest con Chacha20Poly1305

### Modified Capabilities
- None (greenfield project, no existing specs)

## Approach

**Arquitectura:** Workspace monorepo con 8 crates separados por responsabilidad. `core` contiene tipos puros sin dependencias externas pesadas. `store` implementa Storage trait sobre SQLite. `search` encapsula FTS5 + vector + hybrid. `learn` es el crate más experimental (consolidation, capsules, anti-patterns, streaming). `mcp` implementa el servidor MCP. `api` expone HTTP. `tui` provee UI terminal. CLI en `src/main.rs`.

**Patrón clave:** Storage trait como firewall. Todo el código depende del trait, nunca de rusqlite directamente. Esto permite futuro swap a libSQL (Turso) sin cambiar nada fuera de `crates/store`.

**Fases secuenciales:** F1 (Core+Store+Paridad Go) → F2 (Search+Graph+Auto-Learning) → F3 (API+TUI+Sync+Advanced). No saltar fases.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/core/` | New | 16+ módulos de tipos puros |
| `crates/store/` | New | Storage trait + SQLite impl + 16 migrations |
| `crates/search/` | New | FTS5 + vector + hybrid search |
| `crates/learn/` | New | Consolidation, capsules, anti-patterns, streaming |
| `crates/mcp/` | New | MCP server + ~35 tools + 3 resources |
| `crates/api/` | New | HTTP REST API con axum |
| `crates/tui/` | New | Terminal UI con ratatui |
| `crates/sync/` | New | Chunk sync + CRDT |
| `src/main.rs` | New | CLI entrypoint |
| `plugins/` | New | Plugin installers + git hooks |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| sqlite-vec es pre-v1 y puede tener breaking changes | Medium | Mantener vector search abstracta detrás del Storage trait |
| fastembed modelo all-MiniLM-L6-v2 puede quedar obsoleto | Medium | Versionado de embeddings + model drift detection + fallback FTS5 |
| Auto-graph evolution genera ruido (falsos positivos) | High | Empezar con thresholds altos (0.9 similarity, 3+ occurrences) |
| Spaced repetition sin cold start queda inactivo | High | Bootstrap automático al detectar review_schedule vacío |
| Scope creep por 10 innovaciones | High | Fases secuenciales, F1 es obligatorio antes de F2 |
| Complejidad del crate learn/ | Medium | Tests unitarios agresivos antes de integrar |

## Rollback Plan

- **F1 incompleto:** No avanzar a F2. El sistema funciona como SQLite store básico con MCP.
- **F2 problemas con embeddings:** Fallback a FTS5-only (ya funciona en F1.4).
- **F2.5 consolidation genera resultados malos:** Dry-run mode para auditar antes de ejecutar. Consolidation es opt-in.
- **sqlite-vec falla:** Vector search es detrás del trait Storage. Deshabilitar y usar FTS5 puro.
- **Proyecto abandonado:** Export/import JSON es compatible con Engram Go. Migración sin pérdida.

## Dependencies

- Rust edition 2024 (resolver 3)
- rmcp 1.3+ (MCP SDK oficial)
- rusqlite bundled (SQLite embebido)
- sqlite-vec (pre-v1, vector search)
- fastembed (all-MiniLM-L6-v2, ~80MB primer download)
- tokio (async runtime)
- axum 0.8 (HTTP API)
- ratatui 0.29 + crossterm 0.28 (TUI)

## Success Criteria

- [ ] 15/15 MCP tools paridad con Engram Go
- [ ] Test coverage >85% en core + store
- [ ] Hybrid search >20% improvement sobre FTS5-only (NDCG@10)
- [ ] Binary size <20MB
- [ ] Export-context genera contexto coherente <2000 tokens
- [ ] Chunk sync bidireccional compatible con Engram Go
- [ ] Consolidation detecta >5% duplicates del total
- [ ] Anti-pattern detection precision >70%
- [ ] Knowledge capsule coherence >80% useful (human eval)
