//! Agent adapters for MCP installation.
//!
//! Each adapter knows how to detect an AI agent on the system and
//! register/unregister the Engram MCP server in its config file.
//!
//! Supported agents:
//! - Claude Code (.mcp.json / settings.json)
//! - Cursor (mcp.json)
//! - Windsurf (mcp_config.json)
//! - OpenCode (opencode.json/jsonc) — uses existing opencode_setup
//! - Hermes (~/.hermes/config.yaml)

pub mod adapter;
pub mod claude_code;
pub mod cursor;
pub mod hermes;
pub mod opencode;
pub mod windsurf;

pub use adapter::{
    ActionKind, AgentAdapter, SetupAction, SetupResult, all_adapters, detect_agent,
    detect_installed,
};
