# Engram-Rust v2.0.0 — Overview

> Persistent memory for AI coding agents

**Engram-Rust** is a high-performance persistent memory layer that enables AI coding agents to remember learnings, decisions, bugs, and patterns across sessions. It operates as an MCP (Model Context Protocol) server, HTTP REST API, TUI, and CLI — all backed by SQLite with FTS5 full-text search.

## What It Does

Engram bridges the gap between AI agents' ephemeral context windows and long-term knowledge retention. Agents save "observations" — structured knowledge units — that survive across sessions, compactions, and restarts.

```
AI Agent ──MCP stdio──▶ Engram Server ──Storage trait──▶ SQLite + FTS5
                         │
                         ├─ HTTP REST API (Axum, port 7437)
                         ├─ TUI (Ratatui)
                         └─ CLI (Clap)
```

## Key Capabilities

| Capability | Details |
|---|---|
| **31 MCP Tools** | Save, search, consolidate, graph, sync, stream, and more |
| **14 HTTP API Routes** | Full REST API with CORS for web integrations |
| **Interactive TUI** | Dashboard, search, capsules, boundaries views |
| **15 CLI Commands** | mcp, search, save, context, stats, timeline, export, import, export-context, session-start, session-end, serve, tui, consolidate, sync, encrypt, setup |
| **Auto-Learning** | Passive capture, smart injection, spaced repetition, anti-pattern detection |
| **Knowledge Graph** | Temporal edges with 5 relation types (caused_by, related_to, supersedes, blocks, part_of) |
| **Knowledge Capsules** | Dense topic synthesis from multiple observations |
| **Consolidation** | Duplicate merging, obsolete marking, conflict detection |
| **Multimodal Attachments** | CodeDiff, TerminalOutput, ErrorTrace, GitCommit |
| **ChaCha20-Poly1305 Encryption** | Database-level encryption with passphrase derivation |
| **Multi-Agent Permissions** | Read/Write/Admin access levels per agent per project |
| **Cross-Project Sync** | CRDT-based chunk export/import between machines |

## Architecture Overview

Engram-Rust is a **Rust workspace** with 8 crates:

| Crate | Role |
|---|---|
| `engram-core` | Core types: Observation, Edge, Session, Belief, crypto, permissions |
| `engram-store` | Storage trait + SQLite/FTS5 implementation (13 migrations) |
| `engram-mcp` | MCP server with 31 tools + 3 resources (stdio transport via `rmcp`) |
| `engram-api` | HTTP REST API (Axum) — 14 routes |
| `engram-learn` | Auto-learning: consolidation, capsules, anti-patterns, smart injection, stream engine |
| `engram-search` | Hybrid search: FTS5 + vector similarity (embedder + reciprocal rank fusion) |
| `engram-sync` | CRDT-based sync: chunk export/import, vector clocks, conflict resolution |
| `engram-tui` | Terminal UI (Ratatui + Crossterm) — dashboard, search, capsules, boundaries |

## Technology Stack

| Layer | Technology |
|---|---|
| Language | Rust 2024 edition |
| Database | SQLite 3 (via `rusqlite`, bundled) with WAL mode |
| Full-Text Search | SQLite FTS5 |
| MCP Protocol | `rmcp` v1.3 (stdio transport) |
| HTTP Server | Axum 0.8 with CORS |
| TUI | Ratatui + Crossterm |
| CLI | Clap 4 (derive) |
| Encryption | ChaCha20-Poly1305 (AEAD) |
| Hashing | SHA-256 (dedup + key derivation) |
| Serialization | Serde + JSON |
| Async Runtime | Tokio (full features) |
| Error Handling | thiserror + anyhow |
| Logging | tracing + tracing-subscriber |

## Quick Links

- [Architecture](architecture.md) — Crate structure, dependency graph, data flow
- [API Reference](api.md) — All HTTP routes, MCP tools, CLI commands
- [Data Models](data-models.md) — Core entities, Storage trait, SQLite schema
- [Setup Guide](setup.md) — Build, run, test instructions
- [User Guide](user-guide.md) — How to use with AI agents
- [Changelog](changelog.md) — Version history
- [Security Posture](security-posture.md) — Encryption, permissions, safety
