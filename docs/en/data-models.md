# Engram-Rust — Data Models

## Core Entities

### Observation

The fundamental unit of memory. Every piece of knowledge stored in Engram is an Observation.

```rust
pub struct Observation {
    pub id: i64,
    pub r#type: ObservationType,      // 14 types
    pub scope: Scope,                  // Project | Personal
    pub title: String,
    pub content: String,
    pub session_id: String,            // UUID v4
    pub project: String,
    pub topic_key: Option<String>,     // e.g. "bug/n1-query"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: i64,
    pub last_accessed: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub normalized_hash: String,       // SHA-256 of title+content (dedup)

    // Provenance (F2.5.1)
    pub provenance_source: ProvenanceSource,
    pub provenance_confidence: f64,    // 0.0-1.0
    pub provenance_evidence: Vec<String>,

    // Lifecycle (F2.5.9)
    pub lifecycle_state: LifecycleState,

    // Salience (F2.5.7)
    pub emotional_valence: f64,
    pub surprise_factor: f64,
    pub effort_invested: f64,
}
```

**ObservationType** (14 variants):

| Type | Purpose |
|---|---|
| `bugfix` | Bug fix with root cause |
| `decision` | Architectural choice with tradeoffs |
| `architecture` | System design knowledge |
| `pattern` | Recurring code pattern |
| `discovery` | Non-obvious codebase insight |
| `learning` | General learning |
| `config` | Configuration/environment setup |
| `convention` | Team/naming conventions |
| `tool_use` | Tool usage patterns |
| `file_change` | Code change record |
| `command` | Command execution record |
| `file_read` | File read record |
| `search` | Search record |
| `manual` | Manually entered |

**Scope:** `Project` (shared) | `Personal` (per-agent)

**ProvenanceSource:**

| Source | Default Confidence |
|---|---|
| `test_verified` | 0.95 |
| `code_analysis` | 0.85 |
| `user_stated` | 0.70 |
| `external` | 0.65 |
| `llm_reasoning` | 0.60 |
| `inferred` | 0.40 |

**LifecycleState:** `Active` → `Stale` → `Archived` → `Deleted`

---

### Edge

Temporal relationship in the knowledge graph.

```rust
pub struct Edge {
    pub id: i64,
    pub source_id: i64,              // Observation ID
    pub target_id: i64,              // Observation ID
    pub relation: RelationType,
    pub weight: f64,                 // 0.0-1.0
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,  // None = active
    pub superseded_by: Option<i64>,          // Edge ID
    pub auto_detected: bool,
}
```

**RelationType:** `caused_by` | `related_to` | `supersedes` | `blocks` | `part_of`

Edges are temporal — when a new edge supersedes an old one, `valid_until` is set and `supersed_by` points to the new edge.

---

### KnowledgeCapsule

Dense synthesis of knowledge about a topic. More than raw observations — this is what the system "understands".

```rust
pub struct KnowledgeCapsule {
    pub id: i64,
    pub topic: String,
    pub project: Option<String>,
    pub summary: String,                    // 500-1000 chars
    pub key_decisions: Vec<String>,
    pub known_issues: Vec<String>,
    pub anti_patterns: Vec<String>,
    pub best_practices: Vec<String>,
    pub source_observations: Vec<i64>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub last_consolidated: DateTime<Utc>,
    pub version: u32,
}
```

---

### Belief

Evolving belief with evidence tracking and state machine.

