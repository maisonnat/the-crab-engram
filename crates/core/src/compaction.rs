/// Compaction levels — sube de nivel de abstracción.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionLevel {
    Raw,       // Observations crudas
    Fact,      // Hechos comprimidos (Knowledge Capsules)
    Pattern,   // Patrones derivados
    Principle, // Principios abstractos
}

impl std::fmt::Display for CompactionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw => write!(f, "raw"),
            Self::Fact => write!(f, "fact"),
            Self::Pattern => write!(f, "pattern"),
            Self::Principle => write!(f, "principle"),
        }
    }
}

/// Determine compaction level from query context.
pub fn determine_level(query: &str) -> CompactionLevel {
    let lower = query.to_lowercase();

    // Specific question → Fact level
    let fact_signals = [
        "what is", "how to", "config", "setting", "specific", "value",
    ];
    if fact_signals.iter().any(|s| lower.contains(s)) {
        return CompactionLevel::Fact;
    }

    // Trend question → Pattern level
    let pattern_signals = [
        "how do we",
        "tend to",
        "usually",
        "pattern",
        "approach",
        "style",
    ];
    if pattern_signals.iter().any(|s| lower.contains(s)) {
        return CompactionLevel::Pattern;
    }

    // Big picture → Principle level
    let principle_signals = [
        "what kind",
        "type of project",
        "overall",
        "philosophy",
        "values",
        "principle",
    ];
    if principle_signals.iter().any(|s| lower.contains(s)) {
        return CompactionLevel::Principle;
    }

    // Default to Fact
    CompactionLevel::Fact
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specific_query_is_fact() {
        assert_eq!(
            determine_level("what is the auth config"),
            CompactionLevel::Fact
        );
        assert_eq!(determine_level("how to set up JWT"), CompactionLevel::Fact);
    }

    #[test]
    fn trend_query_is_pattern() {
        assert_eq!(
            determine_level("how do we usually handle errors"),
            CompactionLevel::Pattern
        );
        assert_eq!(
            determine_level("what's our pattern for testing"),
            CompactionLevel::Pattern
        );
    }

    #[test]
    fn big_picture_is_principle() {
        assert_eq!(
            determine_level("this project's philosophy"),
            CompactionLevel::Principle
        );
        assert_eq!(
            determine_level("overall guiding principles"),
            CompactionLevel::Principle
        );
    }
}
