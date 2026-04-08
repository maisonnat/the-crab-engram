use serde::{Deserialize, Serialize};

/// Attachment types for multimodal observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Attachment {
    CodeDiff {
        file_path: String,
        before_hash: String,
        after_hash: String,
        diff: String,
    },
    TerminalOutput {
        command: String,
        output: String, // Truncated: last N lines + hash of full output
        exit_code: i32,
    },
    ErrorTrace {
        error_type: String,
        message: String,
        stack_trace: String,
        file_line: Option<(String, u32)>,
    },
    GitCommit {
        hash: String,
        message: String,
        files_changed: Vec<String>,
        diff_summary: String,
    },
}

impl Attachment {
    /// Get the text content for embedding/searching.
    pub fn text_content(&self) -> &str {
        match self {
            Self::CodeDiff { diff, .. } => diff,
            Self::TerminalOutput { output, .. } => output,
            Self::ErrorTrace { message, .. } => message,
            Self::GitCommit { message, .. } => message,
        }
    }

    /// Get enriched text for embedding — combines fields for better semantic matching.
    ///
    /// Unlike `text_content()` which returns only the primary field,
    /// this includes context (file paths, commands, error types) to produce
    /// higher quality embeddings.
    pub fn embeddable_text(&self) -> String {
        match self {
            Self::CodeDiff {
                file_path,
                diff,
                before_hash,
                after_hash,
            } => {
                format!("Code change in {file_path} ({before_hash}..{after_hash}):\n{diff}")
            }
            Self::TerminalOutput {
                command,
                output,
                exit_code,
            } => {
                format!("Command: {command} (exit {exit_code}):\n{output}")
            }
            Self::ErrorTrace {
                error_type,
                message,
                stack_trace,
                file_line,
            } => {
                let location = file_line
                    .as_ref()
                    .map(|(f, l)| format!("{f}:{l}"))
                    .unwrap_or_else(|| "unknown".into());
                format!("{error_type}: {message}\nLocation: {location}\n{stack_trace}")
            }
            Self::GitCommit {
                hash,
                message,
                files_changed,
                diff_summary,
            } => {
                let files = files_changed.join(", ");
                format!("Commit {hash}: {message}\nFiles: {files}\n{diff_summary}")
            }
        }
    }

    /// Get a short description of the attachment.
    pub fn description(&self) -> String {
        match self {
            Self::CodeDiff { file_path, .. } => format!("Code change in {file_path}"),
            Self::TerminalOutput {
                command, exit_code, ..
            } => {
                format!("Command: {command} (exit: {exit_code})")
            }
            Self::ErrorTrace {
                error_type,
                message,
                ..
            } => {
                format!("{error_type}: {message}")
            }
            Self::GitCommit { hash, message, .. } => {
                format!("Commit {}: {}", &hash[..8.min(hash.len())], message)
            }
        }
    }

    /// Truncate output to max_lines.
    pub fn truncate_terminal_output(output: &str, max_lines: usize) -> String {
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() <= max_lines {
            return output.to_string();
        }
        let tail = &lines[lines.len() - max_lines..];
        format!(
            "... ({} lines truncated)\n{}",
            lines.len() - max_lines,
            tail.join("\n")
        )
    }
}

/// Multimodal observation wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalObservation {
    pub id: i64,
    pub text_content: String,
    pub attachments: Vec<Attachment>,
}

