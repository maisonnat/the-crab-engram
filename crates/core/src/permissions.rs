use serde::{Deserialize, Serialize};

/// Access level for multi-agent shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    Read,
    Write,
    Admin,
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::Admin => write!(f, "admin"),
        }
    }
}

/// Permission rule for an agent on a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub agent_id: String,
    pub project: String,
    pub access: AccessLevel,
    pub scope_filter: Option<crate::Scope>, // None = all, Some(Personal) = personal only
}

impl PermissionRule {
    pub fn new(agent_id: String, project: String, access: AccessLevel) -> Self {
        Self {
            agent_id,
            project,
            access,
            scope_filter: None,
        }
    }

    /// Check if this rule allows the given access level.
    pub fn allows(&self, required: AccessLevel) -> bool {
        self.access >= required
    }
}

/// Permission engine for multi-agent access control.
#[derive(Debug, Clone)]
pub struct PermissionEngine {
    pub rules: Vec<PermissionRule>,
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: PermissionRule) {
        self.rules.push(rule);
    }

    /// Check if an agent can perform an action on a project.
    pub fn check(&self, agent_id: &str, project: &str, required: AccessLevel) -> bool {
        self.rules
            .iter()
            .any(|r| r.agent_id == agent_id && r.project == project && r.allows(required))
    }

    /// Get all agents with access to a project.
    pub fn agents_for_project(&self, project: &str) -> Vec<&str> {
        self.rules
            .iter()
            .filter(|r| r.project == project)
            .map(|r| r.agent_id.as_str())
            .collect()
    }
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_level_ordering() {
        assert!(AccessLevel::Read < AccessLevel::Write);
        assert!(AccessLevel::Write < AccessLevel::Admin);
    }

    #[test]
    fn permission_allows() {
        let rule = PermissionRule::new("agent-a".into(), "test".into(), AccessLevel::Write);
        assert!(rule.allows(AccessLevel::Read));
        assert!(rule.allows(AccessLevel::Write));
        assert!(!rule.allows(AccessLevel::Admin));
    }

    #[test]
    fn permission_engine_check() {
        let mut engine = PermissionEngine::new();
        engine.add_rule(PermissionRule::new(
            "agent-a".into(),
            "test".into(),
            AccessLevel::Write,
        ));
        engine.add_rule(PermissionRule::new(
            "agent-b".into(),
            "test".into(),
            AccessLevel::Read,
        ));

        assert!(engine.check("agent-a", "test", AccessLevel::Write));
        assert!(engine.check("agent-b", "test", AccessLevel::Read));
        assert!(!engine.check("agent-b", "test", AccessLevel::Write));
        assert!(!engine.check("agent-c", "test", AccessLevel::Read));
    }

    #[test]
    fn agents_for_project() {
        let mut engine = PermissionEngine::new();
        engine.add_rule(PermissionRule::new(
            "agent-a".into(),
            "proj1".into(),
            AccessLevel::Admin,
        ));
        engine.add_rule(PermissionRule::new(
            "agent-b".into(),
            "proj1".into(),
            AccessLevel::Read,
        ));
        engine.add_rule(PermissionRule::new(
            "agent-c".into(),
            "proj2".into(),
            AccessLevel::Write,
        ));

        let agents = engine.agents_for_project("proj1");
        assert_eq!(agents.len(), 2);
    }
}
