use crate::ObservationType;

/// Suggest a topic key based on observation type and title.
/// Format: "{family}/{slug}" — compatible with Engram Go.
pub fn suggest_topic_key(obs_type: ObservationType, title: &str) -> String {
    let family = match obs_type {
        ObservationType::Architecture => "architecture",
        ObservationType::Bugfix => "bug",
        ObservationType::Decision => "decision",
        ObservationType::Pattern => "pattern",
        ObservationType::Discovery => "discovery",
        ObservationType::Learning => "learning",
        ObservationType::Config => "config",
        ObservationType::Convention => "convention",
        ObservationType::ToolUse => "tool",
        ObservationType::FileChange => "file",
        ObservationType::Command => "command",
        ObservationType::FileRead => "file",
        ObservationType::Search => "search",
        ObservationType::Manual => "manual",
    };
    format!("{family}/{}", slugify(title))
}

/// Slugify text: lowercase, alphanumeric + hyphens only.
pub fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut last_was_hyphen = false;

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen && !slug.is_empty() {
            slug.push('-');
            last_was_hyphen = true;
        }
    }

    // Trim trailing hyphen
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(
            slugify("Fix N+1 Query in UserList!"),
            "fix-n-1-query-in-userlist"
        );
    }

    #[test]
    fn slugify_preserves_hyphens() {
        assert_eq!(slugify("already-slugified"), "already-slugified");
    }

    #[test]
    fn slugify_empty() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("C++ Template"), "c-template");
    }

    #[test]
    fn suggest_topic_architecture() {
        assert_eq!(
            suggest_topic_key(ObservationType::Architecture, "Auth JWT Flow"),
            "architecture/auth-jwt-flow"
        );
    }

    #[test]
    fn suggest_topic_bugfix() {
        assert_eq!(
            suggest_topic_key(ObservationType::Bugfix, "Fix N+1 in UserList"),
            "bug/fix-n-1-in-userlist"
        );
    }

    #[test]
    fn suggest_topic_decision() {
        assert_eq!(
            suggest_topic_key(ObservationType::Decision, "Use SQLite over PostgreSQL"),
            "decision/use-sqlite-over-postgresql"
        );
    }
}
