# The Crab Engram — User Guide

## Quick Start for AI Agents

### 1. Install & Start MCP Server

```bash
# Build
cargo build --release

# Start MCP server (configure your agent to launch this)
./target/release/the-crab-engram mcp
```

### 2. Session Lifecycle

Every coding session should be bracketed:

```
# Start
mem_session_start(project="my-project")
# → Returns: session_id (UUID v4)

# ... do work, save observations ...

# End
mem_session_end(session_id="abc-123", summary="Implemented auth module")
```

### 3. Save Learnings

After significant work, save observations:

```
# Bug fix
mem_save(
    title="Fixed N+1 query in UserList",
    content="Root cause: missing JOIN. Fix: added .includes(:posts) eager loading",
    type="bugfix",
    session_id="abc-123",
    topic_key="bug/n1-query"
)

# Architecture decision
mem_save(
    title="Use SQLite over Postgres",
    content="Tradeoff: single-machine limit vs zero ops overhead. Sufficient for local use.",
    type="decision",
    session_id="abc-123",
    topic_key="decision/storage"
)

# With attachment
mem_save(
    title="Auth validation fix",
    content="Added JWT expiry check",
    type="bugfix",
    session_id="abc-123",
    attachments=[{
        "type": "code_diff",
        "file_path": "src/auth.rs",
        "before_hash": "abc",
        "after_hash": "def",
        "diff": "+fn validate_expiry() {}"
    }]
)
```

### 4. Search Before Acting

Before implementing something new, check existing knowledge:

```
mem_search(query="JWT auth", limit=5)
# → Returns ranked observations with relevance scores

mem_search(query="N+1", type="bugfix")
# → Filter by type
```

### 5. Get Session Context

```
mem_context(project="my-project", limit=10)
# → Recent observations + anti-pattern warnings
```

---

## How to Use MCP Tools

### Core Workflow

| Step | Tool | When |
|---|---|---|
| Start session | `mem_session_start` | Beginning of coding session |
| Save knowledge | `mem_save` | After bug fix, decision, discovery |
| Search knowledge | `mem_search` | Before implementing, to check prior work |
| Get context | `mem_context` | At session start, to refresh context |
| End session | `mem_session_end` | End of coding session |

### Auto-Learning Tools

| Tool | What It Does |
|---|---|
| `mem_capture_passive` | Analyze agent output and auto-extract learnings (errors, changes, test results) |
| `mem_capture_git` | Capture a git commit as observation with GitCommit + CodeDiff attachments |
| `mem_capture_error` | Capture compilation/test error with ErrorTrace attachment |
| `mem_stream` | Detect real-time events: file context, deja-vu, anti-patterns, pending reviews |
| `mem_inject` | Build smart context injection for a task (relevant memories + warnings + boundaries) |

### Knowledge Graph Tools

| Tool | What It Does |
|---|---|
| `mem_relate` | Add typed edge between two observations |
| `mem_graph` | Get graph data for visualization |
| `mem_synthesize` | Generate KnowledgeCapsule from related observations |
| `mem_capsule_list` | List all knowledge capsules |
| `mem_capsule_get` | Get full capsule with decisions, issues, patterns |
| `mem_open_graph` | Open graph visualization |

### Maintenance Tools

| Tool | What It Does |
|---|---|
| `mem_consolidate` | Merge duplicates, mark obsolete, find conflicts, extract patterns |
| `mem_antipatterns` | Detect hotspot files with recurring bugs |
| `mem_knowledge_boundary` | View/update domain confidence levels |
| `mem_transfer` | Cross-project knowledge suggestions |
| `mem_reviews` | Spaced repetition review queue |
| `mem_beliefs` | Query evolving beliefs about subjects |
| `mem_sync` | Sync status, chunk export/import |
| `mem_pin` | Pin/unpin observation — pinned gets maximum relevance score |

---

## How to Use the API

```bash
# Start
the-crab-engram serve --port 7437

# Create observation
curl -X POST http://localhost:7437/observations \
  -H "Content-Type: application/json" \
  -d '{"title":"Auth fix","content":"Added JWT validation","session_id":"abc-123"}'

# Search
curl "http://localhost:7437/observations?query=auth&limit=5"

# Get stats
curl http://localhost:7437/stats

# Smart injection
curl -X POST http://localhost:7437/inject \
  -H "Content-Type: application/json" \
  -d '{"task":"Implement user auth","max_tokens":2000}'

# Run consolidation
curl -X POST http://localhost:7437/consolidate
```

---

## How to Use the TUI

```bash
the-crab-engram tui
```

**Views:**
- `1` **Dashboard** — Project stats overview
- `2` **Search** — Type to search, Enter to view details
- `3` **Capsules** — Browse knowledge capsules
- `4` **Boundaries** — View knowledge confidence levels

**Navigation:** `j`/`k` or arrow keys to move, `Enter` to select, `Esc` to go back, `q` to quit.

---

## How to Use the CLI

```bash
# Export context as system prompt (KILLER FEATURE)
the-crab-engram export-context --max-tokens 2000

# Outputs Markdown with top observations by access count, formatted for injection
# into AI agent system prompts

# Full export/import
the-crab-engram export --output backup.json
the-crab-engram import backup.json

# Sync between machines
the-crab-engram sync export --dir ./chunks
the-crab-engram sync import --dir ./chunks
the-crab-engram sync status

# Encrypt
the-crab-engram encrypt --passphrase "secret"
```

---

## Tips for AI Agents

1. **Always provide `session_id`** — observations without sessions can't be traced
2. **Use `topic_key`** — enables capsule synthesis and dedup (format: `category/slug`)
3. **Save attachments** — code diffs and error traces are more searchable than descriptions
4. **Run `mem_consolidate`** periodically — keeps the knowledge base clean
5. **Use `mem_inject`** before starting work — get relevant memories + warnings + boundaries
6. **Export context** as system prompt for new sessions — `the-crab-engram export-context`
7. **Check `mem_beliefs`** — evolving beliefs capture what the system "thinks" is true
8. **Use `mem_stream`** — real-time detection of deja-vu, anti-patterns, file context
