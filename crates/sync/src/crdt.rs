use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use engram_store::Storage;

/// CRDT device state — persisted per device.
/// Uses per-column Lamport clocks for field-level conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtState {
    pub device_id: String,
    /// Per-column Lamport clock for fine-grained CRDT resolution.
    /// Maps column name → last known Lamport timestamp.
    pub column_clocks: HashMap<String, u64>,
    /// Legacy scalar clock, kept for backward compatibility.
    pub vector_clock: u64,
    pub last_sync: Option<DateTime<Utc>>,
}

impl Default for CrdtState {
    fn default() -> Self {
        Self::new()
    }
}

impl CrdtState {
    pub fn new() -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            column_clocks: HashMap::new(),
            vector_clock: 0,
            last_sync: None,
        }
    }

    /// Get the Lamport clock for a specific column.
    pub fn get_clock(&self, column: &str) -> u64 {
        self.column_clocks.get(column).copied().unwrap_or(0)
    }

    /// Increment the Lamport clock for a specific column.
    /// Returns the new clock value.
    pub fn increment_clock(&mut self, column: &str) -> u64 {
        let new_val = self.column_clocks.get(column).copied().unwrap_or(0) + 1;
        self.column_clocks.insert(column.to_string(), new_val);
        // Also increment legacy scalar clock for backward compat
        self.vector_clock += 1;
        new_val
    }

    /// Merge a remote column clock using max (Lamport clock rule).
    /// Returns true if the remote clock was newer (merge happened).
    pub fn merge_column_clock(&mut self, column: &str, remote_clock: u64) -> bool {
        let local = self.column_clocks.get(column).copied().unwrap_or(0);
        if remote_clock > local {
            self.column_clocks.insert(column.to_string(), remote_clock);
            true
        } else {
            false
        }
    }

    /// Legacy increment — increments scalar clock only.
    pub fn increment(&mut self) {
        self.vector_clock += 1;
    }

    pub fn update_sync_time(&mut self) {
        self.last_sync = Some(Utc::now());
    }
}

/// A delta entry for CRDT sync — now with field-level granularity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDelta {
    pub device_id: String,
    pub vector_clock: u64,
    pub observation_id: i64,
    /// The specific column that changed (field-level granularity).
    pub column: String,
    /// Lamport clock for this specific column at time of change.
    pub column_clock: u64,
    pub operation: SyncOperation,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncOperation {
    Insert,
    Update,
    Delete,
}

/// LWW (Last-Writer-Wins) conflict resolution.
pub fn resolve_conflict(
    local_timestamp: DateTime<Utc>,
    remote_timestamp: DateTime<Utc>,
) -> ConflictResolution {
    if remote_timestamp > local_timestamp {
        ConflictResolution::UseRemote
    } else {
        ConflictResolution::UseLocal
    }
}

/// Per-column LWW conflict resolution using Lamport clocks.
/// Returns the winning clock value and which side won.
pub fn resolve_column_conflict(local_clock: u64, remote_clock: u64) -> ConflictResolution {
    if remote_clock > local_clock {
        ConflictResolution::UseRemote
    } else {
        ConflictResolution::UseLocal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    UseLocal,
    UseRemote,
}

/// Sync status summary.
#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub device_id: String,
    pub vector_clock: u64,
    pub last_sync: Option<DateTime<Utc>>,
    pub pending_deltas: usize,
}

/// Get sync status from a store.
pub fn get_sync_status(_store: &dyn Storage, state: &CrdtState) -> SyncStatus {
    SyncStatus {
        device_id: state.device_id.clone(),
        vector_clock: state.vector_clock,
        last_sync: state.last_sync,
        pending_deltas: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crdt_state_new() {
        let state = CrdtState::new();
        assert!(!state.device_id.is_empty());
        assert_eq!(state.vector_clock, 0);
        assert!(state.last_sync.is_none());
        assert!(state.column_clocks.is_empty());
    }

    #[test]
    fn crdt_increment() {
        let mut state = CrdtState::new();
        state.increment();
        assert_eq!(state.vector_clock, 1);
        state.increment();
        assert_eq!(state.vector_clock, 2);
    }

    #[test]
    fn column_clock_increment() {
        let mut state = CrdtState::new();
        assert_eq!(state.get_clock("title"), 0);
        let v1 = state.increment_clock("title");
        assert_eq!(v1, 1);
        assert_eq!(state.get_clock("title"), 1);
        let v2 = state.increment_clock("title");
        assert_eq!(v2, 2);
        // Also bumps legacy clock
        assert_eq!(state.vector_clock, 2);
    }

    #[test]
    fn column_clock_independent() {
        let mut state = CrdtState::new();
        state.increment_clock("title");
        state.increment_clock("content");
        state.increment_clock("title");
        assert_eq!(state.get_clock("title"), 2);
        assert_eq!(state.get_clock("content"), 1);
        assert_eq!(state.get_clock("unknown"), 0);
    }

    #[test]
    fn merge_column_clock() {
        let mut state = CrdtState::new();
        state.increment_clock("title");
        assert_eq!(state.get_clock("title"), 1);
        // Remote is newer
        assert!(state.merge_column_clock("title", 5));
        assert_eq!(state.get_clock("title"), 5);
        // Remote is older
        assert!(!state.merge_column_clock("title", 3));
        assert_eq!(state.get_clock("title"), 5);
    }

    #[test]
    fn lww_remote_wins() {
        let local = Utc::now();
        let remote = local + chrono::Duration::seconds(1);
        assert_eq!(
            resolve_conflict(local, remote),
            ConflictResolution::UseRemote
        );
    }

    #[test]
    fn lww_local_wins() {
        let local = Utc::now();
        let remote = local - chrono::Duration::seconds(1);
        assert_eq!(
            resolve_conflict(local, remote),
            ConflictResolution::UseLocal
        );
    }

    #[test]
    fn lww_equal_uses_local() {
        let time = Utc::now();
        assert_eq!(resolve_conflict(time, time), ConflictResolution::UseLocal);
    }

    #[test]
    fn column_conflict_resolution() {
        assert_eq!(
            resolve_column_conflict(3, 5),
            ConflictResolution::UseRemote
        );
        assert_eq!(
            resolve_column_conflict(5, 3),
            ConflictResolution::UseLocal
        );
        assert_eq!(
            resolve_column_conflict(5, 5),
            ConflictResolution::UseLocal
        );
    }

    #[test]
    fn sync_status_serializable() {
        let state = CrdtState::new();
        let status = SyncStatus {
            device_id: state.device_id.clone(),
            vector_clock: state.vector_clock,
            last_sync: state.last_sync,
            pending_deltas: 0,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("device_id"));
    }
}
