use anyhow::{Context, Result};
use engram_mcp::config_merge::{
    self, ActionKind, SetupAction, SetupResult, generate_memory_protocol, merge_agents_md,
};
use engram_mcp::doctor::{CheckResult, CheckStatus, DoctorCheck};
use engram_mcp::opencode_paths::OpenCodePaths;
use engram_mcp::plugin_template;

pub fn setup_opencode(
    paths: &OpenCodePaths,
    profile: &str,
    project: &str,
    dry_run: bool,
) -> Result<SetupResult> {
    let mut actions = Vec::new();

    if dry_run {
        if !paths.config_file.exists() {
            actions.push(SetupAction {
                action: ActionKind::Created,
                target: paths.config_file.display().to_string(),
                detail: "Would create config with MCP entry + plugin".to_string(),
            });
        } else {
            actions.push(SetupAction {
                action: ActionKind::Updated,
                target: paths.config_file.display().to_string(),
                detail: "Would merge MCP entry + plugin path".to_string(),
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
                detail: "Would copy plugin file".to_string(),
            });
        } else {
            actions.push(SetupAction {
                action: ActionKind::Skipped,
                target: paths
                    .plugin_dir
                    .join("the-crab-engram.ts")
                    .display()
                    .to_string(),
                detail: "Plugin file already exists".to_string(),
            });
        }

        if !paths.agents_file.exists() {
            actions.push(SetupAction {
                action: ActionKind::Created,
                target: paths.agents_file.display().to_string(),
                detail: "Would create AGENTS.md with Memory Protocol".to_string(),
            });
        } else {
            actions.push(SetupAction {
                action: ActionKind::Updated,
                target: paths.agents_file.display().to_string(),
                detail: "Would inject Memory Protocol".to_string(),
            });
        }

        return Ok(SetupResult { actions });
    }

    paths.ensure_dirs()?;

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
            detail: "Created config with MCP entry + plugin".to_string(),
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
                detail: "Merged MCP entry + plugin path".to_string(),
            });
        } else {
            actions.push(SetupAction {
                action: ActionKind::Skipped,
                target: paths.config_file.display().to_string(),
                detail: "Config already up-to-date".to_string(),
            });
        }
    }

    match plugin_template::write_plugin(&paths.plugin_dir) {
        Ok(true) => {
            actions.push(SetupAction {
                action: ActionKind::Created,
                target: paths
                    .plugin_dir
                    .join("the-crab-engram.ts")
                    .display()
                    .to_string(),
                detail: "Plugin file written".to_string(),
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
                detail: "Plugin file unchanged (hash match)".to_string(),
            });
        }
        Err(e) => {
            actions.push(SetupAction {
                action: ActionKind::Removed,
                target: paths
                    .plugin_dir
                    .join("the-crab-engram.ts")
                    .display()
                    .to_string(),
                detail: format!("Failed to write plugin: {e}"),
            });
        }
    }

    if paths.agents_file.exists() {
        let existing = std::fs::read_to_string(&paths.agents_file)?;
        let protocol = generate_memory_protocol();
        let merged = merge_agents_md(&existing, &protocol);
        if merged != existing {
            std::fs::write(&paths.agents_file, &merged)?;
            actions.push(SetupAction {
                action: ActionKind::Updated,
                target: paths.agents_file.display().to_string(),
                detail: "Memory Protocol injected".to_string(),
            });
        } else {
            actions.push(SetupAction {
                action: ActionKind::Skipped,
                target: paths.agents_file.display().to_string(),
                detail: "Memory Protocol already present".to_string(),
            });
        }
    } else {
        let protocol = generate_memory_protocol();
        let content = format!(
            "<!-- gentle-ai:engram-protocol -->\n{}\n<!-- /gentle-ai:engram-protocol -->\n",
            protocol
        );
        std::fs::write(&paths.agents_file, &content)?;
        actions.push(SetupAction {
            action: ActionKind::Created,
            target: paths.agents_file.display().to_string(),
            detail: "AGENTS.md created with Memory Protocol".to_string(),
        });
    }

    Ok(SetupResult { actions })
}

pub fn uninstall_opencode(paths: &OpenCodePaths, dry_run: bool) -> Result<SetupResult> {
    let mut actions = Vec::new();

    if dry_run {
        if paths.config_file.exists() {
            actions.push(SetupAction {
                action: ActionKind::Removed,
                target: paths.config_file.display().to_string(),
                detail: "Would remove MCP entry and plugin path".to_string(),
            });
        }
        let plugin = paths.plugin_dir.join("the-crab-engram.ts");
        if plugin.exists() {
            actions.push(SetupAction {
                action: ActionKind::Removed,
                target: plugin.display().to_string(),
                detail: "Would delete plugin file".to_string(),
            });
        }
        return Ok(SetupResult { actions });
    }

    if paths.config_file.exists() {
        let raw = std::fs::read_to_string(&paths.config_file)?;
        let clean = if paths.is_jsonc {
            config_merge::strip_jsonc_comments(&raw)
        } else {
            raw.clone()
        };
        let config: serde_json::Value = serde_json::from_str(&clean)
            .with_context(|| format!("failed to parse {}", paths.config_file.display()))?;

        let cleaned = config_merge::remove_mcp_entry(&config);
        let json = serde_json::to_string_pretty(&cleaned)?;
        std::fs::write(&paths.config_file, &json)?;
        actions.push(SetupAction {
            action: ActionKind::Removed,
            target: paths.config_file.display().to_string(),
            detail: "MCP entry and plugin path removed".to_string(),
        });
    }

    match plugin_template::remove_plugin(&paths.plugin_dir) {
        Ok(true) => {
            actions.push(SetupAction {
                action: ActionKind::Removed,
                target: paths
                    .plugin_dir
                    .join("the-crab-engram.ts")
                    .display()
                    .to_string(),
                detail: "Plugin file deleted".to_string(),
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
                detail: "Plugin file not found".to_string(),
            });
        }
        Err(e) => {
            anyhow::bail!("Failed to remove plugin: {e}");
        }
    }

    Ok(SetupResult { actions })
}

pub fn check_database() -> CheckResult {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            return CheckResult {
                name: DoctorCheck::DatabaseOk.name().to_string(),
                status: CheckStatus::Fail,
                message: "Cannot determine home directory".to_string(),
                fix_command: None,
            };
        }
    };
    let db_path = home.join(".engram").join("engram.db");
    if !db_path.exists() {
        return CheckResult {
            name: DoctorCheck::DatabaseOk.name().to_string(),
            status: CheckStatus::Warn,
            message: "Database not found (will be created on first use)".to_string(),
            fix_command: None,
        };
    }
    match engram_store::SqliteStore::new(&db_path) {
        Ok(_) => CheckResult {
            name: DoctorCheck::DatabaseOk.name().to_string(),
            status: CheckStatus::Pass,
            message: "integrity_check: ok".to_string(),
            fix_command: None,
        },
        Err(e) => CheckResult {
            name: DoctorCheck::DatabaseOk.name().to_string(),
            status: CheckStatus::Fail,
            message: format!("Database error: {e}"),
            fix_command: None,
        },
    }
}
