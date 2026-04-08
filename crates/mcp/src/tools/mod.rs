use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::*;

use engram_core::{ObservationType, Scope};
use engram_learn::{
    AntiPatternDetector, BoundaryTracker, CapsuleBuilder, ConsolidationEngine, GraphEvolver,
    HeuristicSynthesizer, MemoryStream, SmartInjector,
};
use engram_store::{AddObservationParams, SearchOptions, UpdateObservationParams};

use crate::*;

/// All tool definitions, filtered by profile in list_tools.
pub fn all_tool_definitions() -> Vec<Tool> {
    vec![
        tool_save(),
        tool_search(),
        tool_context(),
        tool_session_summary(),
        tool_session_start(),
        tool_session_end(),
        tool_get_observation(),
        tool_suggest_topic_key(),
        tool_capture_passive(),
        tool_save_prompt(),
        tool_update(),
        tool_delete(),
        tool_stats(),
        tool_timeline(),
        tool_merge_projects(),
        tool_capture_git(),
        tool_capture_error(),
        tool_stream(),
        tool_relate(),
        tool_graph(),
        tool_pin(),
        tool_inject(),
        tool_synthesize(),
        tool_capsule_list(),
        tool_capsule_get(),
        tool_antipatterns(),
        tool_consolidate(),
        tool_knowledge_boundary(),
        tool_transfer(),
        tool_reviews(),
        tool_beliefs(),
        tool_sync(),
    ]
}

/// Dispatch a tool call to the right handler.
pub async fn dispatch_tool(
    server: &EngramServer,
    name: &str,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<CallToolResult, ErrorData> {
    let args: HashMap<String, serde_json::Value> = arguments
        .map(|m| m.into_iter().collect())
        .unwrap_or_default();

    match name {
        "mem_save" => tool_save_handler(server, args).await,
        "mem_search" => tool_search_handler(server, args).await,
        "mem_context" => tool_context_handler(server, args).await,
        "mem_session_summary" => tool_session_summary_handler(server, args).await,
        "mem_session_start" => tool_session_start_handler(server, args).await,
        "mem_session_end" => tool_session_end_handler(server, args).await,
        "mem_get_observation" => tool_get_handler(server, args).await,
        "mem_suggest_topic_key" => tool_suggest_handler(server, args).await,
        "mem_capture_passive" => tool_capture_handler(server, args).await,
        "mem_save_prompt" => tool_save_prompt_handler(server, args).await,
        "mem_update" => tool_update_handler(server, args).await,
        "mem_delete" => tool_delete_handler(server, args).await,
        "mem_stats" => tool_stats_handler(server, args).await,
        "mem_timeline" => tool_timeline_handler(server, args).await,
        "mem_merge_projects" => tool_merge_handler(server, args).await,
        "mem_capture_git" => tool_capture_git_handler(server, args).await,
        "mem_capture_error" => tool_capture_error_handler(server, args).await,
        "mem_stream" => tool_stream_handler(server, args).await,
        "mem_relate" => tool_relate_handler(server, args).await,
        "mem_graph" => tool_graph_handler(server, args).await,
        "mem_pin" => tool_pin_handler(server, args).await,
        "mem_inject" => tool_inject_handler(server, args).await,
        "mem_synthesize" => tool_synthesize_handler(server, args).await,
        "mem_capsule_list" => tool_capsule_list_handler(server, args).await,
        "mem_capsule_get" => tool_capsule_get_handler(server, args).await,
        "mem_antipatterns" => tool_antipatterns_handler(server, args).await,
        "mem_consolidate" => tool_consolidate_handler(server, args).await,
        "mem_knowledge_boundary" => tool_knowledge_boundary_handler(server, args).await,
        "mem_transfer" => tool_transfer_handler(server, args).await,
        "mem_reviews" => tool_reviews_handler(server, args).await,
        "mem_beliefs" => tool_beliefs_handler(server, args).await,
        "mem_sync" => tool_sync_handler(server, args).await,
        _ => Err(ErrorData::invalid_params(
            format!("unknown tool: {name}"),
            None,
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn text_result(text: String) -> CallToolResult {
    CallToolResult::success(vec![Content::text(text)])
}

fn error_result(msg: &str) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg.to_string())])
}

fn get_string(args: &HashMap<String, serde_json::Value>, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn get_i64(args: &HashMap<String, serde_json::Value>, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

fn get_bool(args: &HashMap<String, serde_json::Value>, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| v.as_bool())
}

fn get_usize(args: &HashMap<String, serde_json::Value>, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|v| v as usize)
}

fn json_schema(schema_json: serde_json::Value) -> Arc<JsonObject> {
    Arc::new(schema_json.as_object().unwrap().clone())
}

// ── Tool Definitions ──────────────────────────────────────────────

fn tool_save() -> Tool {
    Tool::new(
        "mem_save",
        "Save a memory observation. Use to record learnings, decisions, bugs, patterns, or any knowledge worth remembering.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Short title for the observation" },
                "content": { "type": "string", "description": "Full content/detail" },
                "type": { "type": "string", "enum": ["bugfix","decision","architecture","pattern","discovery","learning","config","convention","tool_use","file_change","command","file_read","search","manual"], "default": "manual" },
                "scope": { "type": "string", "enum": ["project","personal"], "default": "project" },
                "session_id": { "type": "string", "description": "Current session ID from mem_session_start" },
                "project": { "type": "string", "description": "Project name (uses default if omitted)" },
                "topic_key": { "type": "string", "description": "Optional topic key for upsert" },
                "attachments": { "type": "array", "description": "Optional attachments (CodeDiff, TerminalOutput, ErrorTrace, GitCommit)", "items": { "type": "object" } }
            },
            "required": ["title", "content", "session_id"]
        })),
    )
}

fn tool_search() -> Tool {
    Tool::new(
        "mem_search",
        "Search memories by keyword. Returns ranked results with relevance scores.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "project": { "type": "string" },
                "type": { "type": "string" },
                "limit": { "type": "integer", "default": 10 }
            },
            "required": ["query"]
        })),
    )
}

fn tool_context() -> Tool {
    Tool::new(
        "mem_context",
        "Get session context — recent observations from the last session.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" },
                "limit": { "type": "integer", "default": 10 }
            }
        })),
    )
}

fn tool_session_summary() -> Tool {
    Tool::new(
        "mem_session_summary",
        "Get a session summary by ID.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": { "type": "string" }
            },
            "required": ["session_id"]
        })),
    )
}

