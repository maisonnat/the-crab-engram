use std::path::PathBuf;

use anyhow::Result;
use dirs::home_dir;

use super::adapter::{ActionKind, AgentAdapter, SetupAction, SetupResult, build_mcp_entry_yaml};

pub struct HermesAdapter;

impl Default for HermesAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl HermesAdapter {
    pub fn new() -> Self {
        Self
    }

    fn config_path(&self) -> Option<PathBuf> {
        Some(
            home_dir()
                .unwrap_or_default()
                .join(".hermes")
                .join("config.yaml"),
        )
    }

    /// Set memory.provider: engram in Hermes config.yaml.
    fn _set_memory_provider(&self, actions: &mut Vec<SetupAction>, dry_run: bool) -> Result<()> {
        let Some(path) = self.config_path() else {
            return Ok(());
        };

        let provider_line = "  provider: engram";

        if dry_run {
            actions.push(SetupAction {
                action: ActionKind::Updated,
                target: path.display().to_string(),
                detail: "Would set memory.provider to 'engram'".into(),
            });
            return Ok(());
        }

        if !path.exists() {
            actions.push(SetupAction {
                action: ActionKind::Skipped,
                target: path.display().to_string(),
                detail: "Hermes config not found — skipping memory provider setup".into(),
            });
            return Ok(());
        }

        let raw = std::fs::read_to_string(&path)?;

        // Already set to engram — skip
        if raw.contains("memory:") && raw.contains(provider_line) {
            actions.push(SetupAction {
                action: ActionKind::Skipped,
                target: path.display().to_string(),
                detail: "memory.provider already set to 'engram'".into(),
            });
            return Ok(());
        }

        let has_memory_section = raw.contains("memory:");
        let has_any_provider = raw.contains("provider:");

        let new_raw = if has_memory_section && !has_any_provider {
            // memory: exists but no provider line — insert one
            let pos = raw.find("memory:").map(|i| i + "memory:".len()).unwrap();
            format!("{}{}\n{}", &raw[..pos], "\n  provider: engram", &raw[pos..])
        } else if !has_memory_section {
            // No memory section at all — append
            format!("{}\n\nmemory:\n  provider: engram\n", raw.trim_end())
        } else {
            // Replace the existing provider value under memory:
            // Only replace the FIRST occurrence (which should be under memory:)
            raw.replacen("  provider:", provider_line, 1)
        };

        std::fs::write(&path, &new_raw)?;
        actions.push(SetupAction {
            action: ActionKind::Updated,
            target: path.display().to_string(),
            detail: "Set memory.provider to 'engram'".into(),
        });

        Ok(())
    }
}

impl AgentAdapter for HermesAdapter {
    fn name(&self) -> &str {
        "Hermes"
    }

    fn key(&self) -> &str {
        "hermes"
    }

    fn detect(&self) -> Result<bool> {
        Ok(which::which("hermes").is_ok()
            || home_dir()
                .map(|h| h.join(".hermes").join("config.yaml").exists())
                .unwrap_or(false))
    }

    fn config_paths(&self) -> Result<Vec<PathBuf>> {
        Ok(self
            .config_path()
            .filter(|p| p.exists())
            .map(|p| vec![p])
            .unwrap_or_default())
    }

    fn install(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        let Some(path) = self.config_path() else {
            return Ok(SetupResult { actions });
        };

        // ── Step 1: Register MCP entry ──────────────────────────────
        let mcp_block = build_mcp_entry_yaml("all");

        if dry_run {
            actions.push(SetupAction {
                action: ActionKind::Updated,
                target: path.display().to_string(),
                detail: format!("Would add mcp_servers.engram section:\n{}", mcp_block),
            });
        } else {
            let raw = std::fs::read_to_string(&path)?;

            if raw.contains("mcp_servers:") && raw.contains("engram:") {
                actions.push(SetupAction {
                    action: ActionKind::Skipped,
                    target: path.display().to_string(),
                    detail: "engram MCP entry already present".into(),
                });
            } else if raw.contains("mcp_servers:") {
                let marker = "mcp_servers:";
                let insert_pos = raw.find(marker).map(|i| i + marker.len()).unwrap_or(0);
                let new_raw = format!(
                    "{}{}\n{}",
                    &raw[..insert_pos],
                    mcp_block,
                    &raw[insert_pos..]
                );
                std::fs::write(&path, &new_raw)?;
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: path.display().to_string(),
                    detail: "Added engram MCP entry under mcp_servers".into(),
                });
            } else {
                let new_raw = format!("{}\nmcp_servers:\n{}", raw, mcp_block);
                std::fs::write(&path, &new_raw)?;
                actions.push(SetupAction {
                    action: ActionKind::Updated,
                    target: path.display().to_string(),
                    detail: "Created mcp_servers section with engram entry".into(),
                });
            }
        }

        // ── Step 2: Set memory.provider = engram ────────────────────
        self._set_memory_provider(&mut actions, dry_run)?;

        Ok(SetupResult { actions })
    }

    fn uninstall(&self, dry_run: bool) -> Result<SetupResult> {
        let mut actions = Vec::new();
        let Some(path) = self.config_path() else {
            return Ok(SetupResult { actions });
        };

        if path.exists() {
            if dry_run {
                actions.push(SetupAction {
                    action: ActionKind::Removed,
                    target: path.display().to_string(),
                    detail: "Would remove engram MCP entry".into(),
                });
            } else {
                let raw = std::fs::read_to_string(&path)?;
                if let Some(start) = raw.find("\n  engram:") {
                    let remaining = &raw[start..];
                    let end = remaining
                        .find("\n\n")
                        .map(|i| start + i)
                        .unwrap_or(raw.len());
                    let new_raw = format!("{}{}", &raw[..start], &raw[end..]);
                    std::fs::write(&path, &new_raw)?;
                    actions.push(SetupAction {
                        action: ActionKind::Removed,
                        target: path.display().to_string(),
                        detail: "Removed engram MCP entry".into(),
                    });
                } else {
                    actions.push(SetupAction {
                        action: ActionKind::Skipped,
                        target: path.display().to_string(),
                        detail: "No engram entry found".into(),
                    });
                }
            }
        }

        Ok(SetupResult { actions })
    }
}
