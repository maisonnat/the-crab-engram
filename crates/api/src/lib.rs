use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use engram_core::{ObservationType, Scope};
use engram_learn::{AntiPatternDetector, ConsolidationEngine, SmartInjector};
use engram_store::{AddObservationParams, SearchOptions, Storage, UpdateObservationParams};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LearnTickStatus {
    pub entities_linked: usize,
    pub capsules_upserted: usize,
    pub reviews_upserted: usize,
    pub anti_patterns_found: usize,
    pub snapshots_written: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnDaemonStatus {
    pub enabled: bool,
    pub project: String,
    pub interval_seconds: u64,
    pub ticks_run: u64,
    pub last_started_at: Option<String>,
    pub last_completed_at: Option<String>,
    pub last_error: Option<String>,
    pub last_tick: Option<LearnTickStatus>,
}

impl LearnDaemonStatus {
    pub fn disabled(project: String) -> Self {
        Self {
            enabled: false,
            project,
            interval_seconds: 0,
            ticks_run: 0,
            last_started_at: None,
            last_completed_at: None,
            last_error: None,
            last_tick: None,
        }
    }

    pub fn enabled(project: String, interval_seconds: u64) -> Self {
        Self {
            enabled: true,
            project,
            interval_seconds,
            ticks_run: 0,
            last_started_at: None,
            last_completed_at: None,
            last_error: None,
            last_tick: None,
        }
    }
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn Storage>,
    pub project: String,
    pub learn_status: Option<Arc<Mutex<LearnDaemonStatus>>>,
}

/// Create the API router.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/learn/status", get(learn_status))
        .route(
            "/observations",
            get(search_observations).post(create_observation),
        )
        .route(
            "/observations/{id}",
            get(get_observation)
                .put(update_observation)
                .delete(delete_observation),
        )
        .route("/search", post(search))
        .route("/stats", get(stats))
        .route("/sessions", post(create_session))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/end", post(end_session))
        .route("/context", get(context))
        .route("/export", get(export))
        .route("/import", post(import))
        .route("/capsules", get(list_capsules))
        .route("/capsules/{topic}", get(get_capsule))
        .route("/consolidate", post(consolidate))
        .route("/graph/{id}", get(graph_edges))
        .route("/inject", post(inject))
        .route("/antipatterns", get(antipatterns))
        // Hermes memory provider endpoints
        .route("/sync", post(sync_turn))
        .route("/remember", post(remember))
        .route("/recall", post(recall))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// ── Request/Response types ────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub r#type: Option<String>,
    pub project: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct CreateObservationRequest {
    pub title: String,
    pub content: String,
    pub r#type: Option<String>,
    pub scope: Option<String>,
    pub session_id: String,
    pub project: Option<String>,
    pub topic_key: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateObservationRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub pinned: Option<bool>,
    pub topic_key: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub project: Option<String>,
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

// ── Handlers ──────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn learn_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = match &state.learn_status {
        Some(status) => status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| LearnDaemonStatus::disabled(state.project.clone())),
        None => LearnDaemonStatus::disabled(state.project.clone()),
    };

    Json(status)
}

#[derive(Deserialize)]
pub struct EndSessionRequest {
    pub summary: Option<String>,
}

async fn end_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<EndSessionRequest>,
) -> impl IntoResponse {
    match state.store.end_session(&id, req.summary.as_deref()) {
        Ok(()) => Json(serde_json::json!({
            "status": "ended",
            "session_id": id
        }))
        .into_response(),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("not found") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiError {
                        error: "session not found".into(),
                    }),
                )
                    .into_response()
            } else {
                ApiError { error: msg }.into_response()
            }
        }
    }
}