fn tool_session_start() -> Tool {
    Tool::new(
        "mem_session_start",
        "Start a new coding session. Returns session_id to use with mem_save.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" }
            }
        })),
    )
}

fn tool_session_end() -> Tool {
    Tool::new(
        "mem_session_end",
        "End a session with optional summary.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": { "type": "string" },
                "summary": { "type": "string" }
            },
            "required": ["session_id"]
        })),
    )
}

fn tool_get_observation() -> Tool {
    Tool::new(
        "mem_get_observation",
        "Get full observation by ID. Increments access count.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "observation_id": { "type": "integer" }
            },
            "required": ["observation_id"]
        })),
    )
}

fn tool_suggest_topic_key() -> Tool {
    Tool::new(
        "mem_suggest_topic_key",
        "Suggest a topic key for an observation type and title.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "type": { "type": "string" },
                "title": { "type": "string" }
            },
            "required": ["type", "title"]
        })),
    )
}

fn tool_capture_passive() -> Tool {
    Tool::new(
        "mem_capture_passive",
        "Extract learnings from agent output automatically. Detects patterns, bugs, decisions, and stores them as observations.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "output": { "type": "string", "description": "Agent output text to analyze" },
                "session_id": { "type": "string" },
                "project": { "type": "string" }
            },
            "required": ["output", "session_id"]
        })),
    )
}

fn tool_save_prompt() -> Tool {
    Tool::new(
        "mem_save_prompt",
        "Save a user prompt for future context.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": { "type": "string" },
                "content": { "type": "string" },
                "project": { "type": "string" }
            },
            "required": ["session_id", "content"]
        })),
    )
}

fn tool_update() -> Tool {
    Tool::new(
        "mem_update",
        "Update an existing observation (partial update).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "observation_id": { "type": "integer" },
                "title": { "type": "string" },
                "content": { "type": "string" },
                "pinned": { "type": "boolean" },
                "topic_key": { "type": "string" }
            },
            "required": ["observation_id"]
        })),
    )
}

fn tool_delete() -> Tool {
    Tool::new(
        "mem_delete",
        "Delete an observation (admin only). Soft by default, hard if specified.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "observation_id": { "type": "integer" },
                "hard": { "type": "boolean", "default": false }
            },
            "required": ["observation_id"]
        })),
    )
}

fn tool_stats() -> Tool {
    Tool::new(
        "mem_stats",
        "Get project statistics (admin only).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" }
            }
        })),
    )
}

fn tool_timeline() -> Tool {
    Tool::new(
        "mem_timeline",
        "Get observations around a specific observation in time (admin only).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "observation_id": { "type": "integer" },
                "window": { "type": "integer", "default": 5 }
            },
            "required": ["observation_id"]
        })),
    )
}

fn tool_merge_projects() -> Tool {
    Tool::new(
        "mem_merge_projects",
        "Merge all observations from source project into target (admin only).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "source_project": { "type": "string" },
                "target_project": { "type": "string" }
            },
            "required": ["source_project", "target_project"]
        })),
    )
}

fn tool_capture_git() -> Tool {
    Tool::new(
        "mem_capture_git",
        "Capture a git commit as an observation with CodeDiff and GitCommit attachments.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "commit_hash": { "type": "string" },
                "commit_message": { "type": "string" },
                "files_changed": { "type": "array", "items": { "type": "string" } },
                "diff_summary": { "type": "string" },
                "session_id": { "type": "string" },
                "project": { "type": "string" }
            },
            "required": ["commit_hash", "commit_message", "session_id"]
        })),
    )
}

fn tool_capture_error() -> Tool {
    Tool::new(
        "mem_capture_error",
        "Capture a compilation/test error as an observation with ErrorTrace attachment.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "error_type": { "type": "string", "description": "e.g. 'compile_error', 'test_failure', 'panic'" },
                "error_message": { "type": "string" },
                "stack_trace": { "type": "string" },
                "file_line": { "type": "string", "description": "e.g. 'src/main.rs:42'" },
                "session_id": { "type": "string" },
                "project": { "type": "string" }
            },
            "required": ["error_type", "error_message", "session_id"]
        })),
    )
}

fn tool_stream() -> Tool {
    Tool::new(
        "mem_stream",
        "Detect memory events for current work (file context, anti-patterns, deja-vu, pending reviews).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" },
                "file_path": { "type": "string", "description": "Currently edited file" },
                "task_description": { "type": "string", "description": "Current task for deja-vu detection" }
            }
        })),
    )
}

fn tool_relate() -> Tool {
    Tool::new(
        "mem_relate",
        "Create a relationship between two observations in the knowledge graph.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "source_id": { "type": "integer", "description": "Source observation ID" },
                "target_id": { "type": "integer", "description": "Target observation ID" },
                "relation": { "type": "string", "enum": ["caused_by", "related_to", "supersedes", "blocks", "part_of"], "description": "Relationship type" },
                "weight": { "type": "number", "default": 1.0, "description": "Edge weight 0.0-1.0" }
            },
            "required": ["source_id", "target_id", "relation"]
        })),
    )
}

fn tool_graph() -> Tool {
    Tool::new(
        "mem_graph",
        "Get the knowledge graph around an observation (edges + related observations via BFS).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "observation_id": { "type": "integer", "description": "Center observation ID" },
                "max_depth": { "type": "integer", "default": 2, "description": "BFS depth limit" }
            },
            "required": ["observation_id"]
        })),
    )
}

fn tool_pin() -> Tool {
    Tool::new(
        "mem_pin",
        "Pin or unpin an observation. Pinned observations get maximum relevance score.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "observation_id": { "type": "integer", "description": "Observation to pin/unpin" },
                "pinned": { "type": "boolean", "description": "true to pin, false to unpin" }
            },
            "required": ["observation_id", "pinned"]
        })),
    )
}

fn tool_inject() -> Tool {
    Tool::new(
        "mem_inject",
        "Smart context injection — get relevant memories, warnings, and boundaries for a task.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "task_description": { "type": "string", "description": "What you're working on" },
                "project": { "type": "string" },
                "current_files": { "type": "array", "items": { "type": "string" }, "description": "Files being edited" }
            },
            "required": ["task_description"]
        })),
    )
}

fn tool_synthesize() -> Tool {
    Tool::new(
        "mem_synthesize",
        "Generate or update a knowledge capsule for a topic by synthesizing observations.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string", "description": "Topic for the capsule" },
                "project": { "type": "string" }
            },
            "required": ["topic"]
        })),
    )
}

