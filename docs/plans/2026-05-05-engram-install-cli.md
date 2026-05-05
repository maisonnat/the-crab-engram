# `engram install` — Universal MCP Registration CLI

> **Goal:** Implement `the-crab-engram install [--all|--detect|agent-name]` that auto-detects AI agents and registers the Engram MCP server in their config.

**Architecture:**
- `AgentAdapter` trait in new module `crates/mcp/src/agents/`
- One adapter per agent (Claude Code, Cursor, Windsurf, OpenCode, Hermes)
- Registry for auto-detection  
- Reuses existing `OpenCodePaths` + `config_merge` for OpenCode
- New code for the other 4 agents

**3 config formats:**
| Family | Agents | Format | Key |
|---|---|---|---|
| `mcpServers` (split) | Claude Code, Cursor, Windsurf | JSON | `mcpServers` |
| `mcp` (unified array) | OpenCode | JSON | `mcp` |
| `mcp_servers` (YAML) | Hermes | YAML | `mcp_servers` |

---

### Task 1: Create AgentAdapter trait + AgentRegistry

**Objective:** Define the trait and registry in `crates/mcp/src/agents/`.

**Files:**
- Create: `crates/mcp/src/agents/mod.rs`
- Create: `crates/mcp/src/agents/adapter.rs`

**Trait definition:**
```rust
/// Adapter for a specific AI agent's MCP config.
pub trait AgentAdapter: Send + Sync {
    /// Human-readable agent name (e.g., "Claude Code").
    fn name(&self) -> &str;

    /// Unique key for CLI arg (e.g., "claude-code").
    fn key(&self) -> &str;

    /// Detect if this agent is installed on the system.
    fn detect(&self) -> Result<bool>;

    /// Path(s) to the config file that needs MCP entry.
    fn config_paths(&self) -> Result<Vec<PathBuf>>;

    /// Build the MCP entry JSON/YAML for this agent.
    fn build_mcp_entry(&self, db_path: &Path, profile: &str) -> serde_json::Value;

    /// Install: register Engram MCP in this agent's config.
    fn install(&self, dry_run: bool) -> Result<SetupResult>;

    /// Uninstall: remove Engram MCP entry.
    fn uninstall(&self, dry_run: bool) -> Result<SetupResult>;
}
```

**Registry:**
```rust
pub fn all_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(ClaudeCodeAdapter::new()),
        Box::new(CursorAdapter::new()),
        Box::new(WindsurfAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
        Box::new(HermesAdapter::new()),
    ]
}

pub fn detect_installed() -> Vec<Box<dyn AgentAdapter>> {
    all_adapters().into_iter().filter(|a| a.detect().unwrap_or(false)).collect()
}
```

---

### Task 2: Implement ClaudeCodeAdapter

**Objective:** Register MCP in Claude Code's `.mcp.json` (project) or `settings.json` (user).

**Detection:** Check for `~/.claude/.mcp.json` or `~/.claude/settings.json`

**Config format:**
```json
{
  "mcpServers": {
    "engram": {
      "command": "the-crab-engram",
      "args": ["--db", "/home/maiso/.engram/engram.db", "mcp", "--profile", "agent"]
    }
  }
}
```

**Files:**
- Create: `crates/mcp/src/agents/claude_code.rs`

---

### Task 3: Implement CursorAdapter

**Objective:** Register MCP in Cursor's config.

**Detection:** Check for `~/.cursor/mcp.json` or `~/.cursor/config.json`

**Files:**
- Create: `crates/mcp/src/agents/cursor.rs`
- Uses same `mcpServers` format as Claude Code

---

### Task 4: Implement WindsurfAdapter

**Objective:** Register MCP in Windsurf's config.

**Detection:** Check for `~/.windsurf/mcp_config.json` or similar

**Files:**
- Create: `crates/mcp/src/agents/windsurf.rs`

---

### Task 5: Implement HermesAdapter

**Objective:** Register MCP in Hermes `~/.hermes/config.yaml`.

**Detection:** Check for `~/.hermes/config.yaml`

**Config format (YAML):**
```yaml
mcp_servers:
  engram:
    command: "the-crab-engram"
    args: ["--db", "/home/maiso/.engram/engram.db", "mcp", "--profile", "all"]
    timeout: 30
```

**Files:**
- Create: `crates/mcp/src/agents/hermes.rs`
- Uses `serde_yaml` for YAML manipulation

---

### Task 6: Integrate OpenCodeAdapter (wraps existing setup)

**Objective:** Wrap the existing `opencode_setup.rs` + `opencode_paths.rs` + `config_merge.rs` behind the AgentAdapter trait.

**Files:**
- Create: `crates/mcp/src/agents/opencode.rs`

```rust
pub struct OpenCodeAdapter;

impl AgentAdapter for OpenCodeAdapter {
    fn name(&self) -> &str { "OpenCode" }
    fn key(&self) -> &str { "opencode" }
    
    fn detect(&self) -> Result<bool> {
        OpenCodePaths::detect().is_ok()
    }
    
    fn install(&self, dry_run: bool) -> Result<SetupResult> {
        let paths = OpenCodePaths::detect()?;
        setup_opencode(&paths, "agent", "default", dry_run)
    }
}
```

---

### Task 7: Add `install` subcommand to CLI

**Objective:** Add `Commands::Install` to the CLI enum in `src/main.rs` and wire it up.

```rust
/// Register Engram MCP with AI agents
Install {
    /// Agent to install for (omit for auto-detect)
    #[arg(value_enum)]
    agent: Option<AgentArg>,

    /// Install for all detected agents
    #[arg(long)]
    all: bool,

    /// Show what would be done without writing
    #[arg(long)]
    dry_run: bool,
},
```

**Handler:**
```rust
Commands::Install { agent, all, dry_run } => {
    let agents: Vec<Box<dyn AgentAdapter>> = if all {
        crate::agents::all_adapters()
    } else if let Some(a) = agent {
        // Find specific adapter
        ...
    } else {
        crate::agents::detect_installed()
    };
    
    for adapter in agents {
        let result = adapter.install(dry_run)?;
        println!("{}:", adapter.name());
        result.display_table();
    }
}
```

---

### Task 8: Register module and wire Cargo.toml

**Files:**
- Modify: `crates/mcp/src/lib.rs` — add `pub mod agents;`
- Modify: `Cargo.toml` — add `serde_yaml` dep if not present
- Modify: `src/main.rs` — add `use engram_mcp::agents;` and wire Install handler

---

### Verification
1. `cargo build --release` — compiles
2. `the-crab-engram install` — detects installed agents, shows what would be done
3. `the-crab-engram install --dry-run` — shows without writing
4. `the-crab-engram install --all` — installs to all detected agents
5. `the-crab-engram install claude-code` — installs only to Claude Code
6. Each agent's config file has valid MCP entry
