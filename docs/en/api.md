# The Crab Engram — API Reference

## HTTP REST API (14 Routes)

Start with: `the-crab-engram serve --port 7437`

All routes accept/return JSON. CORS is fully permissive.

### Observations

| Method | Path | Description | Request Body |
|---|---|---|---|
| `GET` | `/observations` | Search observations | Query: `query`, `type`, `project`, `limit` |
| `POST` | `/observations` | Create observation | `{ title, content, type?, scope?, session_id, project?, topic_key? }` |
| `GET` | `/observations/:id` | Get observation by ID | — |
| `PUT` | `/observations/:id` | Update observation (partial) | `{ title?, content?, pinned?, topic_key? }` |
| `DELETE` | `/observations/:id` | Soft-delete observation | — |

### Search

| Method | Path | Description | Request Body |
|---|---|---|---|
| `POST` | `/search` | Search (POST variant) | `{ query, type?, project?, limit? }` |

### Sessions

| Method | Path | Description | Request Body |
|---|---|---|---|
| `POST` | `/sessions` | Create session | `{ project? }` |
| `GET` | `/sessions/:id` | Get session by ID | — |
| `GET` | `/context` | Get session context | Query: `limit` |

### Data

| Method | Path | Description | Request Body |
|---|---|---|---|
| `GET` | `/stats` | Project statistics | — |
| `GET` | `/export` | Export project data (JSON) | — |
| `POST` | `/import` | Import data | `ExportData` JSON |

### Knowledge (F2+)

| Method | Path | Description | Request Body |
|---|---|---|---|
| `GET` | `/capsules` | List knowledge capsules | — |
| `GET` | `/capsules/:topic` | Get capsule by topic | — |
| `POST` | `/consolidate` | Run consolidation engine | — |
| `GET` | `/graph/:id` | Get edges for observation | — |
| `POST` | `/inject` | Smart context injection | `{ task, max_tokens? }` |
| `GET` | `/antipatterns` | Detect anti-patterns | — |

### Response Types

**Success:** JSON body (200) or `{ "id": 42 }` (201 for creates)

**Error:** `{ "error": "message" }` (400)

---

## MCP Tools (31 Tools)

Start with: `the-crab-engram mcp --profile agent`

### Core Tools (Agent profile)

| # | Tool Name | Description | Required Parameters |
|---|---|---|---|
| 1 | `mem_save` | Save a memory observation | `title`, `content`, `session_id` |
| 2 | `mem_search` | Search memories by keyword (FTS5 ranked) | `query` |
| 3 | `mem_context` | Get session context with anti-pattern warnings | — |
| 4 | `mem_session_start` | Start a new coding session | — (uses project from config) |
| 5 | `mem_session_end` | End a session with optional summary | `session_id` |
| 6 | `mem_get_observation` | Get full observation by ID (increments access count) | `observation_id` |
| 7 | `mem_suggest_topic_key` | Suggest topic key from type + title | `type`, `title` |
| 8 | `mem_capture_passive` | Extract learnings from agent output automatically | `output`, `session_id` |
| 9 | `mem_save_prompt` | Save user prompt for future context | `session_id`, `content` |
| 10 | `mem_update` | Update observation (partial) | `observation_id` |
| 11 | `mem_relate` | Create relationship between observations | `source_id`, `target_id`, `relation` |
| 12 | `mem_graph` | Get knowledge graph around observation (BFS) | `observation_id` |
| 13 | `mem_pin` | Pin/unpin observation (max relevance score) | `observation_id`, `pinned` |
| 14 | `mem_inject` | Smart context injection for a task | `task_description` |
| 15 | `mem_synthesize` | Generate/update knowledge capsule for topic | `topic` |
| 16 | `mem_capsule_list` | List knowledge capsules | — |
| 17 | `mem_capsule_get` | Get full capsule by topic | `topic` |
| 18 | `mem_antipatterns` | Detect anti-patterns (recurring bugs, hotspots) | — |
| 19 | `mem_consolidate` | Run memory consolidation (merge, mark obsolete) | — |
| 20 | `mem_knowledge_boundary` | View/update knowledge boundaries | — |
| 21 | `mem_transfer` | Cross-project knowledge transfers | — |
| 22 | `mem_reviews` | Get pending spaced repetition reviews | — |
| 23 | `mem_beliefs` | Query beliefs about a subject | `subject` |
| 24 | `mem_capture_git` | Capture git commit as observation | `commit_hash`, `commit_message`, `session_id` |
| 25 | `mem_capture_error` | Capture error as observation with ErrorTrace | `error_type`, `error_message`, `session_id` |
| 26 | `mem_stream` | Detect memory events (file context, deja-vu, etc.) | — |
| 27 | `mem_sync` | Sync operations: status, export, import | `action` |