fn tool_capsule_list() -> Tool {
    Tool::new(
        "mem_capsule_list",
        "List knowledge capsules for a project.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" }
            }
        })),
    )
}

fn tool_capsule_get() -> Tool {
    Tool::new(
        "mem_capsule_get",
        "Get a full knowledge capsule by topic.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "topic": { "type": "string" },
                "project": { "type": "string" }
            },
            "required": ["topic"]
        })),
    )
}

fn tool_antipatterns() -> Tool {
    Tool::new(
        "mem_antipatterns",
        "Detect anti-patterns in the project (recurring bugs, unverified decisions, hotspot files).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" },
                "severity": { "type": "string", "enum": ["warning", "critical"] }
            }
        })),
    )
}

fn tool_consolidate() -> Tool {
    Tool::new(
        "mem_consolidate",
        "Run memory consolidation: merge duplicates, mark obsolete, find conflicts, extract patterns. Returns metrics.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" },
                "dry_run": { "type": "boolean", "default": false }
            }
        })),
    )
}

fn tool_knowledge_boundary() -> Tool {
    Tool::new(
        "mem_knowledge_boundary",
        "View or update knowledge boundaries — what the system knows and doesn't know.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "domain": { "type": "string", "description": "Domain to update" },
                "confidence_level": { "type": "string", "enum": ["expert", "proficient", "familiar", "aware", "unknown"] },
                "evidence": { "type": "string" }
            }
        })),
    )
}

fn tool_transfer() -> Tool {
    Tool::new(
        "mem_transfer",
        "Suggest or view cross-project knowledge transfers.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string", "description": "Target project" },
                "context": { "type": "string", "description": "Current context for relevance" }
            }
        })),
    )
}

fn tool_reviews() -> Tool {
    Tool::new(
        "mem_reviews",
        "Get pending spaced repetition reviews.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "project": { "type": "string" },
                "limit": { "type": "integer", "default": 10 }
            }
        })),
    )
}

fn tool_beliefs() -> Tool {
    Tool::new(
        "mem_beliefs",
        "Query beliefs about a subject. Returns belief state, confidence, and evidence.",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "subject": { "type": "string", "description": "Subject to query beliefs about (e.g. 'JWT', 'auth')" }
            },
            "required": ["subject"]
        })),
    )
}

fn tool_sync() -> Tool {
    Tool::new(
        "mem_sync",
        "Sync operations: status (get sync state), export (write chunks to dir), import (read chunks from dir).",
        json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["status", "export", "import"], "description": "Sync action" },
                "dir": { "type": "string", "description": "Directory for export/import (default: ./engram-chunks)" }
            },
            "required": ["action"]
        })),
    )
}

// ── Tool Handlers ─────────────────────────────────────────────────

async fn tool_save_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let title = match get_string(&args, "title") {
        Some(t) => t,
        None => return Ok(error_result("missing 'title'")),
    };
    let content = match get_string(&args, "content") {
        Some(c) => c,
        None => return Ok(error_result("missing 'content'")),
    };
    let session_id = match get_string(&args, "session_id") {
        Some(s) => s,
        None => return Ok(error_result("missing 'session_id'")),
    };
    let type_str = get_string(&args, "type").unwrap_or_else(|| "manual".into());
    let scope_str = get_string(&args, "scope").unwrap_or_else(|| "project".into());
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());
    let topic_key = get_string(&args, "topic_key");

    let obs_type: ObservationType = match type_str.parse() {
        Ok(t) => t,
        Err(_) => return Ok(error_result(&format!("invalid type: {type_str}"))),
    };
    let scope: Scope = if scope_str == "personal" {
        Scope::Personal
    } else {
        Scope::Project
    };

    let params = AddObservationParams {
        r#type: obs_type,
        scope,
        title: title.clone(),
        content,
        session_id,
        project,
        topic_key,
        ..Default::default()
    };

    match server.store.insert_observation(&params) {
        Ok(id) => {
            // Handle attachments if provided
            let mut attachment_count = 0;
            if let Some(attachments_arr) = args.get("attachments").and_then(|v| v.as_array()) {
                for att_json in attachments_arr {
                    if let Ok(att) =
                        serde_json::from_value::<engram_core::Attachment>(att_json.clone())
                    {
                        match server.store.store_attachment(id, &att) {
                            Ok(_) => attachment_count += 1,
                            Err(e) => {
                                tracing::warn!("Failed to store attachment: {e}");
                            }
                        }
                    }
                }
            }

            let att_msg = if attachment_count > 0 {
                format!(" + {attachment_count} attachment(s)")
            } else {
                String::new()
            };

            // Belief extraction from content
            extract_and_upsert_beliefs(server, id, &params.content, &params.project);

            // Anti-pattern check for bugfix saves
            let warning = if obs_type == engram_core::ObservationType::Bugfix {
                let bugfixes = server
                    .store
                    .search(&SearchOptions {
                        query: String::new(),
                        project: Some(params.project.clone()),
                        r#type: Some(engram_core::ObservationType::Bugfix),
                        limit: Some(200),
                        ..Default::default()
                    })
                    .unwrap_or_default();

                // Check if this file/path appears in 3+ bugs
                let mut file_hits: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for bug in &bugfixes {
                    for word in format!("{} {}", bug.title, bug.content).split_whitespace() {
                        if word.contains(".rs") || word.contains(".ts") || word.contains(".go") {
                            *file_hits.entry(word.to_lowercase()).or_insert(0) += 1;
                        }
                    }
                }
                file_hits
                    .into_iter()
                    .filter(|(_, c)| *c >= 3)
                    .map(|(f, c)| {
                        format!(
                            "\n⚠️ Anti-pattern: `{f}` has {c} bugs — consider root cause analysis"
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("")
            } else {
                String::new()
            };

            Ok(text_result(format!(
                "✅ Saved observation #{id}: \"{title}\" (type={type_str}, scope={scope_str}){att_msg}{warning}"
            )))
        }
        Err(e) => Ok(error_result(&format!("failed to save: {e}"))),
    }
}

async fn tool_search_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let query = match get_string(&args, "query") {
        Some(q) => q,
        None => return Ok(error_result("missing 'query'")),
    };

    let r#type = get_string(&args, "type").and_then(|s| s.parse().ok());
    let project = get_string(&args, "project");
    let limit = get_usize(&args, "limit").unwrap_or(10);

    let opts = SearchOptions {
        query,
        project,
        r#type,
        limit: Some(limit),
        ..Default::default()
    };

    match server.store.search(&opts) {
        Ok(results) => {
            if results.is_empty() {
                return Ok(text_result("No results found.".into()));
            }
            let mut text = format!("Found {} result(s):\n\n", results.len());
            for (i, obs) in results.iter().enumerate() {
                text.push_str(&format!(
                    "{}. **#{}** [{}] {} (accessed {}x)\n   {}\n\n",
                    i + 1,
                    obs.id,
                    obs.r#type,
                    obs.title,
                    obs.access_count,
                    obs.content.chars().take(200).collect::<String>()
                ));
            }

            // Suggest capsule if >5 results share a topic_key
            let mut topic_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for obs in &results {
                if let Some(ref topic) = obs.topic_key {
                    *topic_counts.entry(topic.clone()).or_insert(0) += 1;
                }
            }
            for (topic, count) in &topic_counts {
                if *count >= 5 {
                    text.push_str(&format!(
                        "💡 **Tip:** {count} results share topic `{topic}`. Try `mem_synthesize topic:{topic}` to create a capsule.\n"
                    ));
                }
            }

            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("search failed: {e}"))),
    }
}

