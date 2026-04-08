use engram_core::MemorySalience;

/// Infer emotional salience from observation content using keyword heuristics.
/// MVP approach — future: embedding-based classification.
pub fn infer_salience(content: &str, session_length_minutes: Option<f64>) -> MemorySalience {
    let lower = content.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    let valence = infer_valence(&lower, &words);
    let surprise = infer_surprise(&lower);
    let effort = infer_effort(&lower, session_length_minutes);

    MemorySalience {
        emotional_valence: valence,
        surprise_factor: surprise,
        effort_invested: effort,
    }
}

fn infer_valence(lower: &str, _words: &[&str]) -> f64 {
    // Frustration signals
    let frustration_phrases = ["finally", "hours", "weird", "strange", "ugh", "why does"];
    let frustration_words = ["bug", "error", "fail", "broken", "wrong", "stuck"];

    // Achievement signals
    let achievement_phrases = ["breakthrough", "aha", "figured out"];
    let achievement_words = [
        "elegant", "clean", "fast", "solved", "fixed", "works", "great",
    ];

    let frustration_score = frustration_phrases
        .iter()
        .filter(|p| lower.contains(**p))
        .count() as f64
        + frustration_words
            .iter()
            .filter(|w| lower.contains(**w))
            .count() as f64
            * 0.5;

    let achievement_score = achievement_phrases
        .iter()
        .filter(|p| lower.contains(**p))
        .count() as f64
        + achievement_words
            .iter()
            .filter(|w| lower.contains(**w))
            .count() as f64
            * 0.5;

    let total = frustration_score + achievement_score;
    if total == 0.0 {
        return 0.0;
    }

    // Normalize to -1.0..1.0
    let raw = (achievement_score - frustration_score) / total;
    raw.clamp(-1.0, 1.0)
}

fn infer_surprise(lower: &str) -> f64 {
    let surprise_signals = [
        "unexpected",
        "surprising",
        "didn't expect",
        "weird",
        "strange",
        "odd",
        "never seen",
        "first time",
        "new behavior",
    ];

    let count = surprise_signals
        .iter()
        .filter(|s| lower.contains(**s))
        .count();
    (count as f64 * 0.3).min(1.0)
}

fn infer_effort(lower: &str, session_length_minutes: Option<f64>) -> f64 {
    let effort_signals = [
        "hours",
        "debugging",
        "investigating",
        "tracing",
        "complex",
        "tricky",
        "difficult",
        "challenging",
        "finally",
    ];

    let signal_score = effort_signals
        .iter()
        .filter(|s| lower.contains(**s))
        .count() as f64
        * 0.15;

    let time_score = session_length_minutes
        .map(|m| (m / 240.0).min(1.0)) // 4h = max effort
        .unwrap_or(0.0);

    (signal_score + time_score).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frustration_detected() {
        let s = infer_salience("Finally fixed after hours of debugging", None);
        assert!(s.emotional_valence < 0.0, "should be negative valence");
    }

    #[test]
    fn achievement_detected() {
        let s = infer_salience(
            "Elegant solution, solved the performance issue cleanly",
            None,
        );
        assert!(s.emotional_valence > 0.0, "should be positive valence");
    }

    #[test]
    fn neutral_content() {
        let s = infer_salience("Changed configuration setting", None);
        assert!((s.emotional_valence).abs() < 0.5, "should be near neutral");
    }

    #[test]
    fn surprise_detected() {
        let s = infer_salience("Unexpected behavior in the auth module", None);
        assert!(s.surprise_factor > 0.0, "should detect surprise");
    }

    #[test]
    fn effort_from_time() {
        let s = infer_salience("Fixed the bug", Some(180.0)); // 3 hours
        assert!(s.effort_invested > 0.0, "should detect effort from time");
    }

    #[test]
    fn combined_salience() {
        let s = infer_salience(
            "After hours of debugging, finally found the strange error and elegantly solved it",
            Some(120.0),
        );
        // Has both frustration and achievement → valence should be mixed
        // Has surprise ("strange") and effort ("hours", "debugging")
        assert!(s.surprise_factor > 0.0);
        assert!(s.effort_invested > 0.0);
    }
}
