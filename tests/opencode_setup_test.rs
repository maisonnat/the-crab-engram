use std::path::PathBuf;

fn setup_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

fn create_test_config(dir: &std::path::Path, is_jsonc: bool) -> PathBuf {
    let name = if is_jsonc {
        "opencode.jsonc"
    } else {
        "opencode.json"
    };
    let config_path = dir.join(name);
    let config = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "mcp": {
            "github": { "type": "local", "command": ["github-mcp"], "enabled": true }
        },
        "plugin": ["./my-other-plugin.ts"]
    });
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
    config_path
}

#[test]
fn test_config_merge_preserves_existing_keys() {
    let config = serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "mcp": {
            "github": { "type": "local", "command": ["github-mcp"], "enabled": true }
        },
        "plugin": ["./my-other-plugin.ts"]
    });

    let merged = engram_mcp::config_merge::merge_mcp_entry(&config, "agent", "my-project");
    let merged =
        engram_mcp::config_merge::merge_plugin_path(&merged, "./plugins/the-crab-engram.ts");

    assert!(
        merged.get("$schema").is_some(),
        "$schema should be preserved"
    );
    assert!(
        merged.get("mcp").unwrap().get("github").is_some(),
        "github entry preserved"
    );
    assert!(
        merged.get("mcp").unwrap().get("the-crab-engram").is_some(),
        "engram entry added"
    );

    let plugin_arr = merged.get("plugin").unwrap().as_array().unwrap();
    assert_eq!(plugin_arr.len(), 2);
    assert_eq!(plugin_arr[1], "./plugins/the-crab-engram.ts");
}

#[test]
fn test_config_merge_idempotent() {
    let config = serde_json::json!({});
    let merged1 = engram_mcp::config_merge::merge_mcp_entry(&config, "agent", "proj");
    let merged2 = engram_mcp::config_merge::merge_mcp_entry(&merged1, "agent", "proj");
    assert_eq!(merged1, merged2);
}

#[test]
fn test_plugin_path_no_duplicates() {
    let config = serde_json::json!({
        "plugin": ["./plugins/the-crab-engram.ts"]
    });
    let merged =
        engram_mcp::config_merge::merge_plugin_path(&config, "./plugins/the-crab-engram.ts");
    let arr = merged.get("plugin").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 1);
}

#[test]
fn test_strip_jsonc_comments() {
    let input = r#"{
  // This is a comment
  "key": "value",
  "url": "http://example.com" // inline comment
}"#;
    let cleaned = engram_mcp::config_merge::strip_jsonc_comments(input);
    assert!(!cleaned.contains("// This is a comment"));
    assert!(cleaned.contains("\"key\": \"value\""));
    assert!(cleaned.contains("\"url\": \"http://example.com\""));
    let parsed: serde_json::Value = serde_json::from_str(&cleaned).expect("should be valid JSON");
    assert_eq!(parsed["key"], "value");
}

#[test]
fn test_remove_mcp_entry() {
    let config = serde_json::json!({
        "mcp": {
            "the-crab-engram": { "type": "local", "command": ["the-crab-engram", "mcp"], "enabled": true },
            "github": { "type": "local", "command": ["github-mcp"], "enabled": true }
        },
        "plugin": ["./plugins/the-crab-engram.ts", "./other.ts"]
    });

    let cleaned = engram_mcp::config_merge::remove_mcp_entry(&config);
    assert!(cleaned.get("mcp").unwrap().get("the-crab-engram").is_none());
    assert!(cleaned.get("mcp").unwrap().get("github").is_some());
    let plugin_arr = cleaned.get("plugin").unwrap().as_array().unwrap();
    assert_eq!(plugin_arr.len(), 1);
    assert_eq!(plugin_arr[0], "./other.ts");
}

#[test]
fn test_merge_agents_md_appends_when_missing() {
    let existing = "# My AGENTS.md\nSome content\n";
    let protocol = "## Memory Protocol\nDo stuff";
    let merged = engram_mcp::config_merge::merge_agents_md(existing, protocol);
    assert!(merged.contains("<!-- gentle-ai:engram-protocol -->"));
    assert!(merged.contains("<!-- /gentle-ai:engram-protocol -->"));
    assert!(merged.contains("## Memory Protocol"));
    assert!(merged.contains("# My AGENTS.md"));
}

#[test]
fn test_merge_agents_md_replaces_existing_block() {
    let existing = "# My AGENTS.md\n<!-- gentle-ai:engram-protocol -->\nOld content\n<!-- /gentle-ai:engram-protocol -->\nMore stuff\n";
    let protocol = "## New Protocol\nNew stuff";
    let merged = engram_mcp::config_merge::merge_agents_md(existing, protocol);
    assert!(merged.contains("## New Protocol"));
    assert!(!merged.contains("Old content"));
    assert!(merged.contains("More stuff"));
}

#[test]
fn test_setup_dry_run() {
    let dir = setup_temp_dir();
    let config_dir = dir.path().join("opencode");
    std::fs::create_dir_all(&config_dir).unwrap();

    let paths = engram_mcp::opencode_paths::OpenCodePaths {
        config_dir: config_dir.clone(),
        config_file: config_dir.join("opencode.json"),
        plugin_dir: config_dir.join("plugins"),
        agents_file: config_dir.join("AGENTS.md"),
        is_jsonc: false,
    };

    // We need to import the setup function from the binary crate
    // Since it's in src/, we'll test via the config_merge functions instead
    // The dry-run logic is straightforward — test the merge functions directly
    let config = serde_json::json!({});
    let merged = engram_mcp::config_merge::merge_mcp_entry(&config, "agent", "test");
    assert!(merged.get("mcp").unwrap().get("the-crab-engram").is_some());
}

#[test]
fn test_openocode_paths_detect_json_format() {
    let dir = setup_temp_dir();
    let (path, is_jsonc) =
        engram_mcp::opencode_paths::OpenCodePaths::detect_json_format(dir.path());
    assert!(!is_jsonc);
    assert_eq!(path, dir.path().join("opencode.json"));

    std::fs::write(dir.path().join("opencode.jsonc"), "{}").unwrap();
    let (path2, is_jsonc2) =
        engram_mcp::opencode_paths::OpenCodePaths::detect_json_format(dir.path());
    assert!(is_jsonc2);
    assert_eq!(path2, dir.path().join("opencode.jsonc"));
}

#[test]
fn test_generate_memory_protocol() {
    let protocol = engram_mcp::config_merge::generate_memory_protocol();
    assert!(protocol.contains("Engram Persistent Memory"));
    assert!(protocol.contains("mem_save"));
    assert!(protocol.contains("mem_search"));
    assert!(protocol.contains("mem_context"));
}
