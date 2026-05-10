# Plan: `engram install` — Universal MCP Registration

**Date:** 2026-04-22  
**Author:** Hermes + Alejandro  
**Status:** Research Complete, Ready for Design Review

---

## 1. Research: MCP Config Formats by Agent

### 1.1 Comparative Table

| Agent | Config Format | Config Path(s) | MCP Key | Stdio Entry Structure | Notes |
|---|---|---|---|---|---|
| **Hermes** | YAML | `~/.hermes/config.yaml` | `mcp_servers` | `{command: str, args: [str], timeout: int}` | Already configured ✅ |
| **Claude Code** | JSON | User: `~/.claude/settings.json` → `mcpServers` key<br>Project: `.mcp.json` in project root | `mcpServers` | `{"command":"...", "args":[...], "env":{...}}` | CLI: `claude mcp add --transport stdio NAME -- CMD ARGS` or `claude mcp add-json NAME '...'` |
| **OpenCode** | JSON | `~/.config/opencode/config.json` (global)<br>`opencode.json` (project root) | `mcp` | `{"type":"local", "command":[cmd, ...args], "environment":{}, "enabled":true}` | **Already implemented** in `opencode_setup.rs` ✅ |
| **Cursor** | JSON | Global: `~/.cursor/mcp.json`<br>Project: `.cursor/mcp.json` | `mcpServers` | `{"command":"...", "args":[...], "env":{...}}` | Same format as Claude Code `.mcp.json` |
| **Windsurf** | JSON | `~/.codeium/windsurf/mcp_config.json` | `mcpServers` | `{"command":"...", "args":[...], "env":{...}}` | Same format as Cursor/Claude Code |
| **Aider** | YAML | `~/.aider.conf.yml` | ❌ No MCP support | N/A | Aider doesn't support MCP servers at all. No config key, no CLI command. |
| **Gemini CLI** | MD? | `~/.gemini/extensions/` | ❌ Unclear | N/A | Current `setup` only writes a skill .md file |

### 1.2 Detailed Format Examples

#### Hermes (`~/.hermes/config.yaml`)
```yaml
mcp_servers:
  engram:
    command: the-crab-engram
    args:
      - --db
      - ~/.engram/engram.db
      - mcp
      - --profile
      - all
    timeout: 30
```

#### Claude Code — Method 1: `.mcp.json` (project-level)
```json
{
  "mcpServers": {
    "the-crab-engram": {
      "command": "the-crab-engram",
      "args": ["mcp", "--project", "default", "--profile", "agent"],
      "env": {}
    }
  }
}
```

#### Claude Code — Method 2: `claude mcp add-json` (user-level)
```bash
claude mcp add-json the-crab-engram \
  '{"type":"stdio","command":"the-crab-engram","args":["mcp","--project","default","--profile","agent"]}'
```
- Stored in `~/.claude/settings.json` under `mcpServers`
- Scopes: `--scope local` (session), `--scope project` (`.mcp.json`), `--scope user` (settings.json)

#### Claude Code — Method 3: `claude mcp add` (CLI)
```bash
claude mcp add --transport stdio the-crab-engram -- the-crab-engram mcp --project default --profile agent
```

#### OpenCode (`~/.config/opencode/config.json`)
```json
{
  "mcp": {
    "the-crab-engram": {
      "type": "local",
      "command": ["the-crab-engram", "mcp", "--project", "default", "--profile", "agent"],
      "environment": {},
      "enabled": true
    }
  }
}
```
**Note:** `command` is a single array (not split into command + args).

#### Cursor (`~/.cursor/mcp.json`)
```json
{
  "mcpServers": {
    "the-crab-engram": {
      "command": "the-crab-engram",
      "args": ["mcp", "--project", "default", "--profile", "agent"],
      "env": {}
    }
  }
}
```

