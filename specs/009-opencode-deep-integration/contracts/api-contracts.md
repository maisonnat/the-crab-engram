# Contracts: OpenCode Deep Integration

**Date**: 2026-04-15 | **Branch**: `009-opencode-deep-integration`

---

## HTTP API Endpoints (New)

### GET /health

Health check for plugin connectivity.

```
Request:  GET /health
Response: 200 { "status": "ok", "version": "2.0.0" }
          503 { "error": "database unavailable" }
```

### POST /sessions/:id/end

End a session with optional summary.

```
Request:  POST /sessions/:id/end
Body:     { "summary?": "string" }
Response: 200 { "status": "ended", "session_id": "..." }
          404 { "error": "session not found" }
```

---

## CLI Commands (Modified)

### the-crab-engram setup opencode

```
Usage: the-crab-engram setup opencode [OPTIONS]

Options:
  --profile <agent|admin|all>   MCP tool profile (default: agent)
  --project <name>              Project name for MCP command
  --uninstall                   Remove OpenCode integration
  --dry-run                     Show what would be done without writing

Output: Table with columns [Action, Target, Status]
  Created  | ~/.config/opencode/opencode.json  | MCP entry + plugin registered
  Created  | ~/.config/opencode/plugins/the-crab-engram.ts | Plugin file
  Updated  | ~/.config/opencode/AGENTS.md | Memory Protocol injected

Exit codes:
  0 - Success (or dry-run)
  1 - Failure (permission, disk, parse error)
```

### the-crab-engram doctor opencode

```
Usage: the-crab-engram doctor opencode [OPTIONS]

Options:
  --fix   Auto-repair failures

Output: Table with columns [Check, Status, Message]
  Binary in PATH         | PASS | /usr/local/bin/the-crab-engram
  OpenCode installed     | PASS | v1.4.6
  Config exists          | PASS | ~/.config/opencode/opencode.json
  MCP entry valid        | PASS | the-crab-engram registered
  Plugin file exists     | PASS | plugins/the-crab-engram.ts (v2.0.0)
  Server running         | FAIL | http://localhost:7437 unreachable
  Database OK            | PASS | integrity_check: ok

Exit codes:
  0 - All checks pass
  1 - One or more failures
```

---

## Plugin Hooks Contract (TypeScript)

### Hook: event → session.created

```
Trigger: OpenCode creates a new session
Action:  POST /sessions { project: <projectName> }
Output:  Store session_id in plugin state
```

### Hook: event → session.idle

```
Trigger: Session goes idle (agent finished responding)
Action:  POST /consolidate
Output:  None (fire-and-forget)
```

### Hook: event → session.deleted

```
Trigger: Session deleted by user
Action:  POST /sessions/:id/end { summary: "session deleted" }
Output:  Clear session_id from state
```

### Hook: experimental.session.compacting

```
Trigger: OpenCode about to compact session context
Action:
  1. GET /context?limit=10
  2. GET /capsules
  3. GET /antipatterns
Output:
  output.context.push(contextMarkdown)
  output.context.push(capsulesMarkdown)
  output.context.push(antipatternsMarkdown)
  output.context.push(recoveryInstructionsBlock)
```

### Hook: experimental.chat.system.transform

```
Trigger: System prompt being assembled for LLM call
Action:
  1. Check pendingContext cache
  2. If present: output.system.push(pendingContext)
  3. Always: output.system.push(memoryProtocolMarkdown)
Output: Appended strings to system prompt
```

### Hook: chat.message

```
Trigger: User sends a message
Action:
  1. Extract text from message
  2. POST /search { query: <text>, limit: 1 }
  3. If results: POST /inject { task: <text>, max_tokens: 2000 }
  4. Cache result in pendingContext for system.transform
Output: None (context cached for next system.transform call)
```

### Hook: tool.execute.after

```
Trigger: Tool execution completes
Condition: tool === "bash" or tool === "shell"
Action:
  - If output contains "git commit" → POST /observations with type=file_change
  - If output contains non-zero exit + "error"/"failed" → POST /observations with type=bugfix
Output: None (fire-and-forget)
Rate limit: Max 1 capture per 2 seconds
```

---

## Config Merge Contract

### Input: Existing opencode.json

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "github": { "type": "local", "command": ["github-mcp"], "enabled": true }
  },
  "plugin": ["./my-other-plugin.ts"]
}
```

### Output: After merge

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "github": { "type": "local", "command": ["github-mcp"], "enabled": true },
    "the-crab-engram": {
      "type": "local",
      "command": ["the-crab-engram", "mcp"],
      "enabled": true
    }
  },
  "plugin": ["./my-other-plugin.ts", "./plugins/the-crab-engram.ts"]
}
```

**Invariants**:
- Existing keys NEVER modified
- MCP entry upserted (updated if exists, created if not)
- Plugin path appended to array (not duplicated)
- `$schema` preserved
