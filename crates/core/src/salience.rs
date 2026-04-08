use serde::{Deserialize, Serialize};

/// Emotional salience of a memory — affects decay rate.
/// Higher salience = slower decay = stays relevant longer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySalience {
    /// Emotional valence: -1.0 (frustration) to 1.0 (achievement)
    pub emotional_valence: f64,
    /// Surprise factor: 0.0 (expected) to 1.0 (unexpected)
    pub surprise_factor: f64,
    /// Effort invested: 0.0 (2min fix) to 1.0 (4h debugging)
    pub effort_invested: f64,
}

impl Default for MemorySalience {
    fn default() -> Self {
        Self {
            emotional_valence: 0.0,
            surprise_factor: 0.0,
            effort_invested: 0.0,
        }
    }
}

impl MemorySalience {
    /// Compute the decay multiplier based on salience.
    /// Higher salience → multiplier > 1.0 → slower decay.
    /// final_score = base_score * decay_multiplier
    pub fn decay_multiplier(&self) -> f64 {
        let valence_boost = 1.0 + self.emotional_valence * 0.3;
        let surprise_boost = 1.0 + self.surprise_factor * 0.5;
        let result = valence_boost * surprise_boost;
        result.max(0.1) // Never go below 0.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_salience_neutral() {
        let s = MemorySalience::default();
        assert!((s.emotional_valence).abs() < f64::EPSILON);
        assert!((s.surprise_factor).abs() < f64::EPSILON);
        assert!((s.decay_multiplier() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn high_surprise_slows_decay() {
        let s = MemorySalience {
            emotional_valence: 0.0,
            surprise_factor: 1.0,
            effort_invested: 0.0,
        };
        // 1.0 * (1.0 + 0.0 * 0.3) * (1.0 + 1.0 * 0.5) = 1.0 * 1.0 * 1.5 = 1.5
        assert!((s.decay_multiplier() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn positive_valence_slows_decay() {
        let s = MemorySalience {
            emotional_valence: 1.0,
            surprise_factor: 0.0,
            effort_invested: 0.0,
        };
        // (1.0 + 1.0 * 0.3) * 1.0 = 1.3
        assert!((s.decay_multiplier() - 1.3).abs() < 1e-10);
    }

    #[test]
    fn negative_valence_speeds_decay() {
        let s = MemorySalience {
            emotional_valence: -1.0,
            surprise_factor: 0.0,
            effort_invested: 0.0,
        };
        // (1.0 + (-1.0) * 0.3) * 1.0 = 0.7
        assert!((s.decay_multiplier() - 0.7).abs() < 1e-10);
    }

    #[test]
    fn combined_salience() {
        let s = MemorySalience {
            emotional_valence: 0.5,
            surprise_factor: 0.8,
            effort_invested: 0.9,
        };
        // (1.0 + 0.5 * 0.3) * (1.0 + 0.8 * 0.5) = 1.15 * 1.4 = 1.61
        assert!((s.decay_multiplier() - 1.61).abs() < 1e-10);
    }

    #[test]
    fn decay_multiplier_never_negative() {
        let s = MemorySalience {
            emotional_valence: -1.0,
            surprise_factor: -1.0,
            effort_invested: 0.0,
        };
        assert!(s.decay_multiplier() >= 0.1);
    }
}
