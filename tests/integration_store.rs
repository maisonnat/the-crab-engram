//! Integration tests: full store flow

use engram_core::ObservationType;
use engram_store::{
    AddObservationParams, SearchOptions, SqliteStore, Storage, UpdateObservationParams,
};

#[test]
fn full_session_workflow() {
    let store = SqliteStore::in_memory().unwrap();

    // 1. Create session
    let sid = store.create_session("test-project").unwrap();
    assert!(!sid.is_empty());

    // 2. Insert observations
    let id1 = store
        .insert_observation(&AddObservationParams {
            r#type: ObservationType::Bugfix,
            scope: engram_core::Scope::Project,
            title: "Fix N+1 query".into(),
            content: "Used eager loading with JOIN".into(),
            session_id: sid.clone(),
            project: "test-project".into(),
            topic_key: Some("bug/n1-query".into()),
            ..Default::default()
        })
        .unwrap();

    let id2 = store
        .insert_observation(&AddObservationParams {
            r#type: ObservationType::Decision,
            scope: engram_core::Scope::Project,
            title: "Use SQLite".into(),
            content: "For local storage, SQLite with WAL mode is sufficient".into(),
            session_id: sid.clone(),
            project: "test-project".into(),
            topic_key: Some("decision/storage".into()),
            ..Default::default()
        })
        .unwrap();

    // 3. Search finds both
    let results = store
        .search(&SearchOptions {
            query: "SQLite".into(),
            project: Some("test-project".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.title.contains("SQLite")));

    // 4. Get observation increments access count
    let obs = store.get_observation(id1).unwrap().unwrap();
    assert_eq!(obs.access_count, 1);

    // 5. Update observation
    store
        .update_observation(
            id1,
            &UpdateObservationParams {
                pinned: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    let obs = store.peek_observation(id1).unwrap().unwrap();
    assert!(obs.pinned);

    // 6. Get session context
    let ctx = store.get_session_context("test-project", 10).unwrap();
    assert_eq!(ctx.observations.len(), 2);

    // 7. Get timeline
    let timeline = store.get_timeline(id1, 5).unwrap();
    assert!(!timeline.is_empty());

    // 8. Get stats
    let stats = store.get_stats("test-project").unwrap();
    assert_eq!(stats.total_observations, 2);
    assert_eq!(stats.total_sessions, 1);
    assert_eq!(stats.by_type.get("bugfix"), Some(&1));
    assert_eq!(stats.by_type.get("decision"), Some(&1));

    // 9. Export
    let data = store.export(None).unwrap();
    assert_eq!(data.observations.len(), 2);
    assert_eq!(data.sessions.len(), 1);

    // 10. Import into fresh store
    let store2 = SqliteStore::in_memory().unwrap();
    let result = store2.import(&data).unwrap();
    assert_eq!(result.observations_imported, 2);

    let stats2 = store2.get_stats("test-project").unwrap();
    assert_eq!(stats2.total_observations, 2);

    // 11. End session
    store
        .end_session(&sid, Some("completed integration test"))
        .unwrap();
    let session = store.get_session(&sid).unwrap().unwrap();
    assert!(!session.is_active());
    assert_eq!(
        session.summary.as_deref(),
        Some("completed integration test")
    );
}

#[test]
fn dedup_within_window() {
    let store = SqliteStore::in_memory().unwrap();
    let sid = store.create_session("test").unwrap();

    let params = AddObservationParams {
        r#type: ObservationType::Manual,
        scope: engram_core::Scope::Project,
        title: "Same title".into(),
        content: "Same content".into(),
        session_id: sid,
        project: "test".into(),
        ..Default::default()
    };

    store.insert_observation(&params).unwrap();
    let result = store.insert_observation(&params);
    assert!(result.is_err());
}

#[test]
fn soft_delete_preserves_data() {
    let store = SqliteStore::in_memory().unwrap();
    let sid = store.create_session("test").unwrap();

    let id = store
        .insert_observation(&AddObservationParams {
            r#type: ObservationType::Bugfix,
            scope: engram_core::Scope::Project,
            title: "Bug".into(),
            content: "Fix".into(),
            session_id: sid,
            project: "test".into(),
            ..Default::default()
        })
        .unwrap();

    // Soft delete
    store.delete_observation(id, false).unwrap();

    // Can still peek (not in search by default)
    let obs = store.peek_observation(id).unwrap().unwrap();
    assert_eq!(obs.lifecycle_state, engram_core::LifecycleState::Deleted);

    // Not in default search
    let results = store
        .search(&SearchOptions {
            query: "Bug".into(),
            ..Default::default()
        })
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn prompts_workflow() {
    let store = SqliteStore::in_memory().unwrap();
    let sid = store.create_session("test").unwrap();

    store
        .save_prompt(&engram_store::AddPromptParams {
            session_id: sid.clone(),
            project: "test".into(),
            content: "How do I fix the auth bug?".into(),
        })
        .unwrap();

    store
        .save_prompt(&engram_store::AddPromptParams {
            session_id: sid.clone(),
            project: "test".into(),
            content: "What's the best pattern for error handling?".into(),
        })
        .unwrap();

    let prompts = store.get_prompts(&sid).unwrap();
    assert_eq!(prompts.len(), 2);
    assert!(prompts[0].contains("auth bug"));
}

#[test]
fn graph_edges_temporal() {
    let store = SqliteStore::in_memory().unwrap();
    let sid = store.create_session("test").unwrap();

    let id1 = store
        .insert_observation(&AddObservationParams {
            r#type: ObservationType::Bugfix,
            scope: engram_core::Scope::Project,
            title: "Auth bug".into(),
            content: "Fix".into(),
            session_id: sid.clone(),
            project: "test".into(),
            ..Default::default()
        })
        .unwrap();

    let id2 = store
        .insert_observation(&AddObservationParams {
            r#type: ObservationType::Decision,
            scope: engram_core::Scope::Project,
            title: "Use JWT".into(),
            content: "Decision".into(),
            session_id: sid,
            project: "test".into(),
            ..Default::default()
        })
        .unwrap();

    // Add edge
    let edge_id = store
        .add_edge(&engram_store::AddEdgeParams {
            source_id: id1,
            target_id: id2,
            relation: engram_core::RelationType::CausedBy,
            weight: 1.0,
            auto_detected: false,
        })
        .unwrap();
    assert!(edge_id > 0);

    // Get edges (only active)
    let edges = store.get_edges(id1).unwrap();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].is_active());

    // Add same relation again — auto-closes previous
    store
        .add_edge(&engram_store::AddEdgeParams {
            source_id: id1,
            target_id: id2,
            relation: engram_core::RelationType::CausedBy,
            weight: 0.9,
            auto_detected: true,
        })
        .unwrap();

    // Only 1 active edge
    let edges = store.get_edges(id1).unwrap();
    assert_eq!(edges.len(), 1);
    assert!((edges[0].weight - 0.9).abs() < f64::EPSILON);
}

#[test]
fn search_with_type_filter() {
    let store = SqliteStore::in_memory().unwrap();
    let sid = store.create_session("test").unwrap();

    store
        .insert_observation(&AddObservationParams {
            r#type: ObservationType::Bugfix,
            scope: engram_core::Scope::Project,
            title: "Auth issue".into(),
            content: "JWT token expired".into(),
            session_id: sid.clone(),
            project: "test".into(),
            ..Default::default()
        })
        .unwrap();

    store
        .insert_observation(&AddObservationParams {
            r#type: ObservationType::Decision,
            scope: engram_core::Scope::Project,
            title: "Auth design".into(),
            content: "Use JWT for auth".into(),
            session_id: sid,
            project: "test".into(),
            ..Default::default()
        })
        .unwrap();

    // Search with type filter
    let results = store
        .search(&SearchOptions {
            query: "JWT".into(),
            r#type: Some(ObservationType::Bugfix),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].r#type, ObservationType::Bugfix);
}