async fn tool_context_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());
    let limit = get_usize(&args, "limit").unwrap_or(10);

    match server.store.get_session_context(&project, limit) {
        Ok(ctx) => {
            let mut text = format!("## Session Context for \"{project}\"\n\n");
            text.push_str(&format!(
                "**Session:** {} (started {})\n\n",
                &ctx.session.id[..8],
                ctx.session.started_at.format("%Y-%m-%d %H:%M")
            ));

            // Anti-pattern warnings
            let bugfixes = server
                .store
                .search(&engram_store::SearchOptions {
                    query: String::new(),
                    project: Some(project.clone()),
                    r#type: Some(engram_core::ObservationType::Bugfix),
                    limit: Some(200),
                    ..Default::default()
                })
                .unwrap_or_default();

            let mut file_bug_count: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for bug in &bugfixes {
                for word in format!("{} {}", bug.title, bug.content).split_whitespace() {
                    if word.contains(".rs") || word.contains(".ts") || word.contains(".go") {
                        *file_bug_count.entry(word.to_lowercase()).or_insert(0) += 1;
                    }
                }
            }
            let warnings: Vec<_> = file_bug_count.iter().filter(|(_, c)| **c >= 3).collect();
            if !warnings.is_empty() {
                text.push_str("### ⚠️ Warnings\n\n");
                for (file, count) in &warnings {
                    text.push_str(&format!(
                        "- `{file}` has {count} recurring bugs — consider root cause analysis\n"
                    ));
                }
                text.push('\n');
            }

            if !ctx.observations.is_empty() {
                text.push_str("### Recent Observations\n\n");
                for obs in &ctx.observations {
                    text.push_str(&format!(
                        "- **#{}** [{}] {} — {}\n",
                        obs.id,
                        obs.r#type,
                        obs.title,
                        obs.content.chars().take(100).collect::<String>()
                    ));
                }
            } else {
                text.push_str("_No observations in this session yet._\n");
            }

            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("failed to get context: {e}"))),
    }
}

