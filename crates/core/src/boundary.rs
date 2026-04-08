use serde::{Deserialize, Serialize};

/// Confidence level per knowledge domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    Expert,     // >20 observations, high confidence
    Proficient, // 10-20 observations
    Familiar,   // 5-10 observations
    Aware,      // 1-4 observations
    Unknown,    // 0 observations, detected as relevant
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expert => write!(f, "Expert"),
            Self::Proficient => write!(f, "Proficient"),
            Self::Familiar => write!(f, "Familiar"),
            Self::Aware => write!(f, "Aware"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl ConfidenceLevel {
    /// Determine level from observation count.
    pub fn from_count(count: usize) -> Self {
        match count {
            0 => Self::Unknown,
            1..=4 => Self::Aware,
            5..=9 => Self::Familiar,
            10..=20 => Self::Proficient,
            _ => Self::Expert,
        }
    }

    /// Numeric score for comparison (higher = more confident).
    pub fn score(&self) -> f64 {
        match self {
            Self::Expert => 1.0,
            Self::Proficient => 0.75,
            Self::Familiar => 0.5,
            Self::Aware => 0.25,
            Self::Unknown => 0.0,
        }
    }
}

/// Evidence supporting a knowledge boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryEvidence {
    pub observations_count: u32,
    pub successful_applications: u32,
    pub failed_applications: u32,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for BoundaryEvidence {
    fn default() -> Self {
        Self {
            observations_count: 0,
            successful_applications: 0,
            failed_applications: 0,
            last_used: None,
        }
    }
}

/// Tracks what the system knows about a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBoundary {
    pub domain: String,
    pub confidence_level: ConfidenceLevel,
    pub evidence: BoundaryEvidence,
}

impl KnowledgeBoundary {
    pub fn new(domain: String) -> Self {
        Self {
            domain,
            confidence_level: ConfidenceLevel::Unknown,
            evidence: BoundaryEvidence::default(),
        }
    }

    /// Recalculate confidence level from evidence.
    pub fn recalculate(&mut self) {
        self.confidence_level =
            ConfidenceLevel::from_count(self.evidence.observations_count as usize);

        // Lower level if failures exceed successes
        if self.evidence.failed_applications > self.evidence.successful_applications
            && self.evidence.successful_applications > 0
        {
            self.confidence_level = match self.confidence_level {
                ConfidenceLevel::Expert => ConfidenceLevel::Proficient,
                ConfidenceLevel::Proficient => ConfidenceLevel::Familiar,
                ConfidenceLevel::Familiar => ConfidenceLevel::Aware,
                other => other,
            };
        }
    }

    /// Record a successful application.
    pub fn record_success(&mut self) {
        self.evidence.successful_applications += 1;
        self.evidence.last_used = Some(chrono::Utc::now());
        self.recalculate();
    }

    /// Record a failed application.
    pub fn record_failure(&mut self) {
        self.evidence.failed_applications += 1;
        self.evidence.last_used = Some(chrono::Utc::now());
        self.recalculate();
    }

    /// Add observations to this domain.
    pub fn add_observations(&mut self, count: u32) {
        self.evidence.observations_count += count;
        self.recalculate();
    }

    /// Format for display in context injection.
    pub fn format_for_context(&self) -> String {
        let emoji = match self.confidence_level {
            ConfidenceLevel::Expert => "🟢",
            ConfidenceLevel::Proficient => "🔵",
            ConfidenceLevel::Familiar => "🟡",
            ConfidenceLevel::Aware => "🟠",
            ConfidenceLevel::Unknown => "🔴",
        };

        format!(
            "{emoji} {}: {} ({} observations, {} successes, {} failures)",
            self.domain,
            self.confidence_level,
            self.evidence.observations_count,
            self.evidence.successful_applications,
            self.evidence.failed_applications
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_level_from_count() {
        assert_eq!(ConfidenceLevel::from_count(0), ConfidenceLevel::Unknown);
        assert_eq!(ConfidenceLevel::from_count(2), ConfidenceLevel::Aware);
        assert_eq!(ConfidenceLevel::from_count(7), ConfidenceLevel::Familiar);
        assert_eq!(ConfidenceLevel::from_count(15), ConfidenceLevel::Proficient);
        assert_eq!(ConfidenceLevel::from_count(25), ConfidenceLevel::Expert);
    }

    #[test]
    fn boundary_recalculates() {
        let mut b = KnowledgeBoundary::new("rust".into());
        assert_eq!(b.confidence_level, ConfidenceLevel::Unknown);

        b.add_observations(10);
        assert_eq!(b.confidence_level, ConfidenceLevel::Proficient);
    }

    #[test]
    fn failures_lower_level() {
        let mut b = KnowledgeBoundary::new("k8s".into());
        b.add_observations(10);
        assert_eq!(b.confidence_level, ConfidenceLevel::Proficient);

        // More failures than successes → lower level
        b.record_failure();
        b.record_failure();
        b.record_failure();
        b.record_success();

        assert_eq!(b.confidence_level, ConfidenceLevel::Familiar);
    }

    #[test]
    fn format_for_context() {
        let mut b = KnowledgeBoundary::new("rust".into());
        b.add_observations(25);
        b.record_success();

        let fmt = b.format_for_context();
        assert!(fmt.contains("rust"));
        assert!(fmt.contains("Expert"));
        assert!(fmt.contains("25"));
    }

    #[test]
    fn confidence_score_ordering() {
        assert!(ConfidenceLevel::Expert.score() > ConfidenceLevel::Proficient.score());
        assert!(ConfidenceLevel::Proficient.score() > ConfidenceLevel::Familiar.score());
        assert!(ConfidenceLevel::Familiar.score() > ConfidenceLevel::Aware.score());
    }
}
