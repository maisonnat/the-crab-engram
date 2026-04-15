use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

const PLUGIN_TS: &str = include_str!("../../../plugins/opencode/the-crab-engram.ts");

pub fn plugin_content() -> &'static str {
    PLUGIN_TS
}

pub fn write_plugin(plugin_dir: &Path) -> Result<bool> {
    let content = plugin_content();
    let target = plugin_dir.join("the-crab-engram.ts");

    if target.exists() {
        let existing = std::fs::read(&target)?;
        let existing_hash = compute_hash(&existing);
        let new_hash = compute_hash(content.as_bytes());
        if existing_hash == new_hash {
            return Ok(false);
        }
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, content)?;
    Ok(true)
}

pub fn remove_plugin(plugin_dir: &Path) -> Result<bool> {
    let target = plugin_dir.join("the-crab-engram.ts");
    if target.exists() {
        std::fs::remove_file(&target)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