```rust
pub struct Belief {
    pub id: i64,
    pub subject: String,
    pub current_value: String,
    pub previous_values: Vec<HistoricalBelief>,
    pub confidence: f64,
    pub last_evidence: Vec<i64>,
    pub state: BeliefState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**BeliefState:** `Active` → `Confirmed` (≥0.9 confidence, ≥3 evidence) | `Contested` (contradictory evidence) | `Superseded` | `Retracted`

**BeliefOperation:** `Create` | `Update` | `Confirm` | `Contest` | `Retract` | `Resolve`

---

### Session

Groups observations by time and context.

```rust
pub struct Session {
    pub id: String,                    // UUID v4
    pub project: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
}
```

---

### Attachment

Multimodal attachment for observations.

```rust
pub enum Attachment {
    CodeDiff { file_path, before_hash, after_hash, diff },
    TerminalOutput { command, output, exit_code },
    ErrorTrace { error_type, message, stack_trace, file_line: Option<(String, u32)> },
    GitCommit { hash, message, files_changed: Vec<String>, diff_summary },
}
```

---

### Other Entities

**KnowledgeBoundary** — Confidence level per domain (expert/proficient/familiar/aware/unknown)

**Entity** — Extracted named entity with canonical name, type, and aliases

**EpisodicMemory** — Time-bound memory with context and emotional weight

**SemanticMemory** — Generalized knowledge extracted from episodic patterns

---

## Storage Trait Interface

The `Storage` trait (`crates/store/src/trait.rs`) defines 35 methods across these groups:

| Group | Methods |
|---|---|
| **Observations** | `insert_observation`, `get_observation`, `peek_observation`, `update_observation`, `delete_observation`, `search` |
| **Sessions** | `create_session`, `end_session`, `get_session`, `get_session_context` |
| **Prompts** | `save_prompt`, `get_prompts` |
| **Timeline** | `get_timeline` |
| **Statistics** | `get_stats` |
| **Graph** | `add_edge`, `get_edges`, `get_edges_at`, `get_related` |
| **Embeddings** | `store_embedding`, `search_vector`, `count_stale_embeddings`, `update_embedding_versions` |
| **Export/Import** | `export`, `import` |
| **Lifecycle** | `transition_state`, `mark_pending_review` |
| **Attachments** | `store_attachment`, `get_attachments` |
| **Capsules** | `upsert_capsule`, `get_capsule`, `list_capsules` |
| **Spaced Repetition** | `upsert_review`, `get_pending_reviews` |
| **Boundaries** | `upsert_boundary`, `get_boundaries` |
| **Beliefs** | `upsert_belief`, `get_beliefs` |
| **Entities** | `upsert_entity`, `link_entity_observation`, `get_entity` |
| **Cross-Project** | `add_transfer`, `get_transfers`, `accept_transfer` |
| **Agent Personality** | `upsert_personality`, `get_personality` |

---

## SQLite Schema (Key Tables)

### observations (Migration 001)

```sql
CREATE TABLE observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'project',
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    project TEXT NOT NULL,
    topic_key TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed TEXT,
    pinned INTEGER NOT NULL DEFAULT 0,
    normalized_hash TEXT NOT NULL,
    provenance_source TEXT DEFAULT 'llm_reasoning',
    provenance_confidence REAL DEFAULT 0.6,
    provenance_evidence TEXT DEFAULT '[]',
    lifecycle_state TEXT DEFAULT 'active',
    emotional_valence REAL DEFAULT 0.0,
    surprise_factor REAL DEFAULT 0.0,
    effort_invested REAL DEFAULT 0.0
);
```

**Indexes:** session_id, type, project, created_at, topic_key, normalized_hash, lifecycle_state

### sessions (Migration 001)

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    summary TEXT
);
```

### observations_fts (Migration 002)

```sql
CREATE VIRTUAL TABLE observations_fts USING fts5(
    title, content, topic_key,
    content='observations',
    content_rowid='id'
);
```

### edges (Migration 004)

```sql
CREATE TABLE edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL REFERENCES observations(id),
    target_id INTEGER NOT NULL REFERENCES observations(id),
    relation TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    valid_from TEXT NOT NULL DEFAULT (datetime('now')),
    valid_until TEXT,
    superseded_by INTEGER REFERENCES edges(id),
    auto_detected INTEGER NOT NULL DEFAULT 0,
    UNIQUE(source_id, target_id, relation, valid_from)
);
```

### knowledge_capsules (Migration 006)

```sql
CREATE TABLE knowledge_capsules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT NOT NULL,
    project TEXT,
    summary TEXT NOT NULL DEFAULT '',
    key_decisions TEXT NOT NULL DEFAULT '[]',
    known_issues TEXT NOT NULL DEFAULT '[]',
    anti_patterns TEXT NOT NULL DEFAULT '[]',
    best_practices TEXT NOT NULL DEFAULT '[]',
    source_observations TEXT NOT NULL DEFAULT '[]',
    confidence REAL NOT NULL DEFAULT 0.5,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_consolidated TEXT NOT NULL DEFAULT (datetime('now')),
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE(topic, project)
);
```

### observation_attachments (Migration 011)

```sql
CREATE TABLE observation_attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observation_id INTEGER NOT NULL REFERENCES observations(id) ON DELETE CASCADE,
    attachment_type TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Other Tables

| Table | Migration | Purpose |
|---|---|---|
| `prompts` | 001 | User prompts per session |
| `observation_embeddings` | 003 | Vector embeddings for similarity search |
| `knowledge_transfers` | 007 | Cross-project knowledge suggestions |
| `episodic_memories` | 008 | Time-bound memories with context |
| `semantic_memories` | 008 | Generalized knowledge patterns |
| `review_schedule` | 009 | Spaced repetition schedule |
| `knowledge_boundaries` | 012 | Domain confidence levels |
| `agent_personalities` | 013 | Agent working style profiles |
| `beliefs` | 015 | Evolving beliefs with evidence |
| `entities` | 016 | Named entities |
| `entity_observations` | 016 | Entity-observation links |
| `_migrations` | — | Migration tracking |
