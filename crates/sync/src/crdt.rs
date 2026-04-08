use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use engram_store::Storage;

/// CRDT device state — persisted per device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtState {
    pub device_id: String,
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
            vector_clock: 0,
            last_sync: None,
        }
    }

    pub fn increment(&mut self) {
        self.vector_clock += 1;
    }

    pub fn update_sync_time(&mut self) {
        self.last_sync = Some(Utc::now());
    }
}

/// A delta entry for CRDT sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDelta {
    pub device_id: String,
    pub vector_clock: u64,
    pub observation_id: i64,
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
