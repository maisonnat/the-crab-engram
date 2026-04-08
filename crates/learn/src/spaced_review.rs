/// Review result for spaced repetition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewResult {
    Perfect,   // Instant correct answer
    Good,      // Correct with pause
    Hard,      // Incorrect but partially remembered
    Forgotten, // Not remembered at all
}

/// Spaced repetition entry for a memory.
#[derive(Debug, Clone)]
pub struct SpacedRepetition {
    pub memory_id: i64,
    pub interval_days: f64,
    pub ease_factor: f64, // SM-2 ease factor (default 2.5)
    pub review_count: u32,
    pub last_result: Option<ReviewResult>,
}

impl SpacedRepetition {
    pub fn new(memory_id: i64) -> Self {
        Self {
            memory_id,
            interval_days: 1.0,
            ease_factor: 2.5,
            review_count: 0,
            last_result: None,
        }
    }

    /// Process a review result (SM-2 algorithm).
    pub fn process_review(&mut self, result: ReviewResult) {
        self.review_count += 1;
        self.last_result = Some(result);

        match result {
            ReviewResult::Perfect => {
                self.interval_days *= self.ease_factor;
            }
            ReviewResult::Good => {
                self.interval_days *= 1.2;
            }
            ReviewResult::Hard => {
                self.interval_days = 1.0;
                self.ease_factor = (self.ease_factor - 0.15).max(1.3);
            }
            ReviewResult::Forgotten => {
                self.interval_days = 1.0;
                self.ease_factor = (self.ease_factor - 0.20).max(1.3);
            }
        }
    }
}

/// Bootstrap spaced repetition for a project with no reviews.
/// Selects top observations by access count.
pub fn bootstrap_reviews(
    observation_ids_with_access: &[(i64, i64)],
    count: usize,
) -> Vec<SpacedRepetition> {
    let mut sorted = observation_ids_with_access.to_vec();
    sorted.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by access_count descending

    sorted
        .into_iter()
        .take(count)
        .enumerate()
        .map(|(i, (id, _))| {
            let mut rep = SpacedRepetition::new(id);
            // Distribute intervals based on rank
            rep.interval_days = match i {
                0..=9 => 3.0,   // Top 10: already well-known
                10..=29 => 1.0, // Medium familiarity
                _ => 0.5,       // Less familiar
            };
            rep
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_review_starts_at_1_day() {
        let rep = SpacedRepetition::new(1);
        assert!((rep.interval_days - 1.0).abs() < f64::EPSILON);
        assert!((rep.ease_factor - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn perfect_extends_interval() {
        let mut rep = SpacedRepetition::new(1);
        rep.process_review(ReviewResult::Perfect);
        assert!((rep.interval_days - 2.5).abs() < 1e-10); // 1.0 * 2.5
    }

    #[test]
    fn good_extends_slightly() {
        let mut rep = SpacedRepetition::new(1);
        rep.process_review(ReviewResult::Good);
        assert!((rep.interval_days - 1.2).abs() < 1e-10);
    }

    #[test]
    fn forgotten_resets_interval() {
        let mut rep = SpacedRepetition::new(1);
        rep.interval_days = 10.0;
        rep.process_review(ReviewResult::Forgotten);
        assert!((rep.interval_days - 1.0).abs() < f64::EPSILON);
        assert!((rep.ease_factor - 2.3).abs() < 1e-10); // 2.5 - 0.2
    }

    #[test]
    fn hard_lowers_ease() {
        let mut rep = SpacedRepetition::new(1);
        rep.process_review(ReviewResult::Hard);
        assert!((rep.interval_days - 1.0).abs() < f64::EPSILON);
        assert!((rep.ease_factor - 2.35).abs() < 1e-10); // 2.5 - 0.15
    }

    #[test]
    fn ease_never_below_minimum() {
        let mut rep = SpacedRepetition::new(1);
        for _ in 0..20 {
            rep.process_review(ReviewResult::Forgotten);
        }
        assert!(rep.ease_factor >= 1.3);
    }

    #[test]
    fn bootstrap_distributes_intervals() {
        let observations: Vec<(i64, i64)> = (0..50).map(|i| (i, 50 - i)).collect();
        let reviews = bootstrap_reviews(&observations, 50);
        assert_eq!(reviews.len(), 50);
        // Top entries should have 3-day interval
        assert!((reviews[0].interval_days - 3.0).abs() < f64::EPSILON);
        // Later entries should have shorter intervals
        assert!((reviews[40].interval_days - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn bootstrap_limits_to_count() {
        let observations: Vec<(i64, i64)> = (0..100).map(|i| (i, 100 - i)).collect();
        let reviews = bootstrap_reviews(&observations, 50);
        assert_eq!(reviews.len(), 50);
    }
}