### Admin Tools (Admin profile only)

| # | Tool Name | Description | Required Parameters |
|---|---|---|---|
| 28 | `mem_delete` | Delete observation (soft or hard) | `observation_id` |
| 29 | `mem_stats` | Project statistics | — |
| 30 | `mem_timeline` | Timeline around observation | `observation_id` |
| 31 | `mem_merge_projects` | Merge all observations between projects | `source_project`, `target_project` |

### Tool Profiles

| Profile | Available Tools |
|---|---|
| `agent` | All except delete, stats, timeline, merge_projects (27 tools) |
| `admin` | Only delete, stats, timeline, merge_projects (4 tools) |
| `all` | All 31 tools |

### Optional Parameters

**`mem_save`:** `type` (enum: bugfix, decision, architecture, pattern, discovery, learning, config, convention, tool_use, file_change, command, file_read, search, manual; default: manual), `scope` (project/personal; default: project), `project`, `topic_key`, `attachments` (array of Attachment objects)

**`mem_search`:** `project`, `type`, `limit` (default: 10)

**`mem_update`:** `title`, `content`, `pinned`, `topic_key`

**`mem_graph`:** `max_depth` (default: 2)

**`mem_relate`:** `relation` (enum: caused_by, related_to, supersedes, blocks, part_of), `weight` (0.0-1.0, default: 1.0)

**`mem_stream`:** `file_path`, `task_description`, `mode` (file_context, deja_vu, anti_patterns, pending_reviews, entities)

**`mem_sync`:** `action` (enum: status, export, import), `dir`

---

## MCP Resources (3 Resources)

| URI Pattern | Name | Description |
|---|---|---|
| `engram://{project}/current-context` | Current Context | Recent observations from current session (Markdown) |
| `engram://{project}/knowledge-capsules` | Knowledge Capsules | Synthesized knowledge by topic (Markdown) |
| `engram://{project}/anti-patterns` | Anti-Patterns | Active anti-pattern warnings (Markdown) |

---

## CLI Commands (15+)

```
The Crab Engram: Persistent memory for AI agents

Usage: the-crab-engram [OPTIONS] <COMMAND>

Options:
      --db <PATH>         Path to the database [default: ~/.engram/engram.db]
      --project <NAME>    Project name [default: "default"]

Commands:
  mcp             Start MCP server (stdio transport)
  search          Search observations
  save            Save an observation
  context         Get session context
  stats           Get project statistics
  timeline        Get timeline around an observation
  export          Export data to JSON
  import          Import data from JSON
  export-context  Export context as Markdown system prompt
  session-start   Start a new session
  session-end     End a session
  serve           Start HTTP REST API server
  tui             Launch interactive Terminal UI
  consolidate     Run memory consolidation
  sync            Sync operations (chunk export/import, status)
  encrypt         Encrypt or decrypt the database
  setup           Setup The Crab Engram for a specific AI agent
  version         Version info
```

### Examples

```bash
# Start MCP server for Claude Code
the-crab-engram mcp --profile agent --project my-app

# Search for auth-related memories
the-crab-engram search "JWT auth" --limit 5

# Save a learning
the-crab-engram save --title "Fixed N+1 query" --content "Used eager loading" --type bugfix --session-id abc-123

# Start HTTP API
the-crab-engram serve --port 7437

# Launch TUI
the-crab-engram tui

# Export context as system prompt
the-crab-engram export-context --max-tokens 2000

# Encrypt database
the-crab-engram encrypt --passphrase "my-secret"

# Setup for Opencode
the-crab-engram setup opencode
```
