# Quickstart: OpenCode Deep Integration

**Date**: 2026-04-15 | **Branch**: `009-opencode-deep-integration`

---

## Prerequisites

- `the-crab-engram` v2.0.0+ installed and in PATH
- OpenCode v1.4.6+ installed (`npm install -g @opencode-ai/opencode`)

## One-Command Setup

```bash
the-crab-engram setup opencode --project my-project
```

This:
1. Detects your OS and finds OpenCode config directory
2. Merges MCP server entry into `opencode.json` (preserves existing config)
3. Copies the TypeScript plugin to `plugins/the-crab-engram.ts`
4. Injects Memory Protocol into `AGENTS.md`
5. Reports what was created/updated

## Verify Setup

```bash
the-crab-engram doctor opencode
```

Output:
```
CHECK                  STATUS  MESSAGE
Binary in PATH         PASS    /usr/local/bin/the-crab-engram v2.0.0
OpenCode installed     PASS    v1.4.6
Config exists          PASS    ~/.config/opencode/opencode.json
MCP entry valid        PASS    the-crab-engram registered
Plugin file exists     PASS    plugins/the-crab-engram.ts
Server running         PASS    http://localhost:7437
Database OK            PASS    integrity_check: ok
```

## Auto-Start Behavior

When you launch OpenCode, the plugin automatically:
1. Checks if `the-crab-engram serve` is running on port 7437
2. If not → starts it in background
3. Starts a new engram session for your project
4. Injects Memory Protocol into the agent's system prompt

## What Happens Automatically

| Event | Plugin Action |
|---|---|
| Session created | Start engram session, inject welcome context |
| You send a message | Search memories, inject relevant context if found |
| Agent finishes (idle) | Trigger memory consolidation |
| File edited | Track file changes |
| Git commit detected | Auto-capture commit as observation |
| Error detected | Auto-capture error as observation |
| Session compacting | Inject session context + capsules + anti-patterns |
| Session deleted | End engram session |

## Token Budget

Push injection defaults to 2000 tokens. Configure via:

```bash
export ENGRAM_INJECT_BUDGET=4000  # Increase budget
```

## Uninstall

```bash
the-crab-engram setup opencode --uninstall
```

Removes MCP entry and plugin file. Preserves `AGENTS.md`.
