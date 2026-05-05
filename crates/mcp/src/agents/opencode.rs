use std::path::PathBuf;

use anyhow::{Context, Result};

use super::adapter::{ActionKind, AgentAdapter, SetupAction, SetupResult};
use crate::config_merge::{self, generate_memory_protocol, merge_agents_md};
use crate::opencode_paths::OpenCodePaths;
use crate::plugin_template;

pub struct OpenCodeAdapter;

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenCodeAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl AgentAdapter for OpenCodeAdapter {
    fn name(&self) -> &str {
        "OpenCode"
    }

    fn key(&self) -> &str {
        "opencode"
    }

    fn detect(&self) -> Result<bool> {
        Ok(which::which("opencode").is_ok()
            || OpenCodePaths::detect()
                .map(|p| p.config_file.exists())
                .unwrap_or(false))
    }

    fn config_paths(&self) -> Result<Vec<PathBuf>> {
        let paths = OpenCodePaths::detect()?;
        Ok(vec![paths.config_file])
    }

    fn install(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        let paths = OpenCodePaths::detect()?;
        let profile = "agent";
        let project = "default";

        if dry_run {
            if !paths.config_file.exists() {
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: paths.config_file.display().to_string(),
                    detail: "Would create config with MCP entry + plugin".into(),
                });
            } else {
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: paths.config_file.display().to_string(),
                    detail: "Would merge MCP entry + plugin path".into(),
                });
            }
            if !paths.plugin_dir.join("the-crab-engram.ts").exists() {
                actions.push(SetupAction {
                    action: ActionKind::Created,
                    target: paths
                        .plugin_dir
                        .join("the-crab-engram.ts")
                        .display()
                        .to_string(),
                    detail: "Would copy plugin file".into(),
                });
            } else {
                actions.push(SetupAction {
                    action: ActionKind::Skipped,
                    target: paths
                        .plugin_dir
                        .join("the-crab-engram.ts")
                        .display()
                        .to_string(),
                    detail: "Plugin file already exists".into(),
                });
            }
            return Ok(SetupResult { actions });
        }

        paths.ensure_dirs()?;

        // Write/update config
        if !paths.config_file.exists() {
            let config = serde_json::json!({});
            let config = config_merge::merge_mcp_entry(&config, profile, project);
            let plugin_path = "./plugins/the-crab-engram.ts";
            let config = config_merge::merge_plugin_path(&config, plugin_path);
            let json = serde_json::to_string_pretty(&config)?;
            std::fs::write(&paths.config_file, &json)?;
            actions.push(SetupAction {
                action: ActionKind::Created,
                target: paths.config_file.display().to_string(),
                detail: "Created config with MCP entry + plugin".into(),
            });
        } else {
            let raw = std::fs::read_to_string(&paths.config_file)?;
            let clean = if paths.is_jsonc {
                config_merge::strip_jsonc_comments(&raw)
            } else {
                raw.clone()
            };
            let mut config: serde_json::Value = serde_json::from_str(&clean)
                .with_context(|| format!("failed to parse {}", paths.config_file.display()))?;
            let original = config.clone();
            config = config_merge::merge_mcp_entry(&config, profile, project);
            let plugin_path = "./plugins/the-crab-engram.ts";
            config = config_merge::merge_plugin_path(&config, plugin_path);
            if config != original {
                let json = serde_json::to_string_pretty(&config)?;
                std::fs::write(&paths.config_file, &json)?;
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: paths.config_file.display().to_string(),
                    detail: "Merged MCP entry + plugin path".into(),
                });
            } else {
                actions.push(SetupAction {
                    action: ActionKind::Skipped,
                    target: paths.config_file.display().to_string(),
                    detail: "Config already up-to-date".into(),
                });
            }
        }

        // Write plugin file
        match plugin_template::write_plugin(&paths.plugin_dir) {
            Ok(true) => {
                actions.push(SetupAction {
                    action: ActionKind::Created,
                    target: paths
                        .plugin_dir
                        .join("the-crab-engram.ts")
                        .display()
                        .to_string(),
                    detail: "Plugin file written".into(),
                });
            }
            Ok(false) => {
                actions.push(SetupAction {
                    action: ActionKind::Skipped,
                    target: paths
                        .plugin_dir
                        .join("the-crab-engram.ts")
                        .display()
                        .to_string(),
                    detail: "Plugin file unchanged (hash match)".into(),
                });
            }
            Err(e) => {
                actions.push(SetupAction {
                    action: ActionKind::Failed(e.to_string()),
                    target: paths
                        .plugin_dir
                        .join("the-crab-engram.ts")
                        .display()
                        .to_string(),
                    detail: format!("Failed to write plugin: {e}"),
                });
            }
        }

        // Write AGENTS.md
        if paths.agents_file.exists() {
            let existing = std::fs::read_to_string(&paths.agents_file)?;
            let protocol = generate_memory_protocol();
            let merged = merge_agents_md(&existing, &protocol);
            if merged != existing {
                std::fs::write(&paths.agents_file, &merged)?;
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: paths.agents_file.display().to_string(),
                    detail: "Memory Protocol injected".into(),
                });
            } else {
                actions.push(SetupAction {
                    action: ActionKind::Skipped,
                    target: paths.agents_file.display().to_string(),
                    detail: "Memory Protocol already present".into(),
                });
            }
        } else {
            let protocol = generate_memory_protocol();
            let content = format!(
                "<!-- gentle-ai:engram-protocol -->\n{protocol}\n<!-- /gentle-ai:engram-protocol -->\n"
            );
            std::fs::write(&paths.agents_file, &content)?;
            actions.push(SetupAction {
                action: ActionKind::Created,
                target: paths.agents_file.display().to_string(),
                detail: "AGENTS.md created with Memory Protocol".into(),
            });
        }

        Ok(SetupResult { actions })
    }

    fn uninstall(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        let paths = OpenCodePaths::detect()?;

        if dry_run {
            if paths.config_file.exists() {
                actions.push(SetupAction {
                    action: ActionKind::Removed,
                    target: paths.config_file.display().to_string(),
                    detail: "Would remove MCP entry and plugin path".into(),
                });
            }
            let plugin = paths.plugin_dir.join("the-crab-engram.ts");
            if plugin.exists() {
                actions.push(SetupAction {
                    action: ActionKind::Removed,
                    target: plugin.display().to_string(),
                    detail: "Would remove plugin file".into(),
                });
            }
            return Ok(SetupResult { actions });
        }

        // Remove from config
        if paths.config_file.exists() {
            let raw = std::fs::read_to_string(&paths.config_file)?;
            let clean = if paths.is_jsonc {
                config_merge::strip_jsonc_comments(&raw)
            } else {
                raw.clone()
            };
            let mut config: serde_json::Value = serde_json::from_str(&clean)
                .with_context(|| format!("failed to parse {}", paths.config_file.display()))?;
            let original = config.clone();

            // Remove MCP entry
            if let Some(mcp) = config.get_mut("mcp")
                && let Some(arr) = mcp.as_array_mut()
            {
                arr.retain(|e| e.get("name").and_then(|n| n.as_str()) != Some("engram"));
            }
            // Remove plugin path
            if let Some(plugin) = config.get_mut("plugin")
                && let Some(arr) = plugin.as_array_mut()
            {
                arr.retain(|p| p.as_str() != Some("./plugins/the-crab-engram.ts"));
            }

            if config != original {
                let json = serde_json::to_string_pretty(&config)?;
                std::fs::write(&paths.config_file, &json)?;
                actions.push(SetupAction {
                    action: ActionKind::Removed,
                    target: paths.config_file.display().to_string(),
                    detail: "Removed MCP entry and plugin path".into(),
                });
            } else {
                actions.push(SetupAction {
                    action: ActionKind::Skipped,
                    target: paths.config_file.display().to_string(),
                    detail: "No engram entries found in config".into(),
                });
            }
        }

        // Remove plugin file
        let plugin = paths.plugin_dir.join("the-crab-engram.ts");
        if plugin.exists() {
            std::fs::remove_file(&plugin)?;
            actions.push(SetupAction {
                action: ActionKind::Removed,
                target: plugin.display().to_string(),
                detail: "Removed plugin file".into(),
            });
        }

        Ok(SetupResult { actions })
    }
}
