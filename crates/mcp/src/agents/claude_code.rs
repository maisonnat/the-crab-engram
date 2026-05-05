use std::path::PathBuf;

use anyhow::{Context, Result};
use dirs::home_dir;

use super::adapter::{ActionKind, AgentAdapter, SetupAction, SetupResult, build_mcp_entry_json};

pub struct ClaudeCodeAdapter;

impl Default for ClaudeCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    fn project_config_path(&self) -> PathBuf {
        let cwd = std::env::current_dir().unwrap_or_default();
        cwd.join(".mcp.json")
    }

    fn user_config_path(&self) -> Option<PathBuf> {
        home_dir().map(|h| h.join(".claude").join("settings.json"))
    }
}

impl AgentAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &str {
        "Claude Code"
    }

    fn key(&self) -> &str {
        "claude-code"
    }

    fn detect(&self) -> Result<bool> {
        // Check for Claude Code binary
        Ok(which::which("claude").is_ok())
    }

    fn config_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        if self.project_config_path().exists() {
            paths.push(self.project_config_path());
        }
        if let Some(user) = self.user_config_path()
            && user.exists()
        {
            paths.push(user);
        }
        Ok(paths)
    }

    fn install(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        let entry = build_mcp_entry_json("agent");

        // Try project-level config first
        let project_path = self.project_config_path();
        if project_path.exists() {
            if dry_run {
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: project_path.display().to_string(),
                    detail: "Would merge engram MCP entry".into(),
                });
            } else {
                let raw = std::fs::read_to_string(&project_path)?;
                let mut config: serde_json::Value =
                    serde_json::from_str(&raw).context("failed to parse .mcp.json")?;
                let original = config.clone();
                config["mcpServers"]["engram"] = entry["engram"].clone();
                if config != original {
                    let json = serde_json::to_string_pretty(&config)?;
                    std::fs::write(&project_path, &json)?;
                    actions.push(SetupAction {
                        action: ActionKind::Updated,
                        target: project_path.display().to_string(),
                        detail: "Merged engram MCP entry".into(),
                    });
                } else {
                    actions.push(SetupAction {
                        action: ActionKind::Skipped,
                        target: project_path.display().to_string(),
                        detail: "Already up-to-date".into(),
                    });
                }
            }
        } else {
            // Create new .mcp.json
            if dry_run {
                actions.push(SetupAction {
                    action: ActionKind::Created,
                    target: project_path.display().to_string(),
                    detail: "Would create .mcp.json with engram entry".into(),
                });
            } else {
                let config = serde_json::json!({ "mcpServers": entry });
                let json = serde_json::to_string_pretty(&config)?;
                std::fs::write(&project_path, &json)?;
                actions.push(SetupAction {
                    action: ActionKind::Created,
                    target: project_path.display().to_string(),
                    detail: "Created .mcp.json with engram entry".into(),
                });
            }
        }

        Ok(SetupResult { actions })
    }

    fn uninstall(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        let project_path = self.project_config_path();

        if project_path.exists() {
            if dry_run {
                actions.push(SetupAction {
                    action: ActionKind::Removed,
                    target: project_path.display().to_string(),
                    detail: "Would remove engram MCP entry".into(),
                });
            } else {
                let raw = std::fs::read_to_string(&project_path)?;
                let mut config: serde_json::Value =
                    serde_json::from_str(&raw).context("failed to parse .mcp.json")?;
                if let Some(obj) = config.get_mut("mcpServers")
                    && let Some(map) = obj.as_object_mut()
                    && map.remove("engram").is_some()
                {
                    let json = serde_json::to_string_pretty(&config)?;
                    std::fs::write(&project_path, &json)?;
                    actions.push(SetupAction {
                        action: ActionKind::Removed,
                        target: project_path.display().to_string(),
                        detail: "Removed engram MCP entry".into(),
                    });
                }
            }
        }

        Ok(SetupResult { actions })
    }
}