#### Windsurf (`~/.codeium/windsurf/mcp_config.json`)
```json
{
  "mcpServers": {
    "the-crab-engram": {
      "command": "the-crab-engram",
      "args": ["mcp", "--project", "default", "--profile", "agent"],
      "env": {
        "ENGRAM_DB": "~/.engram/engram.db"
      }
    }
  }
}
```

### 1.3 OS Path Differences

| Agent | Linux/macOS | Windows |
|---|---|---|
| Hermes | `~/.hermes/config.yaml` | Same (WSL) or `%APPDATA%\hermes\config.yaml` |
| Claude Code | `~/.claude/settings.json` / `.mcp.json` | `%APPDATA%\claude\settings.json` / `.mcp.json` |
| OpenCode | `~/.config/opencode/config.json` | `%APPDATA%\opencode\config.json` |
| Cursor | `~/.cursor/mcp.json` | `%APPDATA%\Cursor\User\globalStorage\settings\mcp.json` or `~/.cursor/mcp.json` |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | `%USERPROFILE%\.codeium\windsurf\mcp_config.json` |
| Aider | `~/.aider.conf.yml` | Same |

---

## 2. Current State Analysis

### What exists:
- `engram setup opencode` — Fully implemented, writes MCP config + plugin + AGENTS.md
- `engram setup claude-code` / `cursor` / `gemini-cli` — Only writes a skill `.md` file, does NOT write MCP config
- `config_merge.rs` — JSON merge logic (merge_mcp_entry, merge_plugin_path, strip_jsonc_comments)
- `doctor.rs` — Detection logic for OpenCode
- `opencode_paths.rs` — Path detection for OpenCode

### What's missing:
- **No MCP config writing** for Claude Code, Cursor, Windsurf, Hermes
- **No `install` command** (current `setup` is per-agent, not auto-detect)
- **No `install --all`** to configure everything at once
- **No `install --detect`** to scan what's installed
- **No uninstall** for most agents
- **Aider can't be supported** (no MCP)

---

## 3. Proposed Architecture

### 3.1 Command Design

```
# Auto-detect installed agents and configure them
engram install

# Configure specific agents
engram install claude-code
engram install cursor
engram install windsurf
engram install opencode
engram install hermes

# Configure all supported agents
engram install --all

# Dry run (show what would be done)
engram install --dry-run

# Project-level install (.mcp.json in current dir)
engram install --scope project
engram install --scope user

# Uninstall
engram install --uninstall claude-code

# Detect what's installed
engram install --detect
```

### 3.2 Agent Adapter Pattern (Trait)

```rust
// crates/mcp/src/agents/mod.rs

/// Trait that each agent adapter implements
pub trait AgentAdapter: Send + Sync {
    /// Unique identifier (e.g., "claude-code", "cursor")
    fn id(&self) -> &str;
    
    /// Human-readable name
    fn display_name(&self) -> &str;
    
    /// Detect if this agent is installed on the system
    fn detect(&self) -> DetectionResult;
    
    /// Get config file path(s) for the given scope
    fn config_paths(&self, scope: InstallScope) -> Vec<PathBuf>;
    
    /// Build the MCP entry in this agent's native format
    fn build_mcp_entry(&self, config: &EngramMcpConfig) -> serde_json::Value;
    
    /// Install (write config + optional extras)
    fn install(&self, config: &EngramMcpConfig, scope: InstallScope, dry_run: bool) -> Result<SetupResult>;
    
    /// Uninstall (remove config entries)
    fn uninstall(&self, scope: InstallScope, dry_run: bool) -> Result<SetupResult>;
}

pub enum InstallScope {
    User,       // Global/user-level config
    Project,    // Project-level config (.mcp.json, etc.)
}

pub struct EngramMcpConfig {
    pub binary_path: PathBuf,    // Full path to the-crab-engram binary
    pub db_path: PathBuf,        // Path to engram.db
    pub project: String,         // Default project name
    pub profile: String,         // Tool profile (agent/admin/all)
}

pub struct DetectionResult {
    pub installed: bool,
    pub binary_path: Option<PathBuf>,
    pub config_files: Vec<PathBuf>,
    pub version: Option<String>,
}
```

