use std::path::PathBuf;

use anyhow::{Context, Result};
use dirs::home_dir;

use super::adapter::{ActionKind, AgentAdapter, SetupAction, SetupResult, build_mcp_entry_json};

pub struct CursorAdapter;

impl Default for CursorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorAdapter {
    pub fn new() -> Self {
        Self
    }

    fn mcp_path(&self) -> Option<PathBuf> {
        Some(
            home_dir()
                .unwrap_or_default()
                .join(".cursor")
                .join("mcp.json"),
        )
    }
}

impl AgentAdapter for CursorAdapter {
    fn name(&self) -> &str {
        "Cursor"
    }

    fn key(&self) -> &str {
        "cursor"
    }

    fn detect(&self) -> Result<bool> {
        Ok(home_dir()
            .map(|h| h.join(".cursor").exists())
            .unwrap_or(false))
    }

    fn config_paths(&self) -> Result<Vec<PathBuf>> {
        Ok(self
            .mcp_path()
            .filter(|p| p.exists())
            .map(|p| vec![p])
            .unwrap_or_default())
    }

    fn install(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        let path = self
            .mcp_path()
            .unwrap_or_else(|| PathBuf::from(".cursor/mcp.json"));
        let entry = build_mcp_entry_json("agent");

        if path.exists() {
            if dry_run {
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: path.display().to_string(),
                    detail: "Would merge engram MCP entry".into(),
                });
            } else {
                let raw = std::fs::read_to_string(&path)?;
                let mut config: serde_json::Value =
                    serde_json::from_str(&raw).context("failed to parse mcp.json")?;
                let original = config.clone();
                config["mcpServers"]["engram"] = entry["engram"].clone();
                if config != original {
                    let json = serde_json::to_string_pretty(&config)?;
                    std::fs::write(&path, &json)?;
                    actions.push(SetupAction {
                        action: ActionKind::Updated,
                        target: path.display().to_string(),
                        detail: "Merged engram MCP entry".into(),
                    });
                } else {
                    actions.push(SetupAction {
                        action: ActionKind::Skipped,
                        target: path.display().to_string(),
                        detail: "Already up-to-date".into(),
                    });
                }
            }
        } else {
            if dry_run {
                actions.push(SetupAction {
                    action: ActionKind::Created,
                    target: path.display().to_string(),
                    detail: "Would create mcp.json with engram entry".into(),
                });
            } else {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let config = serde_json::json!({ "mcpServers": entry });
                let json = serde_json::to_string_pretty(&config)?;
                std::fs::write(&path, &json)?;
                actions.push(SetupAction {
                    action: ActionKind::Created,
                    target: path.display().to_string(),
                    detail: "Created mcp.json with engram entry".into(),
                });
            }
        }

        Ok(SetupResult { actions })
    }

    fn uninstall(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        if let Some(path) = self.mcp_path()
            && path.exists()
        {
            if dry_run {
                actions.push(SetupAction {
                    action: ActionKind::Removed,
                    target: path.display().to_string(),
                    detail: "Would remove engram MCP entry".into(),
                });
            } else {
                let raw = std::fs::read_to_string(&path)?;
                let mut config: serde_json::Value =
                    serde_json::from_str(&raw).context("failed to parse mcp.json")?;
                if let Some(obj) = config.get_mut("mcpServers")
                    && let Some(map) = obj.as_object_mut()
                    && map.remove("engram").is_some()
                {
                    let json = serde_json::to_string_pretty(&config)?;
                    std::fs::write(&path, &json)?;
                    actions.push(SetupAction {
                        action: ActionKind::Removed,
                        target: path.display().to_string(),
                        detail: "Removed engram MCP entry".into(),
                    });
                }
            }
        }
        Ok(SetupResult { actions })
    }
}
