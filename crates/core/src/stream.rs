use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Memory events for real-time streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryEvent {
    /// Relevant memories found for current file
    RelevantFileContext {
        file_path: String,
        observation_ids: Vec<i64>,
    },
    /// Anti-pattern detected in current work
    AntiPatternWarning {
        pattern_description: String,
        suggestion: String,
    },
    /// Deja-vu: current task matches a previous solution
    DejaVu {
        current_task: String,
        previous_observation_id: i64,
        similarity: f64,
    },
    /// Knowledge capsule updated
    KnowledgeUpdated { topic: String, changes: String },
    /// Spaced repetition review due
    ReviewDue {
        observation_id: i64,
        interval_days: f64,
    },
    /// Entities extracted from event text
    EntityExtracted {
        entities: Vec<ExtractedEntity>,
        topic_entropy: std::collections::HashMap<String, f64>,
    },
}

/// A single extracted entity from event text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub confidence: f64,
}

impl std::fmt::Display for MemoryEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RelevantFileContext {
                file_path,
                observation_ids,
            } => {
                write!(f, "📁 {} has {} memories", file_path, observation_ids.len())
            }
            Self::AntiPatternWarning {
                pattern_description,
                ..
            } => {
                write!(f, "⚠️ Anti-pattern: {pattern_description}")
            }
            Self::DejaVu { similarity, .. } => {
                write!(f, "🔄 Déjà vu! ({:.0}% similarity)", similarity * 100.0)
            }
            Self::KnowledgeUpdated { topic, .. } => {
                write!(f, "📌 Knowledge updated: {topic}")
            }
            Self::ReviewDue { interval_days, .. } => {
                write!(f, "🔄 Review due ({interval_days:.0} days)")
            }
            Self::EntityExtracted { entities, .. } => {
                write!(f, "🔍 Extracted {} entity(ies)", entities.len())
            }
        }
    }
}

/// Throttle controller for event delivery.
#[derive(Debug, Clone)]
pub struct EventThrottle {
    min_interval_secs: u64,
    last_event_time: Option<std::time::Instant>,
}

impl EventThrottle {
    pub fn new(min_interval_secs: u64) -> Self {
        Self {
            min_interval_secs,
            last_event_time: None,
        }
    }

    /// Check if an event should be delivered (not throttled).
    pub fn should_deliver(&mut self) -> bool {
        let now = std::time::Instant::now();
        match self.last_event_time {
            None => {
                self.last_event_time = Some(now);
                true
            }
            Some(last) => {
                let elapsed = now.duration_since(last).as_secs();
                if elapsed >= self.min_interval_secs {
                    self.last_event_time = Some(now);
                    true
                } else {
                    false
                }
            }
        }
    }
}

/// Millisecond-granularity throttle for MCP notifications.
///
/// Enforces a minimum interval between deliveries and uses content hashing
/// to prevent duplicate notifications (anti-spam).
#[derive(Debug)]
pub struct NotificationThrottle {
    min_interval_ms: u64,
    last_sent_time: Option<std::time::Instant>,
    recent_hashes: Vec<String>,
    hash_window_size: usize,
}

impl NotificationThrottle {
    /// Create a new notification throttle.
    ///
    /// `min_interval_ms`: minimum milliseconds between notifications (e.g., 25)
    /// `hash_window_size`: number of recent content hashes to remember for dedup
    pub fn new(min_interval_ms: u64, hash_window_size: usize) -> Self {
        Self {
            min_interval_ms,
            last_sent_time: None,
            recent_hashes: Vec::with_capacity(hash_window_size),
            hash_window_size,
        }
    }

    /// Compute a content hash from a MemoryEvent for dedup.
    pub fn content_hash(event: &MemoryEvent) -> String {
        let json = serde_json::to_string(event).unwrap_or_default();
        let hash = Sha256::digest(json.as_bytes());
        format!("{hash:x}")
    }

    /// Check if an event should be delivered.
    ///
    /// Returns false if:
    /// - The interval since the last delivery hasn't elapsed
    /// - The content hash matches a recently delivered event (spam)
    pub fn should_send(&mut self, event: &MemoryEvent) -> bool {
        let hash = Self::content_hash(event);

        // Anti-spam: check if same content was recently sent
        if self.recent_hashes.contains(&hash) {
            return false;
        }

        // Throttle: check minimum interval
        let now = std::time::Instant::now();
        if let Some(last) = self.last_sent_time {
            let elapsed_ms = now.duration_since(last).as_millis() as u64;
            if elapsed_ms < self.min_interval_ms {
                return false;
            }
        }

        // Allow delivery
        self.last_sent_time = Some(now);
        self.recent_hashes.push(hash);
        if self.recent_hashes.len() > self.hash_window_size {
            self.recent_hashes.remove(0);
        }

        true
    }

