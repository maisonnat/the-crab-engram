# Change Proposal: engram-wiring

## Intent

Complete all remaining wiring gaps in engram-rust: connect dead code modules to production paths, fix architectural inconsistencies, add missing integrations, and clean up warnings. This change covers 29 items across security, architecture, API, and quality.

## Scope

- **crates/store/** — SQL fixes, decay scoring in search, transactional updates, vector stubs
- **crates/mcp/tools/** — wire MemoryStream, beliefs tool, permissions check, search hybrid
- **crates/api/** — add missing F2+ routes, integrate search crate
- **crates/core/** — wire crypto to main.rs, wire episodic/semantic memory, wire compaction
- **crates/search/** — integrate Embedder + RRF into MCP/API search paths
- **src/main.rs** — wire crypto module, add background consolidation
- **crates/tui/** — show learn data (anti-patterns, capsules real data)

## Motivation

The engram-rust v2 SDD claims 100% task completion (236/236 [x]), but audit reveals:
- 10 modules exist with full implementations but are never called from production code
- 5 MCP handlers were replicating logic inline (fixed in previous session)
- Search results ignore decay scoring, hybrid search, and embeddings
- API only exposes CRUD, missing all F2+ features
- 11 compiler warnings from unused imports
- Missing migration for vector store

Without this change, the project has a beautiful architecture on paper but a production path that only uses ~40% of the implemented capability.

## Approach

Organize work into 5 workstreams, each independently testable:

1. **Store/Query** — decay scoring, transactions, episodic/semantic inserts
2. **MCP Wiring** — MemoryStream, beliefs tool, permissions, hybrid search
3. **API Routes** — F2+ endpoints, search integration
4. **Core Wiring** — crypto in main.rs, compaction in SmartInjector
5. **Cleanup** — warnings, vector migration, TUI data

## Risks

- Changing search scoring behavior could break existing consumers who expect FTS5 rank ordering
- Adding permissions checks to MCP could break existing tool calls if permission engine is too strict
- Vector migration (003) has no backing implementation yet — just table creation

## Ready for Proposal

Yes — exploration complete, 29 items identified and categorized.
