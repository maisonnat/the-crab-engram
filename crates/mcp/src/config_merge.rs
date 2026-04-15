use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActionKind {
    Created,
    Updated,
    Skipped,
    Removed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetupAction {
    pub action: ActionKind,
    pub target: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SetupResult {
    pub actions: Vec<SetupAction>,
}

impl SetupResult {
    pub fn display_table(&self) {
        println!("{:<12} {:<50} DETAIL", "ACTION", "TARGET");
        println!("{}", "-".repeat(80));
        for a in &self.actions {
            let kind = match a.action {
                ActionKind::Created => "Created",
                ActionKind::Updated => "Updated",
                ActionKind::Skipped => "Skipped",
                ActionKind::Removed => "Removed",
            };
            println!("{:<12} {:<50} {}", kind, a.target, a.detail);
        }
    }
}

pub fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_string = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if in_string {
            result.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '"' => {
                    in_string = true;
                    result.push(c);
                }
                '/' if chars.peek() == Some(&'/') => {
                    for nc in chars.by_ref() {
                        if nc == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                }
                _ => {
                    result.push(c);
                }
            }
        }
    }
    result
}

pub fn merge_mcp_entry(
    config: &serde_json::Value,
    profile: &str,
    project: &str,
) -> serde_json::Value {
    let mut config = config.clone();

    let mcp_entry = serde_json::json!({
        "type": "local",
        "command": ["the-crab-engram", "mcp", "--project", project, "--profile", profile],
        "enabled": true
    });

    if let Some(obj) = config.as_object_mut() {
        let mcp = obj.entry("mcp").or_insert_with(|| serde_json::json!({}));
        if let Some(mcp_obj) = mcp.as_object_mut() {
            mcp_obj.insert("the-crab-engram".to_string(), mcp_entry);
        }
    }

    config
}

pub fn remove_mcp_entry(config: &serde_json::Value) -> serde_json::Value {
    let mut config = config.clone();
    if let Some(obj) = config.as_object_mut() {
        if let Some(mcp) = obj.get_mut("mcp")
            && let Some(mcp_obj) = mcp.as_object_mut()
        {
            mcp_obj.remove("the-crab-engram");
        }
        if let Some(plugin) = obj.get_mut("plugin")
            && let Some(arr) = plugin.as_array_mut()
        {
            arr.retain(|v| {
                v.as_str()
                    .map(|s| !s.contains("the-crab-engram"))
                    .unwrap_or(true)
            });
        }
    }
    config
}

pub fn merge_plugin_path(config: &serde_json::Value, plugin_path: &str) -> serde_json::Value {
    let mut config = config.clone();

    if let Some(obj) = config.as_object_mut() {
        let plugin = obj.entry("plugin").or_insert_with(|| serde_json::json!([]));
        if let Some(arr) = plugin.as_array_mut() {
            let already_present = arr.iter().any(|v| v.as_str() == Some(plugin_path));
            if !already_present {
                arr.push(serde_json::Value::String(plugin_path.to_string()));
            }
        }
    }

    config
}

pub fn merge_agents_md(existing: &str, protocol_block: &str) -> String {
    let start_marker = "<!-- gentle-ai:engram-protocol -->";
    let end_marker = "<!-- /gentle-ai:engram-protocol -->";

    if let Some(start_idx) = existing.find(start_marker) {
        let before = &existing[..start_idx];
        if let Some(end_idx) = existing.find(end_marker) {
            let after_end = end_idx + end_marker.len();
            let after = &existing[after_end..];
            let mut result = before.to_string();
            result.push_str(start_marker);
            result.push('\n');
            result.push_str(protocol_block);
            result.push('\n');
            result.push_str(end_marker);
            result.push_str(after);
            return result;
        }
    }

    let mut result = existing.to_string();
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result.push('\n');
    result.push_str(start_marker);
    result.push('\n');
    result.push_str(protocol_block);
    result.push('\n');
    result.push_str(end_marker);
    result.push('\n');
    result
}

pub fn generate_memory_protocol() -> String {
    r#"## Engram Persistent Memory — Protocol

You have access to Engram, a persistent memory system that survives across sessions and compactions.
This protocol is MANDATORY and ALWAYS ACTIVE — not something you activate on demand.

### PROACTIVE SAVE TRIGGERS (mandatory — do NOT wait for user to ask)

Call `mem_save` IMMEDIATELY and WITHOUT BEING ASKED after any of these:
- Architecture or design decision made
- Team convention documented or established
- Workflow change agreed upon
- Tool or library choice made with tradeoffs
- Bug fix completed (include root cause)
- Feature implemented with non-obvious approach
- Notion/Jira/GitHub artifact created or updated with significant content
- Configuration change or environment setup done
- Non-obvious discovery about the codebase
- Gotcha, edge case, or unexpected behavior found
- Pattern established (naming, structure, convention)
- User preference or constraint learned

Self-check after EVERY task: "Did I make a decision, fix a bug, learn something non-obvious, or establish a convention? If yes, call mem_save NOW."

Format for `mem_save`:
- **title**: Verb + what — short, searchable (e.g. "Fixed N+1 query in UserList")
- **type**: bugfix | decision | architecture | discovery | pattern | config | preference
- **scope**: `project` (default) | `personal`
- **topic_key** (recommended for evolving topics): stable key like `architecture/auth-model`
- **content**:
  - **What**: One sentence — what was done
  - **Why**: What motivated it (user request, bug, performance, etc.)
  - **Where**: Files or paths affected
  - **Learned**: Gotchas, edge cases, things that surprised you (omit if none)

Topic update rules:
- Different topics MUST NOT overwrite each other
- Same topic evolving → use same `topic_key` (upsert)
- Unsure about key → call `mem_suggest_topic_key` first
- Know exact ID to fix → use `mem_update`

### WHEN TO SEARCH MEMORY

On any variation of "remember", "recall", "what did we do", "how did we solve", "recordar", "qué hicimos", or references to past work:
1. Call `mem_context` — checks recent session history (fast, cheap)
2. If not found, call `mem_search` with relevant keywords
3. If found, use `mem_get_observation` for full untruncated content

Also search PROACTIVELY when:
- Starting work on something that might have been done before
- User mentions a topic you have no context on
- User's FIRST message references the project, a feature, or a problem — call `mem_search` with keywords from their message to check for prior work before responding

### SESSION CLOSE PROTOCOL (mandatory)

Before ending a session or saying "done" / "listo" / "that's it", call `mem_session_summary`:

## Goal
[What we were working on this session]

## Instructions
[User preferences or constraints discovered — skip if none]

## Discoveries
- [Technical findings, gotchas, non-obvious learnings]

## Accomplished
- [Completed items with key details]

## Next Steps
- [What remains to be done — for the next session]

## Relevant Files
- path/to/file — [what it does or what changed]

This is NOT optional. If you skip this, the next session starts blind.

### AFTER COMPACTION

If you see a compaction message or "FIRST ACTION REQUIRED":
1. IMMEDIATELY call `mem_session_summary` with the compacted summary content — this persists what was done before compaction
2. Call `mem_context` to recover additional context from previous sessions
3. Only THEN continue working

Do not skip step 1. Without it, everything done before compaction is lost from memory."#
        .to_string()
}