### 3.3 Agent Implementations

```
crates/mcp/src/agents/
├── mod.rs              // AgentAdapter trait + InstallScope + registry
├── claude_code.rs      // Claude Code adapter (.mcp.json + settings.json + claude mcp add)
├── cursor.rs           // Cursor adapter (mcp.json)
├── windsurf.rs         // Windsurf adapter (mcp_config.json)
├── opencode.rs         // Migrated from current opencode_setup.rs
├── hermes.rs           // Hermes adapter (config.yaml)
└── registry.rs         // Agent registry (detect all, get by name)
```

### 3.4 Config Format Families

There are really only **3 format families**:

| Family | Agents | Key | Command Format |
|---|---|---|---|
| **mcpServers** (split) | Claude Code, Cursor, Windsurf | `mcpServers` | `{"command":"bin", "args":[...]}` |
| **mcp** (unified array) | OpenCode | `mcp` | `{"type":"local", "command":["bin","arg1",...]}` |
| **mcp_servers** (YAML) | Hermes | `mcp_servers` | `command: bin\nargs: [...]` |

This means we can share logic:
- `mcpServers` family → shared `merge_mcp_servers_json()` function
- Each adapter just specifies path + key + entry builder

### 3.5 Install Flow

```
engram install
    │
    ├── 1. Detect binary path (which the-crab-engram)
    ├── 2. Detect DB path (~/.engram/engram.db)
    ├── 3. Detect installed agents (parallel)
    │       ├── claude-code → ❌ not installed
    │       ├── cursor → ✅ v0.48 detected
    │       ├── windsurf → ❌ not installed
    │       ├── opencode → ✅ v0.35 detected
    │       └── hermes → ✅ detected
    │
    ├── 4. Show detection table
    │       ┌──────────────┬────────────┬──────────────────────────────┐
    │       │ Agent        │ Status     │ Config Path                  │
    │       ├──────────────┼────────────┼──────────────────────────────┤
    │       │ Cursor       │ ✅ v0.48   │ ~/.cursor/mcp.json          │
    │       │ OpenCode     │ ✅ v0.35   │ ~/.config/opencode/...      │
    │       │ Hermes       │ ✅         │ ~/.hermes/config.yaml       │
    │       │ Claude Code  │ ❌         │ —                            │
    │       │ Windsurf     │ ❌         │ —                            │
    │       └──────────────┴────────────┴──────────────────────────────┘
    │
    ├── 5. For each detected agent:
    │       a. Build MCP entry in native format
    │       b. Read existing config (or create new)
    │       c. Merge entry (idempotent — skip if already present)
    │       d. Write config
    │       e. Report action (Created/Updated/Skipped)
    │
    └── 6. Display results table
            ACTION       TARGET                              DETAIL
            ────────────────────────────────────────────────────────────
            Updated      ~/.cursor/mcp.json                  Added MCP server entry
            Skipped      ~/.config/opencode/config.json      Already configured
            Skipped      ~/.hermes/config.yaml               Already configured
```

### 3.6 Key Design Decisions

1. **Idempotent**: Running `engram install` multiple times is safe. Skips if already configured.
2. **Non-destructive**: Never removes other MCP servers from config. Only adds/updates ours.
3. **Scope-aware**: User-level by default, `--scope project` for `.mcp.json` in CWD.
4. **Extensible**: Adding a new agent = implementing `AgentAdapter` trait + registering it.
5. **Dry-run first**: `--dry-run` shows exactly what would change.
6. **No `--db` flag needed**: Auto-detects `~/.engram/engram.db`.
7. **Binary path resolution**: Uses `which the-crab-engram` to get full path for absolute command references.

### 3.7 Files to Create/Modify

