use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct OpenCodePaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub plugin_dir: PathBuf,
    pub agents_file: PathBuf,
    pub is_jsonc: bool,
}

impl OpenCodePaths {
    pub fn detect() -> Result<Self> {
        let config_dir = if let Ok(dir) = std::env::var("OPENCODE_CONFIG_DIR") {
            PathBuf::from(dir)
        } else if let Some(dir) = dirs::config_dir() {
            dir.join("opencode")
        } else {
            PathBuf::from(".opencode")
        };

        let (config_file, is_jsonc) = Self::detect_json_format(&config_dir);

        let plugin_dir = config_dir.join("plugins");
        let agents_file = config_dir.join("AGENTS.md");

        Ok(Self {
            config_dir,
            config_file,
            plugin_dir,
            agents_file,
            is_jsonc,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)
            .with_context(|| format!("failed to create {}", self.config_dir.display()))?;
        std::fs::create_dir_all(&self.plugin_dir)
            .with_context(|| format!("failed to create {}", self.plugin_dir.display()))?;
        Ok(())
    }

    pub fn detect_json_format(config_dir: &std::path::Path) -> (PathBuf, bool) {
        let jsonc = config_dir.join("opencode.jsonc");
        let json = config_dir.join("opencode.json");

        if jsonc.exists() {
            (jsonc, true)
        } else {
            (json, false)
        }
    }
}
