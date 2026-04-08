use crate::ObservationType;

/// Lifecycle policy per observation type.
#[derive(Debug, Clone)]
pub struct LifecyclePolicy {
    pub obs_type: ObservationType,
    pub active_max_age_days: Option<u32>,
    pub stale_after_days: Option<u32>,
    pub archive_after_days: Option<u32>,
    pub auto_delete_after_days: Option<u32>,
    pub require_review_before_delete: bool,
    pub decay_multiplier: f64,
    pub searchable_when_stale: bool,
    pub searchable_when_archived: bool,
}

impl LifecyclePolicy {
    /// Get the default policy for an observation type.
    pub fn for_type(obs_type: ObservationType) -> Self {
        match obs_type {
            // Decision — permanent, never auto-delete, slow decay
            ObservationType::Decision => Self {
                obs_type,
                active_max_age_days: None,
                stale_after_days: None,
                archive_after_days: None,
                auto_delete_after_days: None,
                require_review_before_delete: true,
                decay_multiplier: 0.5,
                searchable_when_stale: true,
                searchable_when_archived: true,
            },
            // Architecture — permanent, minimal decay
            ObservationType::Architecture => Self {
                obs_type,
                active_max_age_days: None,
                stale_after_days: None,
                archive_after_days: None,
                auto_delete_after_days: None,
                require_review_before_delete: true,
                decay_multiplier: 0.3,
                searchable_when_stale: true,
                searchable_when_archived: true,
            },
            // Bugfix — active 90d, archive 180d, preserve for anti-patterns
            ObservationType::Bugfix => Self {
                obs_type,
                active_max_age_days: Some(90),
                stale_after_days: Some(90),
                archive_after_days: Some(180),
                auto_delete_after_days: None,
                require_review_before_delete: false,
                decay_multiplier: 1.0,
                searchable_when_stale: true,
                searchable_when_archived: false,
            },
            // Pattern — permanent, like decisions
            ObservationType::Pattern => Self {
                obs_type,
                active_max_age_days: None,
                stale_after_days: None,
                archive_after_days: None,
                auto_delete_after_days: None,
                require_review_before_delete: true,
                decay_multiplier: 0.5,
                searchable_when_stale: true,
                searchable_when_archived: true,
            },
            // Command — active 30d, auto-purge 180d
            ObservationType::Command => Self {
                obs_type,
                active_max_age_days: Some(30),
                stale_after_days: Some(30),
                archive_after_days: Some(90),
                auto_delete_after_days: Some(180),
                require_review_before_delete: false,
                decay_multiplier: 1.5,
                searchable_when_stale: false,
                searchable_when_archived: false,
            },
            // FileRead/Search — ephemeral, aggressive purge
            ObservationType::FileRead | ObservationType::Search => Self {
                obs_type,
                active_max_age_days: Some(14),
                stale_after_days: Some(14),
                archive_after_days: Some(60),
                auto_delete_after_days: Some(90),
                require_review_before_delete: false,
                decay_multiplier: 2.0,
                searchable_when_stale: false,
                searchable_when_archived: false,
            },
            // Default for remaining types
            _ => Self {
                obs_type,
                active_max_age_days: Some(90),
                stale_after_days: Some(90),
                archive_after_days: Some(180),
                auto_delete_after_days: None,
                require_review_before_delete: false,
                decay_multiplier: 1.0,
                searchable_when_stale: true,
                searchable_when_archived: false,
            },
        }
    }

    /// Get all default policies.
    pub fn all_defaults() -> Vec<Self> {
        use ObservationType::*;
        [
            Bugfix,
            Decision,
            Architecture,
            Pattern,
            Discovery,
            Learning,
            Config,
            Convention,
            ToolUse,
            FileChange,
            Command,
            FileRead,
            Search,
            Manual,
        ]
        .into_iter()
        .map(Self::for_type)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_is_permanent() {
        let p = LifecyclePolicy::for_type(ObservationType::Decision);
        assert!(p.auto_delete_after_days.is_none());
        assert!(p.stale_after_days.is_none());
        assert!(p.require_review_before_delete);
        assert!(p.decay_multiplier < 1.0);
    }

    #[test]
    fn command_auto_purges() {
        let p = LifecyclePolicy::for_type(ObservationType::Command);
        assert_eq!(p.auto_delete_after_days, Some(180));
        assert_eq!(p.stale_after_days, Some(30));
        assert!(!p.require_review_before_delete);
        assert!(p.decay_multiplier > 1.0);
    }

    #[test]
    fn bugfix_archived_not_deleted() {
        let p = LifecyclePolicy::for_type(ObservationType::Bugfix);
        assert!(p.auto_delete_after_days.is_none());
        assert_eq!(p.archive_after_days, Some(180));
        assert!(p.searchable_when_stale);
    }

    #[test]
    fn architecture_is_permanent() {
        let p = LifecyclePolicy::for_type(ObservationType::Architecture);
        assert!(p.auto_delete_after_days.is_none());
        assert!(p.archive_after_days.is_none());
        assert!((p.decay_multiplier - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn fileread_is_ephemeral() {
        let p = LifecyclePolicy::for_type(ObservationType::FileRead);
        assert_eq!(p.auto_delete_after_days, Some(90));
        assert!(!p.searchable_when_stale);
        assert!((p.decay_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn all_defaults_cover_all_types() {
        let policies = LifecyclePolicy::all_defaults();
        assert_eq!(policies.len(), 14); // 14 observation types
    }
}
