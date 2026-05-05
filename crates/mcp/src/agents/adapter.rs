use std::path::PathBuf;

use anyhow::Result;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// What action was taken for a specific config target.
#[derive(Clone, Debug)]
pub enum ActionKind {
    Created,
    Updated,
    Skipped,
    Removed,
    Failed(String),
}

/// A single action in the setup result.
#[derive(Clone, Debug)]
pub struct SetupAction {
    pub action: ActionKind,
    pub target: String,
    pub detail: String,
}

/// Result of an install/uninstall operation.
#[derive(Clone, Debug, Default)]
pub struct SetupResult {
    pub actions: Vec<SetupAction>,
}

impl SetupResult {
    pub fn display_table(&self) {
        println!("{:<12} {:<50} DETAIL", "ACTION", "TARGET");
        println!("{}", "-".repeat(80));
        for a in &self.actions {
            let kind = match &a.action {
                ActionKind::Created => "Created",
                ActionKind::Updated => "Updated",
                ActionKind::Skipped => "Skipped",
                ActionKind::Removed => "Removed",
                ActionKind::Failed(e) => {
                    println!("{:<12} {:<50} FAILED: {}", "Failed", a.target, e);
                    continue;
                }
            };
            println!("{:<12} {:<50} {}", kind, a.target, a.detail);
        }
    }
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

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

    /// Install: register Engram MCP in this agent's config.
    fn install(&self, dry_run: bool) -> Result<SetupResult>;

    /// Uninstall: remove Engram MCP entry.
    fn uninstall(&self, dry_run: bool) -> Result<SetupResult>;
}

// ---------------------------------------------------------------------------
// Build helpers
// ---------------------------------------------------------------------------

const DEFAULT_DB_PATH: &str = "/home/maiso/.engram/engram.db";

/// Build the standard MCP server entry for the agents JSON format (mcpServers).
pub fn build_mcp_entry_json(profile: &str) -> serde_json::Value {
    serde_json::json!({
        "engram": {
            "command": "the-crab-engram",
            "args": ["--db", DEFAULT_DB_PATH, "mcp", "--profile", profile]
        }
    })
}

/// Build the standard MCP entry YAML string for Hermes.
pub fn build_mcp_entry_yaml(profile: &str) -> String {
    format!(
        r#"  engram:
    command: "the-crab-engram"
    args: ["--db", "{}", "mcp", "--profile", "{}"]
    timeout: 30
"#,
        DEFAULT_DB_PATH, profile
    )
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Return all known agent adapters.
pub fn all_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(crate::agents::claude_code::ClaudeCodeAdapter::new()),
        Box::new(crate::agents::cursor::CursorAdapter::new()),
        Box::new(crate::agents::windsurf::WindsurfAdapter::new()),
        Box::new(crate::agents::opencode::OpenCodeAdapter::new()),
        Box::new(crate::agents::hermes::HermesAdapter::new()),
    ]
}

/// Return only adapters for agents detected on this system.
pub fn detect_installed() -> Vec<Box<dyn AgentAdapter>> {
    all_adapters()
        .into_iter()
        .filter(|a| a.detect().unwrap_or(false))
        .collect()
}

/// Find a specific adapter by key.
pub fn detect_agent(key: &str) -> Option<Box<dyn AgentAdapter>> {
    all_adapters().into_iter().find(|a| a.key() == key)
}