async fn tool_session_summary_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let session_id = match get_string(&args, "session_id") {
        Some(s) => s,
        None => return Ok(error_result("missing 'session_id'")),
    };

    match server.store.get_session(&session_id) {
        Ok(Some(session)) => {
            let mut text = format!(
                "## Session {}\n\n**Project:** {}\n**Started:** {}\n",
                &session.id[..8],
                session.project,
                session.started_at.format("%Y-%m-%d %H:%M")
            );
            if let Some(ended) = session.ended_at {
                text.push_str(&format!("**Ended:** {}\n", ended.format("%Y-%m-%d %H:%M")));
            }
            if let Some(summary) = &session.summary {
                text.push_str(&format!("\n**Summary:** {summary}\n"));
            }
            Ok(text_result(text))
        }
        Ok(None) => Ok(error_result("session not found")),
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_session_start_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    match server.store.create_session(&project) {
        Ok(id) => Ok(text_result(format!(
            "🟢 Session started: {id}\n\nUse this session_id with mem_save, mem_session_end, etc."
        ))),
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_session_end_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let session_id = match get_string(&args, "session_id") {
        Some(s) => s,
        None => return Ok(error_result("missing 'session_id'")),
    };
    let summary = get_string(&args, "summary");

    match server.store.end_session(&session_id, summary.as_deref()) {
        Ok(()) => Ok(text_result(format!(
            "🔴 Session {} ended.",
            &session_id[..8]
        ))),
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_get_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let id = match get_i64(&args, "observation_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'observation_id'")),
    };

    match server.store.get_observation(id) {
        Ok(Some(obs)) => {
            let prov_source = format!("{:?}", obs.provenance_source);
            let text = format!(
                "## Observation #{id}\n\n\
                 **Type:** {}\n**Scope:** {}\n**Title:** {}\n\
                 **Topic:** {}\n**Created:** {}\n\
                 **Access count:** {}\n**Pinned:** {}\n\
                 **Provenance:** {prov_source} ({:.0}%)\n\n\
                 ---\n\n{}",
                obs.r#type,
                obs.scope,
                obs.title,
                obs.topic_key.as_deref().unwrap_or("—"),
                obs.created_at.format("%Y-%m-%d %H:%M"),
                obs.access_count,
                obs.pinned,
                obs.provenance_confidence * 100.0,
                obs.content,
            );
            Ok(text_result(text))
        }
        Ok(None) => Ok(error_result(&format!("observation #{id} not found"))),
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_suggest_handler(
    _server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let type_str = match get_string(&args, "type") {
        Some(t) => t,
        None => return Ok(error_result("missing 'type'")),
    };
    let title = match get_string(&args, "title") {
        Some(t) => t,
        None => return Ok(error_result("missing 'title'")),
    };

    let obs_type: ObservationType = match type_str.parse() {
        Ok(t) => t,
        Err(_) => return Ok(error_result(&format!("invalid type: {type_str}"))),
    };

    let key = engram_core::suggest_topic_key(obs_type, &title);
    Ok(text_result(format!("Suggested topic key: `{key}`")))
}

async fn tool_capture_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let output = match get_string(&args, "output") {
        Some(o) => o,
        None => return Ok(error_result("missing 'output'")),
    };
    let session_id = match get_string(&args, "session_id") {
        Some(s) => s,
        None => return Ok(error_result("missing 'session_id'")),
    };
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    let lower = output.to_lowercase();
    let mut captured = Vec::new();

    if lower.contains("test passed") || lower.contains("✅") {
        captured.push(("decision", "Tests passing", "test_verified"));
    }
    if lower.contains("error") || lower.contains("failed") || lower.contains("panic") {
        captured.push(("bugfix", "Error detected in output", "llm_reasoning"));
    }
    if lower.contains("changed") || lower.contains("modified") || lower.contains("refactor") {
        captured.push(("file_change", "Code change detected", "code_analysis"));
    }

    if captured.is_empty() {
        return Ok(text_result("No learnings detected in output.".into()));
    }

    let mut results = String::from("📥 Captured learnings:\n\n");
    for (type_str, title, prov) in &captured {
        let obs_type: ObservationType = type_str.parse().unwrap_or(ObservationType::Manual);
        let snippet = output.chars().take(300).collect::<String>();

        let params = AddObservationParams {
            r#type: obs_type,
            scope: Scope::Project,
            title: title.to_string(),
            content: snippet,
            session_id: session_id.clone(),
            project: project.clone(),
            provenance_source: Some(prov.to_string()),
            ..Default::default()
        };

        match server.store.insert_observation(&params) {
            Ok(id) => {
                // Infer salience and update observation
                let salience = engram_learn::infer_salience(
                    &format!("{title} {}", output.chars().take(200).collect::<String>()),
                    None,
                );
                let _ = server.store.update_observation(
                    id,
                    &engram_store::UpdateObservationParams {
                        ..Default::default()
                    },
                );
                results.push_str(&format!(
                    "- #{id} [{type_str}] {title} (salience: {:.0}%)\n",
                    (salience.emotional_valence.abs() + salience.surprise_factor) * 50.0
                ));
            }
            Err(e) => {
                results.push_str(&format!("- FAILED [{type_str}] {title}: {e}\n"));
            }
        }
    }

    Ok(text_result(results))
}

async fn tool_save_prompt_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let session_id = match get_string(&args, "session_id") {
        Some(s) => s,
        None => return Ok(error_result("missing 'session_id'")),
    };
    let content = match get_string(&args, "content") {
        Some(c) => c,
        None => return Ok(error_result("missing 'content'")),
    };
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    let params = engram_store::AddPromptParams {
        session_id,
        project,
        content,
    };

    match server.store.save_prompt(&params) {
        Ok(()) => Ok(text_result("✅ Prompt saved.".into())),
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_update_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let id = match get_i64(&args, "observation_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'observation_id'")),
    };

    let params = UpdateObservationParams {
        title: get_string(&args, "title"),
        content: get_string(&args, "content"),
        pinned: get_bool(&args, "pinned"),
        topic_key: get_string(&args, "topic_key"),
        ..Default::default()
    };

    match server.store.update_observation(id, &params) {
        Ok(()) => Ok(text_result(format!("✅ Observation #{id} updated."))),
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_delete_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let id = match get_i64(&args, "observation_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'observation_id'")),
    };

    // Permission check — delete requires Admin
    if server.config.profile == ToolProfile::Agent {
        return Ok(error_result(
            "delete requires Admin profile. Use --profile admin",
        ));
    }

    let hard = get_bool(&args, "hard").unwrap_or(false);

    match server.store.delete_observation(id, hard) {
        Ok(()) => {
            let mode = if hard { "hard-deleted" } else { "soft-deleted" };
            Ok(text_result(format!("🗑️ Observation #{id} {mode}.")))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_stats_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    match server.store.get_stats(&project) {
        Ok(stats) => {
            let mut text = format!("## Stats for \"{project}\"\n\n");
            text.push_str(&format!(
                "- **Total observations:** {}\n",
                stats.total_observations
            ));
            text.push_str(&format!("- **Total sessions:** {}\n", stats.total_sessions));
            text.push_str(&format!("- **Total edges:** {}\n\n", stats.total_edges));

            text.push_str("**By type:**\n");
            for (t, count) in &stats.by_type {
                text.push_str(&format!("- {t}: {count}\n"));
            }

            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_timeline_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let id = match get_i64(&args, "observation_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'observation_id'")),
    };
    let window = get_usize(&args, "window").unwrap_or(5);

    match server.store.get_timeline(id, window) {
        Ok(entries) => {
            let mut text = format!("## Timeline around observation #{id}\n\n");
            for entry in &entries {
                let marker = match entry.position {
                    engram_store::TimelinePosition::Before => "  ",
                    engram_store::TimelinePosition::Center => "→ ",
                    engram_store::TimelinePosition::After => "  ",
                };
                text.push_str(&format!(
                    "{}{} [{}] {}\n",
                    marker,
                    entry.observation.created_at.format("%H:%M"),
                    entry.observation.r#type,
                    entry.observation.title,
                ));
            }
            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_merge_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let source = match get_string(&args, "source_project") {
        Some(s) => s,
        None => return Ok(error_result("missing 'source_project'")),
    };
    let target = match get_string(&args, "target_project") {
        Some(t) => t,
        None => return Ok(error_result("missing 'target_project'")),
    };

    match server.store.export(Some(&source)) {
        Ok(mut data) => {
            for obs in &mut data.observations {
                obs.project = target.clone();
            }
            for session in &mut data.sessions {
                session.project = target.clone();
            }
            for prompt in &mut data.prompts {
                prompt.project = target.clone();
            }

            match server.store.import(&data) {
                Ok(result) => Ok(text_result(format!(
                    "✅ Merged \"{source}\" → \"{target}\"\n\
                     - {} observations imported\n\
                     - {} sessions imported\n\
                     - {} duplicates skipped",
                    result.observations_imported,
                    result.sessions_imported,
                    result.duplicates_skipped,
                ))),
                Err(e) => Ok(error_result(&format!("import failed: {e}"))),
            }
        }
        Err(e) => Ok(error_result(&format!("export failed: {e}"))),
    }
}

async fn tool_capture_git_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let commit_hash = match get_string(&args, "commit_hash") {
        Some(h) => h,
        None => return Ok(error_result("missing 'commit_hash'")),
    };
    let commit_message = match get_string(&args, "commit_message") {
        Some(m) => m,
        None => return Ok(error_result("missing 'commit_message'")),
    };
    let session_id = match get_string(&args, "session_id") {
        Some(s) => s,
        None => return Ok(error_result("missing 'session_id'")),
    };
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());
    let files_changed: Vec<String> = args
        .get("files_changed")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let diff_summary = get_string(&args, "diff_summary").unwrap_or_default();

    // Create observation
    let params = AddObservationParams {
        r#type: engram_core::ObservationType::FileChange,
        scope: engram_core::Scope::Project,
        title: format!(
            "{}: {}",
            &commit_hash[..8.min(commit_hash.len())],
            truncate_str(&commit_message, 80)
        ),
        content: format!(
            "Commit: {commit_hash}\nMessage: {commit_message}\nFiles: {}",
            files_changed.join(", ")
        ),
        session_id: session_id.clone(),
        project: project.clone(),
        topic_key: Some(format!("git/{}", &commit_hash[..8.min(commit_hash.len())])),
        provenance_source: Some("code_analysis".into()),
        ..Default::default()
    };

    match server.store.insert_observation(&params) {
        Ok(obs_id) => {
            // Store GitCommit attachment
            let att = engram_core::Attachment::GitCommit {
                hash: commit_hash.clone(),
                message: commit_message.clone(),
                files_changed: files_changed.clone(),
                diff_summary: diff_summary.clone(),
            };
            let att_msg = match server.store.store_attachment(obs_id, &att) {
                Ok(att_id) => format!("\n📎 Attachment #{att_id} (GitCommit)"),
                Err(e) => format!("\n⚠️ Attachment failed: {e}"),
            };

            // Store CodeDiff attachment if diff_summary is present
            if !diff_summary.is_empty()
                && let Some(first_file) = files_changed.first()
            {
                let diff_att = engram_core::Attachment::CodeDiff {
                    file_path: first_file.clone(),
                    before_hash: String::new(),
                    after_hash: commit_hash.clone(),
                    diff: diff_summary.clone(),
                };
                let _ = server.store.store_attachment(obs_id, &diff_att);
            }

            Ok(text_result(format!(
                "✅ Captured git commit #{obs_id}: {}\n  Files: {}{att_msg}",
                truncate_str(&commit_message, 60),
                files_changed.join(", "),
            )))
        }
        Err(e) => Ok(error_result(&format!("failed to capture: {e}"))),
    }
}

async fn tool_capture_error_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let error_type = match get_string(&args, "error_type") {
        Some(t) => t,
        None => return Ok(error_result("missing 'error_type'")),
    };
    let error_message = match get_string(&args, "error_message") {
        Some(m) => m,
        None => return Ok(error_result("missing 'error_message'")),
    };
    let session_id = match get_string(&args, "session_id") {
        Some(s) => s,
        None => return Ok(error_result("missing 'session_id'")),
    };
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());
    let stack_trace = get_string(&args, "stack_trace").unwrap_or_default();
    let file_line = get_string(&args, "file_line").unwrap_or_default();

    // Create observation
    let params = AddObservationParams {
        r#type: engram_core::ObservationType::Bugfix,
        scope: engram_core::Scope::Project,
        title: format!("{error_type}: {}", truncate_str(&error_message, 80)),
        content: format!(
            "Error: {error_message}\nLocation: {file_line}\nStack: {}",
            truncate_str(&stack_trace, 500)
        ),
        session_id: session_id.clone(),
        project: project.clone(),
        topic_key: Some(format!("bug/{error_type}")),
        provenance_source: Some("code_analysis".into()),
        ..Default::default()
    };

    match server.store.insert_observation(&params) {
        Ok(obs_id) => {
            // Store ErrorTrace attachment
            let (file, line) = parse_file_line(&file_line);
            let att = engram_core::Attachment::ErrorTrace {
                error_type: error_type.clone(),
                message: error_message.clone(),
                stack_trace: stack_trace.clone(),
                file_line: file.map(|f| (f, line)),
            };
            let att_msg = match server.store.store_attachment(obs_id, &att) {
                Ok(att_id) => format!("\n📎 Attachment #{att_id} (ErrorTrace)"),
                Err(e) => format!("\n⚠️ Attachment failed: {e}"),
            };

            Ok(text_result(format!(
                "✅ Captured error #{obs_id}: {error_type}: {}\n  Location: {file_line}{att_msg}",
                truncate_str(&error_message, 60),
            )))
        }
        Err(e) => Ok(error_result(&format!("failed to capture: {e}"))),
    }
}

