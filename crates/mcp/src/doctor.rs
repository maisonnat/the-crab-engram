use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "PASS"),
            CheckStatus::Fail => write!(f, "FAIL"),
            CheckStatus::Warn => write!(f, "WARN"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub fix_command: Option<String>,
}

#[derive(Clone, Copy)]
pub enum DoctorCheck {
    BinaryInPath,
    OpencodeInstalled,
    ConfigExists,
    McpEntryValid,
    PluginExists,
    ServerRunning,
    DatabaseOk,
    McpToolsProbe,
}

impl DoctorCheck {
    pub fn name(&self) -> &'static str {
        match self {
            DoctorCheck::BinaryInPath => "Binary in PATH",
            DoctorCheck::OpencodeInstalled => "OpenCode installed",
            DoctorCheck::ConfigExists => "Config exists",
            DoctorCheck::McpEntryValid => "MCP entry valid",
            DoctorCheck::PluginExists => "Plugin file exists",
            DoctorCheck::ServerRunning => "Server running",
            DoctorCheck::DatabaseOk => "Database OK",
            DoctorCheck::McpToolsProbe => "MCP tools probe",
        }
    }

    pub fn all_checks() -> &'static [DoctorCheck] {
        &[
            DoctorCheck::BinaryInPath,
            DoctorCheck::OpencodeInstalled,
            DoctorCheck::ConfigExists,
            DoctorCheck::McpEntryValid,
            DoctorCheck::PluginExists,
            DoctorCheck::ServerRunning,
            DoctorCheck::DatabaseOk,
            DoctorCheck::McpToolsProbe,
        ]
    }
}

pub fn check_binary_in_path() -> CheckResult {
    match which::which("the-crab-engram") {
        Ok(path) => CheckResult {
            name: DoctorCheck::BinaryInPath.name().to_string(),
            status: CheckStatus::Pass,
            message: path.display().to_string(),
            fix_command: None,
        },
        Err(_) => CheckResult {
            name: DoctorCheck::BinaryInPath.name().to_string(),
            status: CheckStatus::Fail,
            message: "the-crab-engram not found in PATH".to_string(),
            fix_command: Some("Install the-crab-engram and add to PATH".to_string()),
        },
    }
}

pub fn check_opencode_installed() -> CheckResult {
    match which::which("opencode") {
        Ok(path) => CheckResult {
            name: DoctorCheck::OpencodeInstalled.name().to_string(),
            status: CheckStatus::Pass,
            message: path.display().to_string(),
            fix_command: None,
        },
        Err(_) => CheckResult {
            name: DoctorCheck::OpencodeInstalled.name().to_string(),
            status: CheckStatus::Fail,
            message: "opencode not found".to_string(),
            fix_command: Some("npm install -g @opencode-ai/opencode".to_string()),
        },
    }
}

pub fn check_server_running() -> CheckResult {
    let url = "http://localhost:7437/health";
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());
    let resp = client.get(url).send();
    match resp {
        Ok(r) if r.status().is_success() => CheckResult {
            name: DoctorCheck::ServerRunning.name().to_string(),
            status: CheckStatus::Pass,
            message: url.to_string(),
            fix_command: None,
        },
        _ => CheckResult {
            name: DoctorCheck::ServerRunning.name().to_string(),
            status: CheckStatus::Fail,
            message: format!("{url} unreachable"),
            fix_command: Some("the-crab-engram serve --port 7437".to_string()),
        },
    }
}

pub fn display_results(results: &[CheckResult]) {
    println!("{:<25} {:<8} MESSAGE", "CHECK", "STATUS");
    println!("{}", "-".repeat(70));
    for r in results {
        let symbol = match r.status {
            CheckStatus::Pass => "PASS",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Warn => "WARN",
        };
        println!("{:<25} {:<8} {}", r.name, symbol, r.message);
    }
}

pub fn all_passed(results: &[CheckResult]) -> bool {
    results.iter().all(|r| r.status != CheckStatus::Fail)
}
