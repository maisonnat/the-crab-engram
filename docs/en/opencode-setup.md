# OpenCode Setup Guide

## One-Command Setup

```bash
the-crab-engram setup opencode --project my-project
```

This command:
1. Detects your OS and finds the OpenCode config directory
2. Merges the MCP server entry into `opencode.json` (preserves existing config)
3. Copies the TypeScript plugin to `plugins/the-crab-engram.ts`
4. Injects the Memory Protocol into `AGENTS.md`
5. Reports what was created/updated

## Verify Setup

```bash
the-crab-engram doctor opencode
```

Output:
```
CHECK                     STATUS   MESSAGE
Binary in PATH            PASS     /usr/local/bin/the-crab-engram
OpenCode installed        PASS     v1.4.6
Config exists             PASS     ~/.config/opencode/opencode.json
MCP entry valid           PASS     the-crab-engram registered
Plugin file exists        PASS     plugins/the-crab-engram.ts
Server running            PASS     http://localhost:7437
Database OK               PASS     integrity_check: ok
```

## Options

| Flag | Description |
|---|---|
| `--profile <agent\|admin\|all>` | MCP tool profile (default: agent) |
| `--project <name>` | Project name for MCP command |
| `--dry-run` | Show what would be done without writing |
| `--uninstall` | Remove OpenCode integration |

## Uninstall

```bash
the-crab-engram setup opencode --uninstall
```

Removes MCP entry and plugin file. Preserves `AGENTS.md`.

## Plugin Architecture

The TypeScript plugin (`plugins/opencode/the-crab-engram.ts`) handles:

| Event | Plugin Action |
|---|---|
| Session created | Start engram session, inject welcome context |
| User message | Search memories, inject relevant context |
| Agent idle | Trigger memory consolidation |
| File edited | Track file changes |
| Git commit detected | Auto-capture commit as observation |
| Error detected | Auto-capture error as observation |
| Session compacting | Inject session context + capsules + anti-patterns |
| Session deleted | End engram session |

## Auto-Start Behavior

When you launch OpenCode, the plugin automatically:
1. Checks if `the-crab-engram serve` is running on port 7437
2. If not, starts it in background
3. Starts a new engram session for your project
4. Injects Memory Protocol into the agent's system prompt

## Token Budget

Push injection defaults to 2000 tokens. Configure via:

```bash
export ENGRAM_INJECT_BUDGET=4000
```

## Manual Setup (Fallback)

If the automatic setup doesn't work:

1. Create/edit `~/.config/opencode/opencode.json`:
```json
{
  "mcp": {
    "the-crab-engram": {
      "type": "local",
      "command": ["the-crab-engram", "mcp", "--project", "my-project"],
      "enabled": true
    }
  },
  "plugin": ["./plugins/the-crab-engram.ts"]
}
```

2. Copy the plugin:
```bash
mkdir -p ~/.config/opencode/plugins
cp plugins/opencode/the-crab-engram.ts ~/.config/opencode/plugins/
```

3. Start the server:
```bash
the-crab-engram serve --port 7437 &
```

## Troubleshooting

### "Binary in PATH" fails
Ensure `the-crab-engram` is installed and in your PATH.

### "Server running" fails
Start the server manually: `the-crab-engram serve --port 7437`

### "Config parse error"
Your `opencode.json` may have invalid JSON. Validate with a JSON linter.

### Plugin not loading
Ensure the plugin path in config matches the actual file location.

### Doctor auto-repair
Run `the-crab-engram doctor opencode --fix` to auto-repair common issues.
