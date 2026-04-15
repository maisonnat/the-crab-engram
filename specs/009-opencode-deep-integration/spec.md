# Specification: OpenCode Deep Integration

## Context

The OpenCode agent integration for the-crab-engram is currently the weakest among all supported agents. While `setup claude-code`, `setup cursor`, and `setup gemini-cli` exist, OpenCode only has a manual JSON config snippet and a note to copy AGENTS.md manually. The competitor (Gentleman-Programming/engram) has a one-command `engram setup opencode` with a TypeScript plugin that injects memory protocol, handles compaction recovery, and auto-starts the server.

This spec defines a deep integration that goes beyond matching the competitor: push-based memory injection, auto-project bootstrapping, token-budget-aware context loading, and a health-check diagnostic command.

## Requirements

### WRQ-1: Cross-Platform Setup Command
- **WHEN** user runs `the-crab-engram setup opencode`
- **THEN** the system SHALL detect the OS (Linux/macOS/Windows) and write to the correct config paths:
  - Linux/macOS: `~/.config/opencode/`
  - Windows: `%APPDATA%\opencode\`
- **AND** SHALL merge MCP server entry into existing `opencode.json` (never overwrite)
- **AND** SHALL copy the plugin to `plugins/the-crab-engram.ts`
- **AND** SHALL merge Engram Memory Protocol into `AGENTS.md` (never overwrite)
- **AND** SHALL report what was created/modified with a summary

### WRQ-2: MCP Server Registration
- **WHEN** `setup opencode` completes
- **THEN** `opencode.json` SHALL contain a valid MCP entry:
  ```json
  {
    "mcp": {
      "the-crab-engram": {
        "type": "local",
        "command": ["the-crab-engram", "mcp"],
        "enabled": true
      }
    }
  }
  ```
- **AND** the entry SHALL use `merge` strategy (preserving other MCP servers)
- **AND** SHALL support `--project` and `--profile` flags forwarded to the MCP command

### WRQ-3: OpenCode TypeScript Plugin — Lifecycle Hooks
- **WHEN** the plugin is loaded by OpenCode
- **THEN** it SHALL register the following hooks:
  - `session.created` → call `mem_session_start`
  - `session.idle` → trigger `mem_consolidate`
  - `experimental.session.compacting` → inject recovery context (previous session + capsules + anti-patterns)
  - `file.edited` → passive capture via `mem_capture_passive`
  - `tool.execute.after` → detect git commits and call `mem_capture_git`
- **AND** the plugin SHALL auto-start `the-crab-engram serve` if not running
- **AND** SHALL be cross-platform (use `std::process::Command` equivalent or shell-agnostic paths)

### WRQ-4: Push-Based Memory Injection
- **WHEN** a user message arrives (`message.updated` with `role=user`)
- **THEN** the plugin SHALL perform a fast keyword search (`mem_search --limit 1 --format ids-only`)
- **AND** IF relevant memories exist, SHALL call `mem_inject` with `--max-tokens` budget
- **AND** SHALL inject the result as additional context via the hook's context mechanism
- **AND** SHALL NOT inject if no relevant memories are found (zero-cost path)
- **AND** the token budget SHALL be configurable (default: 2000 tokens, env var `ENGRAM_INJECT_BUDGET`)

### WRQ-5: Auto-Project Bootstrapping
- **WHEN** a `session.created` event fires for a project with zero observations
- **THEN** the system SHALL run `mem_capture_passive --scan-project <directory>`
- **AND** SHALL inject a bootstrapping context message into the session
- **AND** SHALL create initial knowledge boundaries from the scan
- **IF** the project has existing observations, SHALL inject `mem_context --sessions 3` as welcome-back context

### WRQ-6: Compaction Recovery with Engine Context
- **WHEN** `experimental.session.compacting` fires
- **THEN** the plugin SHALL inject:
  - Previous session context (`export-context --max-tokens 2000`)
  - Relevant knowledge capsules (`mem_capsule_list --format compact`)
  - Detected anti-patterns (`mem_antipatterns --format compact`)
  - Knowledge boundaries for the current task domain
  - Recovery instructions (call `mem_context`, review anti-patterns, check boundaries)
- **AND** SHALL set `output.context` (not replace `output.prompt`) to preserve default compaction behavior

### WRQ-7: Doctor Command
- **WHEN** user runs `the-crab-engram doctor opencode`
- **THEN** the system SHALL verify:
  1. `the-crab-engram` binary is in PATH
  2. `opencode` binary is installed
  3. `opencode.json` exists and contains valid MCP entry
  4. Plugin file exists in `plugins/` directory
  5. `the-crab-engram serve` is responding (HTTP health check)
  6. SQLite DB is accessible and FTS5 index is valid
  7. All 32 MCP tools respond (quick probe via `mem_stats`)
- **AND** SHALL output a status table with ✅/❌ per check
- **AND** SHALL support `--fix` flag that auto-repairs failures (re-run setup, restart server)

### WRQ-8: Uninstall/Reset
- **WHEN** user runs `the-crab-engram setup opencode --uninstall`
- **THEN** the system SHALL remove the MCP entry from `opencode.json`
- **AND** SHALL remove the plugin file from `plugins/`
- **AND** SHALL NOT remove `AGENTS.md` (may contain other agent instructions)
- **AND** SHALL report what was removed

### WRQ-9: Profile-Aware Plugin
- **WHEN** the plugin initializes
- **THEN** it SHALL respect the configured MCP profile (agent/admin/all)
- **AND** the `mem_inject` push-based injection SHALL only use read + search tools (no admin operations)
- **AND** file capture and git capture SHALL run with `agent` profile regardless of MCP config

### WRQ-10: Plugin Packaging via npm
- **THEN** the plugin SHALL be publishable as npm package `the-crab-engram-opencode`
- **AND** users SHALL be able to install via `"plugin": ["the-crab-engram-opencode"]` in `opencode.json`
- **AND** the npm package SHALL contain the compiled plugin + types
- **AND** `setup opencode` SHALL offer both local copy AND npm registration as options

### WRQ-11: Documentation
- **THEN** `docs/en/opencode-setup.md` SHALL exist with:
  - One-command setup instructions
  - Manual setup fallback
  - Plugin architecture explanation
  - Token budget configuration
  - Troubleshooting guide
  - Comparison table vs competitor (engram Go)
- **AND** the README SHALL list OpenCode in the agent setup table with one-liner

### WRQ-12: Cross-Platform Path Detection
- **WHEN** any setup/doctor/uninstall command needs OpenCode paths
- **THEN** the system SHALL use the following resolution order:
  1. `$OPENCODE_CONFIG_DIR` env var (if set)
  2. Platform defaults: `~/.config/opencode/` (Linux/macOS), `%APPDATA%\opencode\` (Windows)
  3. Project-level: `.opencode/` in current directory
- **AND** SHALL create directories if they don't exist
- **AND** SHALL handle both `opencode.json` and `opencode.jsonc` (JSON with comments)

## Scenarios

### S1: First-time setup on Linux
```gherkin
Given the-crab-engram is installed
And OpenCode is installed
And ~/.config/opencode/opencode.json does not exist
When user runs `the-crab-engram setup opencode`
Then ~/.config/opencode/opencode.json is created with MCP entry
And ~/.config/opencode/plugins/the-crab-engram.ts is created
And ~/.config/opencode/AGENTS.md is created with Memory Protocol
And setup reports 3 files created
```

### S2: Setup preserves existing config
```gherkin
Given ~/.config/opencode/opencode.json exists with MCP servers ["jira", "github"]
When user runs `the-crab-engram setup opencode`
Then opencode.json contains all 3 MCP servers: jira, github, the-crab-engram
And existing config values are not modified
```

### S3: Push injection fires on relevant query
```gherkin
Given 5 observations about "auth JWT implementation" exist
And the OpenCode plugin is loaded
When user sends message "how did we handle JWT refresh tokens?"
Then plugin performs fast search for relevant memories
And injects auth-related observations as context
And the agent receives the memories without needing to call mem_search
```

### S4: Push injection skips irrelevant query
```gherkin
Given no observations match "weather forecast"
When user sends message "what's the weather forecast?"
Then plugin performs fast search and finds no results
And no context is injected (zero token cost)
```

### S5: Doctor detects missing server
```gherkin
Given opencode.json has MCP entry
And the-crab-engram serve is NOT running
When user runs `the-crab-engram doctor opencode`
Then output shows ❌ for "engram serve running"
And suggests running `the-crab-engram serve &`
And with --fix flag, auto-starts the server
```

### S6: Project bootstrapping on first session
```gherkin
Given project "my-app" has zero observations in the-crab-engram
When OpenCode creates a new session for this project
Then plugin runs passive capture scan on the project directory
And injects bootstrapping context: "First session — I've scanned the codebase"
And initial knowledge boundaries are created
```

### S7: Compaction recovery
```gherkin
Given a session with 50 messages about "refactoring auth module"
And 3 knowledge capsules exist for this project
When OpenCode triggers compaction
Then plugin injects: previous context + capsules + anti-patterns + boundaries
And recovery instructions tell agent to call mem_context first
And the new agent session has full awareness of prior work
```

### S8: Uninstall cleans up
```gherkin
Given the-crab-engram is configured in OpenCode
When user runs `the-crab-engram setup opencode --uninstall`
Then MCP entry is removed from opencode.json
And plugin file is deleted
And AGENTS.md is preserved (may have other content)
And uninstall reports 2 items removed
```

### S9: Windows setup
```gherkin
Given the-crab-engram is installed on Windows
And %APPDATA%\opencode\ exists
When user runs `the-crab-engram setup opencode`
Then config is written to %APPDATA%\opencode\opencode.json
And plugin is written to %APPDATA%\opencode\plugins\the-crab-engram.ts
And paths use Windows separators
```

### S10: npm plugin install
```gherkin
Given the-crab-engram-opencode is published to npm
When user adds "the-crab-engram-opencode" to plugin array in opencode.json
Then OpenCode auto-installs the plugin via Bun
And all hooks (injection, compaction, capture) are active
And no local setup command was needed
```
