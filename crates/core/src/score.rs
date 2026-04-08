use chrono::{DateTime, Utc};

/// Half-life for temporal decay in days.
const HALF_LIFE_DAYS: f64 = 30.0;

/// Access count that yields maximum frequency score.
const MAX_ACCESS_FOR_FULL_SCORE: f64 = 100.0;

/// Compute relevance score based on temporal decay, access frequency, and pinning.
///
/// Design: extensible for salience modification in F2.5.7.
pub fn decay_score(created_at: DateTime<Utc>, access_count: i64, pinned: bool) -> f64 {
    decay_score_with_lifecycle(created_at, access_count, pinned, 1.0)
}

/// Compute relevance score with lifecycle decay multiplier.
///
/// `decay_multiplier` comes from LifecyclePolicy::decay_multiplier:
/// - Decision: 0.5 (slow decay)
/// - Architecture: 0.3 (very slow)
/// - FileRead/Search: 2.0 (fast decay)
/// - Default: 1.0
pub fn decay_score_with_lifecycle(
    created_at: DateTime<Utc>,
    access_count: i64,
    pinned: bool,
    decay_multiplier: f64,
) -> f64 {
    let recency = recency_score(created_at, pinned, decay_multiplier);
    let frequency = frequency_score(access_count);

    // Weighted: 0.5 recency + 0.5 frequency
    0.5 * recency + 0.5 * frequency
}

/// Temporal decay with half-life. Pinned = always max recency.
/// `decay_multiplier` controls how fast the half-life degrades.
fn recency_score(created_at: DateTime<Utc>, pinned: bool, decay_multiplier: f64) -> f64 {
    if pinned {
        return 1.0;
    }

    let age_days = (Utc::now() - created_at).num_seconds() as f64 / 86400.0;
    // Apply lifecycle multiplier: effective age = age * multiplier
    let effective_age = age_days * decay_multiplier;
    // Exponential decay: score = 0.5^(effective_age/half_life)
    0.5f64.powf(effective_age / HALF_LIFE_DAYS)
}

/// Logarithmic frequency boost. Scales from 0 to 1 as access_count approaches MAX_ACCESS_FOR_FULL_SCORE.
fn frequency_score(access_count: i64) -> f64 {
    if access_count <= 0 {
        return 0.0;
    }
    let score = (access_count as f64).ln() / MAX_ACCESS_FOR_FULL_SCORE.ln();
    score.min(1.0)
}

/// Compute final score combining multiple signals.
/// Extensible: salience multiplier applied in F2.5.7.
pub fn compute_final_score(
    fts_score: f64,
    vector_score: f64,
    recency_score: f64,
    frequency_score: f64,
) -> f64 {
    0.3 * fts_score + 0.3 * vector_score + 0.2 * recency_score + 0.2 * frequency_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn pinned_has_max_recency() {
        let old = Utc::now() - Duration::days(365);
        assert!((recency_score(old, true, 1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn recent_score_higher_than_old() {
        let recent = Utc::now() - Duration::days(1);
        let old = Utc::now() - Duration::days(90);
        assert!(recency_score(recent, false, 1.0) > recency_score(old, false, 1.0));
    }

    #[test]
    fn frequency_boosts_score() {
        let f0 = frequency_score(0);
        let f5 = frequency_score(5);
        let f50 = frequency_score(50);
        let f100 = frequency_score(100);
        assert_eq!(f0, 0.0);
        assert!(f5 > f0);
        assert!(f50 > f5);
        assert!(f100 > f50);
        assert!(f100 <= 1.0);
    }

    #[test]
    fn decay_score_combines_signals() {
        let recent = Utc::now() - Duration::days(1);
        let score = decay_score(recent, 10, false);
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn pinned_score_beats_old_unpinned() {
        let old = Utc::now() - Duration::days(180);
        let pinned_score = decay_score(old, 5, true);
        let unpinned_score = decay_score(old, 5, false);
        assert!(pinned_score > unpinned_score);
    }

    #[test]
    fn compute_final_score_weighted() {
        let score = compute_final_score(1.0, 0.8, 0.6, 0.4);
        let expected = 0.3 * 1.0 + 0.3 * 0.8 + 0.2 * 0.6 + 0.2 * 0.4;
        assert!((score - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn lifecycle_multiplier_affects_decay() {
        let old = Utc::now() - Duration::days(60);
        // Decision (0.5x) should decay slower than default
        let decision_score = decay_score_with_lifecycle(old, 5, false, 0.5);
        let default_score = decay_score_with_lifecycle(old, 5, false, 1.0);
        assert!(decision_score > default_score);

        // FileRead (2.0x) should decay faster than default
        let fileread_score = decay_score_with_lifecycle(old, 5, false, 2.0);
        assert!(fileread_score < default_score);
    }

    #[test]
    fn lifecycle_multiplier_ignores_pinned() {
        let old = Utc::now() - Duration::days(365);
        // Pinned should always have recency=1.0 regardless of multiplier
        // (combined score depends on frequency, so just check it's higher than unpinned)
        let pinned_score = decay_score_with_lifecycle(old, 5, true, 2.0);
        let unpinned_score = decay_score_with_lifecycle(old, 5, false, 2.0);
        assert!(pinned_score > unpinned_score);
    }
}