impl MultimodalObservation {
    pub fn new(id: i64, text_content: String) -> Self {
        Self {
            id,
            text_content,
            attachments: Vec::new(),
        }
    }

    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.attachments.push(attachment);
    }

    /// Get all searchable text from this observation and its attachments.
    pub fn searchable_text(&self) -> String {
        let mut text = self.text_content.clone();
        for attachment in &self.attachments {
            text.push('\n');
            text.push_str(attachment.text_content());
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_diff_attachment() {
        let att = Attachment::CodeDiff {
            file_path: "src/main.rs".into(),
            before_hash: "abc123".into(),
            after_hash: "def456".into(),
            diff: "+fn new() {}\n-old_fn()".into(),
        };
        assert_eq!(att.text_content(), "+fn new() {}\n-old_fn()");
        assert!(att.description().contains("src/main.rs"));
    }

    #[test]
    fn terminal_output_attachment() {
        let att = Attachment::TerminalOutput {
            command: "cargo test".into(),
            output: "test passed".into(),
            exit_code: 0,
        };
        assert_eq!(att.text_content(), "test passed");
        assert!(att.description().contains("cargo test"));
    }

    #[test]
    fn error_trace_attachment() {
        let att = Attachment::ErrorTrace {
            error_type: "panic".into(),
            message: "index out of bounds".into(),
            stack_trace: "at main.rs:42".into(),
            file_line: Some(("main.rs".into(), 42)),
        };
        assert_eq!(att.text_content(), "index out of bounds");
    }

    #[test]
    fn truncate_terminal_output() {
        let output = "line1\nline2\nline3\nline4\nline5";
        let truncated = Attachment::truncate_terminal_output(output, 2);
        assert!(truncated.contains("3 lines truncated"));
        assert!(truncated.contains("line4"));
        assert!(truncated.contains("line5"));
        assert!(!truncated.contains("line1"));
    }

    #[test]
    fn truncate_no_change_if_short() {
        let output = "line1\nline2";
        let truncated = Attachment::truncate_terminal_output(output, 5);
        assert_eq!(truncated, output);
    }

    #[test]
    fn multimodal_searchable_text() {
        let mut obs = MultimodalObservation::new(1, "main content".into());
        obs.add_attachment(Attachment::GitCommit {
            hash: "abc123def".into(),
            message: "fix auth bug".into(),
            files_changed: vec!["src/auth.rs".into()],
            diff_summary: "changed validation".into(),
        });

        let searchable = obs.searchable_text();
        assert!(searchable.contains("main content"));
        assert!(searchable.contains("fix auth bug"));
    }

    #[test]
    fn embeddable_text_code_diff() {
        let att = Attachment::CodeDiff {
            file_path: "src/auth.rs".into(),
            before_hash: "abc123".into(),
            after_hash: "def456".into(),
            diff: "+fn validate() {}".into(),
        };
        let text = att.embeddable_text();
        assert!(text.contains("src/auth.rs"));
        assert!(text.contains("abc123..def456"));
        assert!(text.contains("+fn validate() {}"));
    }

    #[test]
    fn embeddable_text_terminal_output() {
        let att = Attachment::TerminalOutput {
            command: "cargo test".into(),
            output: "running 5 tests\n5 passed".into(),
            exit_code: 0,
        };
        let text = att.embeddable_text();
        assert!(text.contains("Command: cargo test"));
        assert!(text.contains("exit 0"));
        assert!(text.contains("5 passed"));
    }

    #[test]
    fn embeddable_text_error_trace() {
        let att = Attachment::ErrorTrace {
            error_type: "panic".into(),
            message: "index out of bounds".into(),
            stack_trace: "at main.rs:42".into(),
            file_line: Some(("main.rs".into(), 42)),
        };
        let text = att.embeddable_text();
        assert!(text.contains("panic: index out of bounds"));
        assert!(text.contains("main.rs:42"));
        assert!(text.contains("at main.rs:42"));
    }

    #[test]
    fn embeddable_text_git_commit() {
        let att = Attachment::GitCommit {
            hash: "abc123def456".into(),
            message: "fix: auth validation".into(),
            files_changed: vec!["src/auth.rs".into(), "tests/auth_test.rs".into()],
            diff_summary: "+validate_token".into(),
        };
        let text = att.embeddable_text();
        assert!(text.contains("abc123def456"));
        assert!(text.contains("fix: auth validation"));
        assert!(text.contains("src/auth.rs"));
        assert!(text.contains("+validate_token"));
    }

    #[test]
    fn embeddable_text_richer_than_text_content() {
        let att = Attachment::CodeDiff {
            file_path: "src/api/routes.rs".into(),
            before_hash: "aaa".into(),
            after_hash: "bbb".into(),
            diff: "+fn handler() {}".into(),
        };
        // text_content returns only diff
        assert_eq!(att.text_content(), "+fn handler() {}");
        // embeddable_text includes file path + hashes + diff
        let enriched = att.embeddable_text();
        assert!(enriched.contains("src/api/routes.rs"));
        assert!(enriched.len() > att.text_content().len());
    }
}