async fn tool_stream_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());
    let file_path = get_string(&args, "file_path");
    let task_description = get_string(&args, "task_description");
    let mode = get_string(&args, "mode").unwrap_or_else(|| "file_context".into());

    let stream = MemoryStream::new(server.store.clone(), None);

    let events = match mode.as_str() {
        "file_context" => {
            if let Some(fp) = &file_path {
                stream.detect_file_context(&project, fp).unwrap_or_default()
            } else {
                vec![]
            }
        }
        "deja_vu" => {
            if let Some(task) = &task_description {
                stream.detect_deja_vu(&project, task).unwrap_or_default()
            } else {
                vec![]
            }
        }
        "anti_patterns" => {
            let content = get_string(&args, "content").unwrap_or_default();
            stream
                .detect_anti_pattern_warnings(&project, &content)
                .unwrap_or_default()
        }
        "pending_reviews" => stream.detect_pending_reviews(&project).unwrap_or_default(),
        "entities" => {
            let text = get_string(&args, "text").unwrap_or_default();
            stream.detect_entities(&text).unwrap_or_default()
        }
        _ => {
            // Default: file_context if file_path given, else anti_patterns
            if let Some(fp) = &file_path {
                stream.detect_file_context(&project, fp).unwrap_or_default()
            } else {
                vec![]
            }
        }
    };

    if events.is_empty() {
        Ok(text_result("No events detected.".into()))
    } else {
        let mut text = format!("🌊 Memory events ({})\n\n", events.len());
        for event in &events {
            text.push_str(&format!("- {}\n", event));
        }
        Ok(text_result(text))
    }
}

async fn tool_relate_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let source_id = match get_i64(&args, "source_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'source_id'")),
    };
    let target_id = match get_i64(&args, "target_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'target_id'")),
    };
    let relation_str = match get_string(&args, "relation") {
        Some(r) => r,
        None => return Ok(error_result("missing 'relation'")),
    };
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0);

    let relation = match relation_str.as_str() {
        "caused_by" => engram_core::RelationType::CausedBy,
        "related_to" => engram_core::RelationType::RelatedTo,
        "supersedes" => engram_core::RelationType::Supersedes,
        "blocks" => engram_core::RelationType::Blocks,
        "part_of" => engram_core::RelationType::PartOf,
        _ => return Ok(error_result(&format!("invalid relation: {relation_str}"))),
    };

    let params = engram_store::AddEdgeParams {
        source_id,
        target_id,
        relation,
        weight,
        auto_detected: false,
    };

    match server.store.add_edge(&params) {
        Ok(edge_id) => Ok(text_result(format!(
            "🔗 Edge #{edge_id} created: #{source_id} --[{relation_str}]--> #{target_id}"
        ))),
        Err(e) => Ok(error_result(&format!("failed to create edge: {e}"))),
    }
}