    /// Reset the throttle state (useful after reconnection).
    pub fn reset(&mut self) {
        self.last_sent_time = None;
        self.recent_hashes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_event_display() {
        let event = MemoryEvent::RelevantFileContext {
            file_path: "src/auth.rs".into(),
            observation_ids: vec![1, 2, 3],
        };
        assert!(format!("{event}").contains("3 memories"));
    }

    #[test]
    fn deja_vu_display() {
        let event = MemoryEvent::DejaVu {
            current_task: "fix auth".into(),
            previous_observation_id: 42,
            similarity: 0.92,
        };
        let display = format!("{event}");
        assert!(display.contains("92%"));
    }

    #[test]
    fn anti_pattern_display() {
        let event = MemoryEvent::AntiPatternWarning {
            pattern_description: "Recurring auth bug".into(),
            suggestion: "Check token expiry".into(),
        };
        assert!(format!("{event}").contains("Anti-pattern"));
    }

    #[test]
    fn throttle_allows_first_event() {
        let mut throttle = EventThrottle::new(30);
        assert!(throttle.should_deliver());
    }

    #[test]
    fn throttle_blocks_rapid_events() {
        let mut throttle = EventThrottle::new(30);
        assert!(throttle.should_deliver());
        assert!(!throttle.should_deliver()); // Second event immediately = blocked
    }

    #[test]
    fn entity_extracted_display() {
        let mut entities = Vec::new();
        entities.push(ExtractedEntity {
            name: "auth.rs".into(),
            entity_type: "file".into(),
            confidence: 0.9,
        });
        let event = MemoryEvent::EntityExtracted {
            entities,
            topic_entropy: std::collections::HashMap::new(),
        };
        assert!(format!("{event}").contains("1 entity"));
    }

    #[test]
    fn entity_extracted_with_entropy() {
        let entities = vec![
            ExtractedEntity {
                name: "TokenValidator".into(),
                entity_type: "class".into(),
                confidence: 0.85,
            },
            ExtractedEntity {
                name: "auth.rs".into(),
                entity_type: "file".into(),
                confidence: 0.9,
            },
        ];
        let mut entropy = std::collections::HashMap::new();
        entropy.insert("auth".to_string(), 1.5);
        entropy.insert("validation".to_string(), 0.8);

        let event = MemoryEvent::EntityExtracted {
            entities,
            topic_entropy: entropy,
        };
        let display = format!("{event}");
        assert!(display.contains("2 entity"));
    }

    #[test]
    fn notification_throttle_allows_first() {
        let mut throttle = NotificationThrottle::new(25, 5);
        let event = MemoryEvent::RelevantFileContext {
            file_path: "src/main.rs".into(),
            observation_ids: vec![1],
        };
        assert!(throttle.should_send(&event));
    }

    #[test]
    fn notification_throttle_blocks_duplicate() {
        let mut throttle = NotificationThrottle::new(0, 5); // no interval
        let event = MemoryEvent::RelevantFileContext {
            file_path: "src/main.rs".into(),
            observation_ids: vec![1],
        };
        assert!(throttle.should_send(&event)); // first delivery
        assert!(!throttle.should_send(&event)); // duplicate blocked
    }

    #[test]
    fn notification_throttle_different_events_allowed() {
        let mut throttle = NotificationThrottle::new(0, 5);
        let event1 = MemoryEvent::RelevantFileContext {
            file_path: "src/main.rs".into(),
            observation_ids: vec![1],
        };
        let event2 = MemoryEvent::AntiPatternWarning {
            pattern_description: "recurring bug".into(),
            suggestion: "check auth".into(),
        };
        assert!(throttle.should_send(&event1));
        assert!(throttle.should_send(&event2)); // different content, allowed
    }

    #[test]
    fn notification_throttle_hash_window_evicts() {
        let mut throttle = NotificationThrottle::new(0, 2); // window of 2
        let e1 = MemoryEvent::RelevantFileContext {
            file_path: "a.rs".into(),
            observation_ids: vec![1],
        };
        let e2 = MemoryEvent::RelevantFileContext {
            file_path: "b.rs".into(),
            observation_ids: vec![2],
        };
        let e3 = MemoryEvent::RelevantFileContext {
            file_path: "c.rs".into(),
            observation_ids: vec![3],
        };
        assert!(throttle.should_send(&e1));
        assert!(throttle.should_send(&e2));
        assert!(throttle.should_send(&e3)); // e1 evicted, e3 is new
        // But e1 is now evicted from window, so it can be sent again
        assert!(throttle.should_send(&e1));
    }

    #[test]
    fn notification_throttle_reset() {
        let mut throttle = NotificationThrottle::new(0, 5);
        let event = MemoryEvent::RelevantFileContext {
            file_path: "src/main.rs".into(),
            observation_ids: vec![1],
        };
        assert!(throttle.should_send(&event));
        assert!(!throttle.should_send(&event)); // duplicate
        throttle.reset();
        assert!(throttle.should_send(&event)); // after reset, allowed again
    }
}