| File | Action | Purpose |
|---|---|---|
| `crates/mcp/src/agents/mod.rs` | **CREATE** | AgentAdapter trait, InstallScope, EngramMcpConfig |
| `crates/mcp/src/agents/claude_code.rs` | **CREATE** | Claude Code adapter |
| `crates/mcp/src/agents/cursor.rs` | **CREATE** | Cursor adapter |
| `crates/mcp/src/agents/windsurf.rs` | **CREATE** | Windsurf adapter |
| `crates/mcp/src/agents/opencode.rs` | **CREATE** | Migrate from `opencode_setup.rs` |
| `crates/mcp/src/agents/hermes.rs` | **CREATE** | Hermes adapter (YAML merge) |
| `crates/mcp/src/agents/registry.rs` | **CREATE** | Agent registry + auto-detect |
| `crates/mcp/src/lib.rs` | **MODIFY** | Add `pub mod agents;` |
| `src/main.rs` | **MODIFY** | Add `Install` subcommand, refactor `Setup` to use adapters |
| `src/opencode_setup.rs` | **MODIFY** | Migrate logic to `agents/opencode.rs` |

### 3.8 Dependencies

- **Existing**: `serde`, `serde_json`, `anyhow`, `dirs`, `clap`
- **New**: `serde_yaml` (for Hermes config merge) — already in workspace? Check.
- **Optional**: `dialoguer` or `indicatif` for interactive selection (can defer)

### 3.9 Risks & Mitigations

| Risk | Mitigation |
|---|---|
| YAML merge is tricky (Hermes) | Use `serde_yaml` to parse → modify Value → serialize back. Preserve comments? May lose them. |
| Claude Code prefers `claude mcp add` over file editing | Support both: try CLI first, fall back to `.mcp.json` writing |
| Config file permissions | Check write permissions before attempting |
| NTFS I/O slowness | Minimal file ops (read + write only) — no scanning |
| Aider has no MCP support | Document clearly, don't list as supported agent |
| Windsurf config path varies by version | Check both `~/.codeium/windsurf/` and `%APPDATA%` paths |

### 3.10 Future Extensions

- `engram install --interactive` — TUI picker with checkbox for each agent
- `engram install aider` — Write AGENTS.md / .aider.conf.yml with memory protocol instructions
- `engram install --remote` — Configure remote HTTP transport instead of stdio
- `engram status` — Show which agents are configured and their connection status
- Plugin system for community adapters (`~/.engram/adapters/xxx.rs`)

---

## 4. Implementation Order

### Phase 1: Foundation ( adapters trait + registry )
1. Create `crates/mcp/src/agents/mod.rs` with trait definitions
2. Create `crates/mcp/src/agents/registry.rs` with auto-detect
3. Add `pub mod agents` to `crates/mcp/src/lib.rs`

### Phase 2: Agent Adapters ( easy ones first )
4. Implement `cursor.rs` (simplest — just JSON merge with mcpServers key)
5. Implement `windsurf.rs` (same format as Cursor, different path)
6. Implement `claude_code.rs` (.mcp.json writing + optional `claude mcp add-json`)
7. Implement `hermes.rs` (YAML config merge)
8. Migrate `opencode.rs` from existing code

### Phase 3: CLI Integration
9. Add `Install` subcommand to `main.rs`
10. Wire up `engram install` → registry.detect() → install all detected
11. Wire up `engram install <agent>` → single adapter install
12. Add `--detect`, `--dry-run`, `--uninstall` flags

### Phase 4: Polish
13. Add `engram doctor --all` to verify all registered agents
14. Update `engram setup` to delegate to adapter pattern
15. Tests for each adapter (config merge edge cases)

---

## 5. Open Questions for Ale

1. **Aider**: Drop from the list entirely? Or add as "documentation only" (writes AGENTS.md)?
2. **Claude Code**: Prefer `claude mcp add-json` CLI (requires claude installed) or direct `.mcp.json` file writing?
3. **Hermes adapter**: Should `engram install hermes` modify `config.yaml` in-place or just print the snippet?
4. **Gemini CLI**: Keep in the list? Currently only writes a .md file with no MCP config.
5. **Interactive mode**: Want TUI picker in v1 or defer?