async fn tool_graph_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let observation_id = match get_i64(&args, "observation_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'observation_id'")),
    };
    let max_depth = get_usize(&args, "max_depth").unwrap_or(2);

    // Get direct edges
    let edges = match server.store.get_edges(observation_id) {
        Ok(e) => e,
        Err(e) => return Ok(error_result(&format!("failed to get edges: {e}"))),
    };

    // Get related observations via BFS
    let related = match server.store.get_related(observation_id, max_depth) {
        Ok(r) => r,
        Err(e) => return Ok(error_result(&format!("failed to get related: {e}"))),
    };

    let mut text = format!("## Graph around observation #{observation_id}\n\n");

    if !edges.is_empty() {
        text.push_str("### Direct Edges\n\n");
        for edge in &edges {
            text.push_str(&format!(
                "- #{} --[{:?}]--> #{} (weight: {:.2})\n",
                edge.source_id, edge.relation, edge.target_id, edge.weight
            ));
        }
    }

    if !related.is_empty() {
        text.push_str("\n### Related Observations (BFS)\n\n");
        for (obs, rel, depth) in &related {
            text.push_str(&format!(
                "- **#{}** [depth {depth}] [{:?}] {} — {}\n",
                obs.id,
                rel,
                obs.title,
                obs.content.chars().take(80).collect::<String>()
            ));
        }
    }

    if edges.is_empty() && related.is_empty() {
        text.push_str("_No connections found._\n");
    }

    Ok(text_result(text))
}

async fn tool_pin_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let id = match get_i64(&args, "observation_id") {
        Some(i) => i,
        None => return Ok(error_result("missing 'observation_id'")),
    };
    let pinned = match get_bool(&args, "pinned") {
        Some(p) => p,
        None => return Ok(error_result("missing 'pinned'")),
    };

    let params = engram_store::UpdateObservationParams {
        pinned: Some(pinned),
        ..Default::default()
    };

    match server.store.update_observation(id, &params) {
        Ok(()) => {
            let status = if pinned { "📌 Pinned" } else { "Unpinned" };
            Ok(text_result(format!("{status} observation #{id}")))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_inject_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let task = match get_string(&args, "task_description") {
        Some(t) => t,
        None => return Ok(error_result("missing 'task_description'")),
    };
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());
    let max_tokens = get_usize(&args, "max_tokens").unwrap_or(2000);

    let injector = SmartInjector::new(server.store.clone());
    match injector.build_context(&project, &task, max_tokens) {
        Ok(ctx) => Ok(text_result(ctx.to_markdown())),
        Err(e) => Ok(error_result(&format!("context injection failed: {e}"))),
    }
}

async fn tool_synthesize_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let topic = match get_string(&args, "topic") {
        Some(t) => t,
        None => return Ok(error_result("missing 'topic'")),
    };
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    let builder = CapsuleBuilder::new(server.store.clone(), Box::new(HeuristicSynthesizer));
    match builder.build_capsule(&project, &topic) {
        Ok(capsule) => match server.store.upsert_capsule(&capsule) {
            Ok(id) => Ok(text_result(format!(
                "✅ Capsule synthesized #{id}: '{}' (confidence: {:.0}%, {} sources, {} decisions, {} issues)",
                capsule.topic,
                capsule.confidence * 100.0,
                capsule.source_observations.len(),
                capsule.key_decisions.len(),
                capsule.known_issues.len(),
            ))),
            Err(e) => Ok(error_result(&format!("failed to save capsule: {e}"))),
        },
        Err(e) => Ok(error_result(&format!("capsule synthesis failed: {e}"))),
    }
}

async fn tool_capsule_list_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project");

    match server.store.list_capsules(project.as_deref()) {
        Ok(capsules) => {
            if capsules.is_empty() {
                return Ok(text_result("No capsules found.".into()));
            }
            let mut text = format!("## {} Capsule(s)\n\n", capsules.len());
            for cap in &capsules {
                text.push_str(&format!(
                    "- **{}** (confidence: {:.0}%, v{}) — {} sources, {}\n",
                    cap.topic,
                    cap.confidence * 100.0,
                    cap.version,
                    cap.source_observations.len(),
                    cap.summary.chars().take(80).collect::<String>(),
                ));
            }
            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_capsule_get_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let topic = match get_string(&args, "topic") {
        Some(t) => t,
        None => return Ok(error_result("missing 'topic'")),
    };
    let project = get_string(&args, "project");

    match server.store.get_capsule(&topic, project.as_deref()) {
        Ok(Some(capsule)) => Ok(text_result(capsule.to_markdown())),
        Ok(None) => Ok(error_result(&format!("capsule '{topic}' not found"))),
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_antipatterns_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    let detector = AntiPatternDetector::new(server.store.clone(), None);
    match detector.detect_all(&project) {
        Ok(patterns) => {
            if patterns.is_empty() {
                return Ok(text_result("✅ No anti-patterns detected.".into()));
            }
            let mut text = format!("## ⚠️ Anti-Patterns ({})\n\n", patterns.len());
            for p in &patterns {
                text.push_str(&format!("- {p}\n"));
            }
            text.push_str("\n**Suggestion:** Investigate root causes and consider refactoring.");
            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("anti-pattern detection failed: {e}"))),
    }
}

async fn tool_consolidate_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    let engine = ConsolidationEngine::new(server.store.clone(), None);
    match engine.run_consolidation(&project) {
        Ok(result) => {
            // Evolve graph after consolidation
            let evolver = GraphEvolver::new(server.store.clone(), None);
            let edge_info = match evolver.evolve(&project) {
                Ok(ev) if ev.edges_created > 0 => {
                    format!("\n  - {} graph edges created", ev.edges_created)
                }
                _ => String::new(),
            };
            Ok(text_result(format!(
                "✅ Consolidation complete in {}ms:\n  - {} duplicates merged\n  - {} obsolete marked\n  - {} conflicts found\n  - {} patterns extracted{}",
                result.time_taken_ms,
                result.duplicates_merged,
                result.obsolete_marked,
                result.conflicts_found,
                result.patterns_extracted,
                edge_info,
            )))
        }
        Err(e) => Ok(error_result(&format!("consolidation failed: {e}"))),
    }
}

