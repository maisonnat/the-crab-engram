use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Belief state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefState {
    Active,     // One source, not contradicted
    Confirmed,  // Multiple sources corroborate
    Contested,  // Contradictory evidence without resolution
    Superseded, // Replaced by newer belief
    Retracted,  // Explicitly corrected
}

impl std::fmt::Display for BeliefState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Contested => write!(f, "Contested"),
            Self::Superseded => write!(f, "Superseded"),
            Self::Retracted => write!(f, "Retracted"),
        }
    }
}

/// Operations on beliefs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeliefOperation {
    Create,
    Update,
    Confirm,
    Contest,
    Retract,
    Resolve,
}

/// A historical belief value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalBelief {
    pub value: String,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub superseded_by: i64,
    pub reason: String,
}

/// A belief — evolves with evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: i64,
    pub subject: String,
    pub current_value: String,
    pub previous_values: Vec<HistoricalBelief>,
    pub confidence: f64,
    pub last_evidence: Vec<i64>,
    pub state: BeliefState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Belief {
    pub fn new(subject: String, value: String) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            subject,
            current_value: value,
            previous_values: Vec::new(),
            confidence: 0.5,
            last_evidence: Vec::new(),
            state: BeliefState::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Determine the operation based on new evidence.
    pub fn process_evidence(&self, new_value: &str, new_confidence: f64) -> BeliefOperation {
        if self.current_value == new_value {
            // Same value → confirm
            if self.confidence > 0.9 {
                return BeliefOperation::Confirm;
            }
            return BeliefOperation::Confirm;
        }

        // Different value
        let confidence_delta = new_confidence - self.confidence;

        if confidence_delta > 0.2 {
            // Much stronger evidence → update
            BeliefOperation::Update
        } else if confidence_delta > -0.1 {
            // Similar confidence → contest
            BeliefOperation::Contest
        } else {
            // Weaker evidence → ignore (confirm existing)
            BeliefOperation::Confirm
        }
    }

    /// Execute an operation on this belief.
    pub fn execute_operation(&mut self, op: BeliefOperation, value: &str, evidence_id: i64) {
        let now = Utc::now();
        self.updated_at = now;
        self.last_evidence.push(evidence_id);

        match op {
            BeliefOperation::Create => {
                self.state = BeliefState::Active;
            }
            BeliefOperation::Update => {
                // Move current to history
                self.previous_values.push(HistoricalBelief {
                    value: self.current_value.clone(),
                    valid_from: self.created_at,
                    valid_until: now,
                    superseded_by: evidence_id,
                    reason: "Stronger evidence found".into(),
                });
                self.current_value = value.to_string();
                self.state = BeliefState::Active;
                self.confidence = (self.confidence + 0.2).min(1.0);
            }
            BeliefOperation::Confirm => {
                self.confidence = (self.confidence + 0.1).min(1.0);
                if self.confidence > 0.9 && self.last_evidence.len() >= 3 {
                    self.state = BeliefState::Confirmed;
                }
            }
            BeliefOperation::Contest => {
                self.state = BeliefState::Contested;
            }
            BeliefOperation::Retract => {
                self.previous_values.push(HistoricalBelief {
                    value: self.current_value.clone(),
                    valid_from: self.created_at,
                    valid_until: now,
                    superseded_by: evidence_id,
                    reason: "User correction".into(),
                });
                self.current_value = value.to_string();
                self.state = BeliefState::Retracted;
            }
            BeliefOperation::Resolve => {
                self.state = BeliefState::Confirmed;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn belief_new_is_active() {
        let b = Belief::new("auth_method".into(), "RS256".into());
        assert_eq!(b.state, BeliefState::Active);
        assert!((b.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn same_value_confirms() {
        let b = Belief::new("auth".into(), "RS256".into());
        let op = b.process_evidence("RS256", 0.8);
        assert_eq!(op, BeliefOperation::Confirm);
    }

    #[test]
    fn stronger_evidence_updates() {
        let b = Belief::new("auth".into(), "RS256".into());
        let op = b.process_evidence("ES256", 0.85);
        assert_eq!(op, BeliefOperation::Update);
    }

    #[test]
    fn similar_confidence_contests() {
        let b = Belief::new("auth".into(), "RS256".into());
        let op = b.process_evidence("ES256", 0.55);
        assert_eq!(op, BeliefOperation::Contest);
    }

    #[test]
    fn weaker_evidence_ignored() {
        let b = Belief::new("auth".into(), "RS256".into());
        let op = b.process_evidence("HS256", 0.3);
        assert_eq!(op, BeliefOperation::Confirm);
    }

    #[test]
    fn update_preserves_history() {
        let mut b = Belief::new("auth".into(), "RS256".into());
        b.execute_operation(BeliefOperation::Update, "ES256", 1);
        assert_eq!(b.current_value, "ES256");
        assert_eq!(b.previous_values.len(), 1);
        assert_eq!(b.previous_values[0].value, "RS256");
    }

    #[test]
    fn confirm_becomes_confirmed() {
        let mut b = Belief::new("auth".into(), "RS256".into());
        for i in 0..5 {
            b.execute_operation(BeliefOperation::Confirm, "RS256", i);
        }
        assert_eq!(b.state, BeliefState::Confirmed);
    }

    #[test]
    fn contested_state() {
        let mut b = Belief::new("auth".into(), "RS256".into());
        b.execute_operation(BeliefOperation::Contest, "ES256", 1);
        assert_eq!(b.state, BeliefState::Contested);
    }
}