async fn search_observations(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let opts = SearchOptions {
        query: query.query,
        project: query.project.or(Some(state.project.clone())),
        r#type: query.r#type.and_then(|t| t.parse().ok()),
        limit: query.limit,
        ..Default::default()
    };

    match state.store.search(&opts) {
        Ok(results) => Json(results).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn create_observation(
    State(state): State<AppState>,
    Json(req): Json<CreateObservationRequest>,
) -> impl IntoResponse {
    let obs_type: ObservationType = match req.r#type.unwrap_or("manual".into()).parse() {
        Ok(t) => t,
        Err(e) => {
            return ApiError {
                error: e.to_string(),
            }
            .into_response();
        }
    };
    let scope: Scope = match req.scope.as_deref() {
        Some("personal") => Scope::Personal,
        _ => Scope::Project,
    };

    let params = AddObservationParams {
        r#type: obs_type,
        scope,
        title: req.title,
        content: req.content,
        session_id: req.session_id,
        project: req.project.unwrap_or_else(|| state.project.clone()),
        topic_key: req.topic_key,
        ..Default::default()
    };

    match state.store.insert_observation(&params) {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn get_observation(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    match state.store.get_observation(id) {
        Ok(Some(obs)) => Json(obs).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not found".into(),
            }),
        )
            .into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn update_observation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateObservationRequest>,
) -> impl IntoResponse {
    let params = UpdateObservationParams {
        title: req.title,
        content: req.content,
        pinned: req.pinned,
        topic_key: req.topic_key,
        ..Default::default()
    };

    match state.store.update_observation(id, &params) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn delete_observation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.store.delete_observation(id, false) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn search(
    State(state): State<AppState>,
    Json(query): Json<SearchQuery>,
) -> impl IntoResponse {
    search_observations(State(state), Query(query))
        .await
        .into_response()
}

async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.get_stats(&state.project) {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let project = req.project.unwrap_or_else(|| state.project.clone());
    match state.store.create_session(&project) {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "session_id": id })),
        )
            .into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn get_session(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.store.get_session(&id) {
        Ok(Some(session)) => Json(session).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not found".into(),
            }),
        )
            .into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn context(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(10);
    match state.store.get_session_context(&state.project, limit) {
        Ok(ctx) => Json(ctx).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn export(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.export(Some(&state.project)) {
        Ok(data) => Json(data).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn import(
    State(state): State<AppState>,
    Json(data): Json<engram_store::ExportData>,
) -> impl IntoResponse {
    match state.store.import(&data) {
        Ok(result) => Json(result).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

// ── F2+ Routes ─────────────────────────────────────────────────────

async fn list_capsules(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.list_capsules(None) {
        Ok(capsules) => Json(capsules).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn get_capsule(
    State(state): State<AppState>,
    Path(topic): Path<String>,
) -> impl IntoResponse {
    match state.store.get_capsule(&topic, Some(&state.project)) {
        Ok(Some(capsule)) => Json(capsule).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: "not found".into(),
            }),
        )
            .into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn consolidate(State(state): State<AppState>) -> impl IntoResponse {
    let engine = ConsolidationEngine::new(state.store.clone(), None);
    match engine.run_consolidation(&state.project) {
        Ok(result) => Json(serde_json::json!({
            "time_taken_ms": result.time_taken_ms,
            "duplicates_merged": result.duplicates_merged,
            "obsolete_marked": result.obsolete_marked,
            "conflicts_found": result.conflicts_found,
            "patterns_extracted": result.patterns_extracted,
        }))
        .into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn graph_edges(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    match state.store.get_edges(id) {
        Ok(edges) => Json(edges).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

#[derive(Deserialize)]
pub struct InjectRequest {
    pub task: String,
    pub max_tokens: Option<usize>,
}

async fn inject(
    State(state): State<AppState>,
    Json(req): Json<InjectRequest>,
) -> impl IntoResponse {
    let injector = SmartInjector::new(state.store.clone());
    let max_tokens = req.max_tokens.unwrap_or(2000);
    match injector.build_context(&state.project, &req.task, max_tokens) {
        Ok(ctx) => Json(serde_json::json!({
            "relevant_memories": ctx.relevant_memories.len(),
            "warnings": ctx.warnings,
            "knowledge_boundaries": ctx.knowledge_boundaries,
            "total_tokens": ctx.total_tokens,
            "markdown": ctx.to_markdown(),
        }))
        .into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

async fn antipatterns(State(state): State<AppState>) -> impl IntoResponse {
    let detector = AntiPatternDetector::new(state.store.clone(), None);
    match detector.detect_all(&state.project) {
        Ok(patterns) => {
            let items: Vec<_> = patterns
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "type": format!("{:?}", p.r#type),
                        "severity": format!("{}", p.severity),
                        "description": p.description,
                        "suggestion": p.suggestion,
                        "evidence_count": p.evidence.len(),
                    })
                })
                .collect();
            Json(items).into_response()
        }
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_error_is_serializable() {
        let err = ApiError {
            error: "test error".into(),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("test error"));
    }

    #[test]
    fn learn_daemon_status_is_serializable() {
        let status = LearnDaemonStatus::enabled("default".into(), 60);
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("default"));
        assert!(json.contains("interval_seconds"));
    }
}

// ── Hermes Memory Provider Routes ─────────────────────────────────

#[derive(Deserialize)]
pub struct SyncTurnRequest {
    pub user: String,
    pub assistant: String,
    pub session_id: Option<String>,
}

/// POST /sync — Persist a completed conversation turn.
/// Used by Hermes memory provider to sync each turn into the knowledge graph.
async fn sync_turn(
    State(state): State<AppState>,
    Json(req): Json<SyncTurnRequest>,
) -> impl IntoResponse {
    let session_id = req.session_id.unwrap_or_else(|| {
        state
            .store
            .create_session(&state.project)
            .unwrap_or("default".into())
    });

    // Store the user turn
    let user_params = AddObservationParams {
        r#type: ObservationType::Manual,
        scope: Scope::Project,
        title: format!("User: {}", truncate(&req.user, 80)),
        content: req.user,
        session_id: session_id.clone(),
        project: state.project.clone(),
        ..Default::default()
    };

    // Store the assistant turn
    let asst_params = AddObservationParams {
        r#type: ObservationType::Manual,
        scope: Scope::Project,
        title: format!("Assistant: {}", truncate(&req.assistant, 80)),
        content: req.assistant,
        session_id,
        project: state.project.clone(),
        ..Default::default()
    };

    let user_id = state.store.insert_observation(&user_params);
    let asst_id = state.store.insert_observation(&asst_params);

    match (user_id, asst_id) {
        (Ok(uid), Ok(aid)) => Json(serde_json::json!({
            "status": "synced",
            "user_observation_id": uid,
            "assistant_observation_id": aid,
        }))
        .into_response(),
        (Err(e), _) | (_, Err(e)) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

#[derive(Deserialize)]
pub struct RememberRequest {
    pub content: String,
    pub memory_type: Option<String>,
    pub importance: Option<f64>,
    pub topic_key: Option<String>,
}

/// POST /remember — Persist an explicit fact, preference, or decision.
/// Simplified endpoint for Hermes to save knowledge explicitly.
async fn remember(
    State(state): State<AppState>,
    Json(req): Json<RememberRequest>,
) -> impl IntoResponse {
    let obs_type = match req.memory_type.as_deref() {
        Some("preference") => ObservationType::Decision,
        Some("decision") => ObservationType::Decision,
        Some("procedure") => ObservationType::Learning,
        Some("discovery") => ObservationType::Discovery,
        _ => ObservationType::Manual,
    };

    let session_id = state
        .store
        .create_session(&state.project)
        .unwrap_or("memory".into());

    let params = AddObservationParams {
        r#type: obs_type,
        scope: Scope::Project,
        title: truncate(&req.content, 80),
        content: req.content,
        session_id,
        project: state.project.clone(),
        topic_key: req.topic_key,
        ..Default::default()
    };

    match state.store.insert_observation(&params) {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

#[derive(Deserialize)]
pub struct RecallRequest {
    pub query: String,
    pub limit: Option<usize>,
}

/// POST /recall — Semantic search returning formatted context for Hermes.
/// Returns top-k results as a markdown block ready for injection.
async fn recall(
    State(state): State<AppState>,
    Json(req): Json<RecallRequest>,
) -> impl IntoResponse {
    let limit = req.limit.unwrap_or(8);
    let opts = SearchOptions {
        query: req.query,
        project: Some(state.project.clone()),
        limit: Some(limit),
        ..Default::default()
    };

    match state.store.search(&opts) {
        Ok(results) => {
            let mut lines = vec!["## Recalled Context".to_string()];
            for (i, obs) in results.iter().enumerate() {
                lines.push(format!(
                    "{}. **{}** [{}]\n   {}",
                    i + 1,
                    obs.title,
                    obs.r#type,
                    truncate(&obs.content, 200)
                ));
            }
            if results.is_empty() {
                lines.push("_No relevant memories found._".to_string());
            }
            Json(serde_json::json!({
                "count": results.len(),
                "context": lines.join("\n\n"),
            }))
            .into_response()
        }
        Err(e) => ApiError {
            error: e.to_string(),
        }
        .into_response(),
    }
}

/// Truncate a string to max_len characters, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