async fn tool_knowledge_boundary_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    if let Some(domain) = get_string(&args, "domain") {
        let level = get_string(&args, "confidence_level").unwrap_or_else(|| "familiar".into());
        let evidence = get_string(&args, "evidence").unwrap_or_default();

        match server.store.upsert_boundary(&domain, &level, &evidence) {
            Ok(()) => Ok(text_result(format!(
                "✅ Boundary updated: `{domain}` = {level}"
            ))),
            Err(e) => Ok(error_result(&format!("failed: {e}"))),
        }
    } else {
        // Compute boundaries from observations using BoundaryTracker
        let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());
        let tracker = BoundaryTracker::new(server.store.clone());
        match tracker.compute_boundaries(&project) {
            Ok(boundaries) => {
                if boundaries.is_empty() {
                    return Ok(text_result("No knowledge boundaries detected yet.".into()));
                }
                let mut text = String::from("## 🗺️ Knowledge Boundaries\n\n");
                for b in &boundaries {
                    text.push_str(&format!("{}\n", b.format_for_context()));
                }
                Ok(text_result(text))
            }
            Err(e) => Ok(error_result(&format!("failed: {e}"))),
        }
    }
}

async fn tool_transfer_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let project = get_string(&args, "project").unwrap_or_else(|| server.config.project.clone());

    match server.store.get_transfers(&project) {
        Ok(transfers) => {
            if transfers.is_empty() {
                return Ok(text_result(
                    "No knowledge transfers available for this project.".into(),
                ));
            }
            let mut text = format!("## 🔄 Knowledge Transfers ({})\n\n", transfers.len());
            for (id, source, _capsule_id, relevance, accepted) in &transfers {
                let status = if *accepted {
                    "✅ accepted"
                } else {
                    "⏳ pending"
                };
                text.push_str(&format!(
                    "- **#{id}** from `{source}` (relevance: {relevance:.0}%) — {status}\n"
                ));
            }
            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_reviews_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let limit = get_usize(&args, "limit").unwrap_or(10);

    match server.store.get_pending_reviews(None, limit) {
        Ok(reviews) => {
            if reviews.is_empty() {
                return Ok(text_result("✅ No pending reviews.".into()));
            }
            let mut text = format!("## 📚 Pending Reviews ({})\n\n", reviews.len());
            for (obs_id, interval, ease) in &reviews {
                text.push_str(&format!(
                    "- Observation **#{obs_id}**: next interval {interval:.0} days, ease {ease:.2}\n"
                ));
            }
            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_beliefs_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let subject = match get_string(&args, "subject") {
        Some(s) => s,
        None => return Ok(error_result("missing 'subject'")),
    };

    match server.store.get_beliefs(&subject) {
        Ok(beliefs) => {
            if beliefs.is_empty() {
                return Ok(text_result(format!(
                    "No beliefs recorded about '{subject}'."
                )));
            }
            let mut text = format!("## 🧠 Beliefs about '{subject}' ({})\n\n", beliefs.len());
            for (subj, pred, val, conf, state) in &beliefs {
                text.push_str(&format!(
                    "- **{subj}** {pred} **{val}** (confidence: {:.0}%, state: {state})\n",
                    conf * 100.0
                ));
            }
            Ok(text_result(text))
        }
        Err(e) => Ok(error_result(&format!("failed: {e}"))),
    }
}

async fn tool_sync_handler(
    server: &EngramServer,
    args: HashMap<String, serde_json::Value>,
) -> Result<CallToolResult, ErrorData> {
    let action = get_string(&args, "action").unwrap_or_else(|| "status".into());
    let dir = get_string(&args, "dir").unwrap_or_else(|| "./engram-chunks".into());

    match action.as_str() {
        "status" => {
            let state = engram_sync::CrdtState::new();
            let status = engram_sync::get_sync_status(&*server.store, &state);
            Ok(text_result(format!(
                "## 🔄 Sync Status\n\n- Device: {}\n- Clock: {}\n- Last sync: {:?}\n- Pending deltas: {}",
                &status.device_id[..8.min(status.device_id.len())],
                status.vector_clock,
                status.last_sync,
                status.pending_deltas,
            )))
        }
        "export" => {
            let path = std::path::Path::new(&dir);
            match engram_sync::export_chunks(&*server.store, Some(&server.config.project), path) {
                Ok(manifest) => {
                    let mut text = format!(
                        "✅ Exported {} chunk(s) to `{}`\n\n",
                        manifest.chunks.len(),
                        dir
                    );
                    for chunk in &manifest.chunks {
                        text.push_str(&format!(
                            "- {} ({} bytes, {} observations)\n",
                            chunk.filename, chunk.size, chunk.observation_count
                        ));
                    }
                    Ok(text_result(text))
                }
                Err(e) => Ok(error_result(&format!("export failed: {e}"))),
            }
        }
        "import" => {
            let path = std::path::Path::new(&dir);
            match engram_sync::import_chunks(&*server.store, path) {
                Ok(result) => Ok(text_result(format!(
                    "✅ Imported from `{}`:\n- {} observations\n- {} sessions\n- {} duplicates skipped",
                    dir,
                    result.observations_imported,
                    result.sessions_imported,
                    result.duplicates_skipped,
                ))),
                Err(e) => Ok(error_result(&format!("import failed: {e}"))),
            }
        }
        _ => Ok(error_result(
            "unknown sync action. Use: status, export, import",
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────

/// Extract simple subject-predicate-value triples from content and upsert beliefs.
fn extract_and_upsert_beliefs(
    server: &EngramServer,
    _observation_id: i64,
    content: &str,
    _project: &str,
) {
    // Simple heuristic: look for patterns like "X uses Y", "X is Y", "X requires Y"
    let patterns = [
        " uses ",
        " is ",
        " requires ",
        " depends on ",
        " implements ",
    ];
    for line in content.lines() {
        for pattern in &patterns {
            if let Some(pos) = line.find(pattern) {
                let subject = line[..pos].trim();
                let rest = &line[pos + pattern.len()..];
                let value = rest.trim().trim_end_matches('.').trim_end_matches(',');
                let predicate = pattern.trim();

                if !subject.is_empty()
                    && !value.is_empty()
                    && subject.len() < 100
                    && value.len() < 200
                {
                    let _ = server
                        .store
                        .upsert_belief(subject, predicate, value, 0.5, "active");
                }
            }
        }
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn parse_file_line(s: &str) -> (Option<String>, u32) {
    if let Some((file, line_str)) = s.rsplit_once(':')
        && let Ok(line) = line_str.parse::<u32>()
    {
        return (Some(file.to_string()), line);
    }
    (None, 0)
}
