use std::sync::Arc;

use rmcp::{
    handler::server::ServerHandler,
    model::*,
    service::{Peer, RequestContext, RoleServer},
};
use tokio::sync::mpsc;

use engram_core::{MemoryEvent, NotificationThrottle};
use engram_store::Storage;

/// Configuration for the MCP server.
#[derive(Debug, Clone)]
pub struct EngramConfig {
    pub project: String,
    pub profile: ToolProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolProfile {
    Agent,
    Admin,
    All,
}

impl Default for EngramConfig {
    fn default() -> Self {
        Self {
            project: "default".into(),
            profile: ToolProfile::Agent,
        }
    }
}

/// The Engram MCP server.
#[derive(Clone)]
pub struct EngramServer {
    pub store: Arc<dyn Storage>,
    pub config: EngramConfig,
    /// Peer reference captured during MCP initialization (for sending notifications).
    peer: Arc<std::sync::Mutex<Option<Peer<RoleServer>>>>,
    /// Channel sender for stream events.
    event_tx: Arc<std::sync::Mutex<Option<mpsc::Sender<MemoryEvent>>>>,
}

impl EngramServer {
    pub fn new(store: Arc<dyn Storage>, config: EngramConfig) -> Self {
        Self {
            store,
            config,
            peer: Arc::new(std::sync::Mutex::new(None)),
            event_tx: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Start the MCP server on stdio transport. Blocks until connection closes.
    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        eprintln!("engram-mcp v2.0.0 — starting stdio transport");
        eprintln!("Project: {}", self.config.project);
        eprintln!("Profile: {:?}", self.config.profile);

        // Start auto-consolidation in background
        self.start_auto_consolidation();

        use rmcp::ServiceExt;
        let transport = rmcp::transport::io::stdio();
        let running = self.serve(transport).await.map_err(|e| anyhow::anyhow!("{e:?}"))?;
        running.waiting().await.map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }

    /// Spawn background task that runs consolidation every 30 minutes.
    fn start_auto_consolidation(&self) {
        let store = self.store.clone();
        let project = self.config.project.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(30 * 60) // 30 minutes
            );
            loop {
                interval.tick().await;
                tracing::info!("Auto-consolidation triggered for project: {project}");
                let engine = engram_learn::ConsolidationEngine::new(store.clone(), None);
                match engine.run_consolidation(&project) {
                    Ok(result) => {
                        if result.duplicates_merged > 0 || result.obsolete_marked > 0 {
                            tracing::info!(
                                "Auto-consolidation complete: {} duplicates, {} obsolete, {} conflicts",
                                result.duplicates_merged,
                                result.obsolete_marked,
                                result.conflicts_found,
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Auto-consolidation failed: {e}");
                    }
                }
            }
        });
    }

    fn is_tool_allowed(&self, name: &str) -> bool {
        match self.config.profile {
            ToolProfile::Agent => !matches!(
                name,
                "mem_delete" | "mem_stats" | "mem_timeline" | "mem_merge_projects"
            ),
            ToolProfile::Admin => matches!(
                name,
                "mem_delete" | "mem_stats" | "mem_timeline" | "mem_merge_projects"
            ),
            ToolProfile::All => true,
        }
    }

    /// Get a sender for stream events. Call after MCP init.
    pub fn event_sender(&self) -> Option<mpsc::Sender<MemoryEvent>> {
        self.event_tx.lock().unwrap().clone()
    }

    /// Send a stream event to the MCP client via notifications/stream/event.
    ///
    /// This is called internally by the background delivery task.
    async fn send_stream_notification(
        peer: &Peer<RoleServer>,
        event: &MemoryEvent,
    ) -> Result<(), String> {
        let params = serde_json::to_value(event)
            .map_err(|e| format!("failed to serialize event: {e}"))?;

        let notification = CustomNotification::new(
            "notifications/stream/event",
            Some(params),
        );

        peer.send_notification(ServerNotification::CustomNotification(notification))
            .await
            .map_err(|e| format!("failed to send notification: {e:?}"))
    }

    /// Start the background event delivery task.
    ///
    /// Listens on the mpsc channel, applies throttling (25ms min interval)
    /// and anti-spam (content hash dedup), then sends MCP notifications.
    fn start_delivery_task(
        peer: Peer<RoleServer>,
        mut rx: mpsc::Receiver<MemoryEvent>,
    ) {
        tokio::spawn(async move {
            let mut throttle = NotificationThrottle::new(25, 20);

            while let Some(event) = rx.recv().await {
                if !throttle.should_send(&event) {
                    tracing::debug!("Event throttled or duplicate: {:?}", event);
                    continue;
                }

                match Self::send_stream_notification(&peer, &event).await {
                    Ok(()) => {
                        tracing::debug!("Sent stream notification: {:?}", event);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to send stream notification: {e}");
                        // Don't break — try to continue receiving
                    }
                }
            }

            tracing::info!("Stream event delivery task ended");
        });
    }
}

impl ServerHandler for EngramServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions(
            "Engram: Persistent memory for AI agents. Use mem_save to store learnings, \
             mem_search to find them, mem_context for session context. \
             Always provide session_id and project for proper scoping. \
             Resources: engram://project/current-context, engram://project/knowledge-capsules, \
             engram://project/anti-patterns",
        )
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        // Store peer reference for sending notifications
        {
            let mut peer_guard = self.peer.lock().unwrap();
            *peer_guard = Some(context.peer.clone());
        }

        // Initialize peer info
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }

        // Start the event delivery channel
        let (tx, rx) = mpsc::channel::<MemoryEvent>(64);
        {
            let mut tx_guard = self.event_tx.lock().unwrap();
            *tx_guard = Some(tx);
        }
        EngramServer::start_delivery_task(context.peer.clone(), rx);

        tracing::info!("MCP server initialized — stream delivery channel active");

        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let all_tools = crate::tools::all_tool_definitions();
        let filtered: Vec<Tool> = all_tools
            .into_iter()
            .filter(|t| self.is_tool_allowed(&t.name))
            .collect();

        Ok(ListToolsResult {
            tools: filtered,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        if !self.is_tool_allowed(&request.name) {
            return Err(ErrorData::invalid_params(
                format!("tool '{}' not allowed in profile", request.name),
                None,
            ));
        }

        crate::tools::dispatch_tool(self, &request.name, request.arguments).await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let project = &self.config.project;
        Ok(ListResourcesResult {
            resources: vec![
                Annotated::new(RawResource {
                    uri: format!("engram://{project}/current-context"),
                    name: "Current Context".into(),
                    title: Some("Session Context".into()),
                    description: Some("Recent observations from current session".into()),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                    icons: None,
                    meta: None,
                }, None),
                Annotated::new(RawResource {
                    uri: format!("engram://{project}/knowledge-capsules"),
                    name: "Knowledge Capsules".into(),
                    title: Some("Capsules".into()),
                    description: Some("Synthesized knowledge by topic".into()),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                    icons: None,
                    meta: None,
                }, None),
                Annotated::new(RawResource {
                    uri: format!("engram://{project}/anti-patterns"),
                    name: "Anti-Patterns".into(),
                    title: Some("Anti-Patterns".into()),
                    description: Some("Active anti-pattern warnings".into()),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                    icons: None,
                    meta: None,
                }, None),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        let project = &self.config.project;
        let uri = &request.uri;

        let content = if uri.ends_with("/current-context") {
            match self.store.get_session_context(project, 10) {
                Ok(ctx) => {
                    let mut md = format!("## Session Context for \"{project}\"\n\n");
                    for obs in &ctx.observations {
                        md.push_str(&format!(
                            "- **#{}** [{}] {} — {}\n",
                            obs.id, obs.r#type, obs.title,
                            obs.content.chars().take(100).collect::<String>()
                        ));
                    }
                    md
                }
                Err(e) => format!("Error: {e}"),
            }
        } else if uri.ends_with("/knowledge-capsules") {
            match self.store.list_capsules(Some(project)) {
                Ok(capsules) => {
                    let mut md = format!("## Knowledge Capsules ({})\n\n", capsules.len());
                    for cap in &capsules {
                        md.push_str(&format!(
                            "- **{}** (confidence: {:.0}%, v{}) — {}\n",
                            cap.topic, cap.confidence * 100.0, cap.version,
                            cap.summary.chars().take(80).collect::<String>()
                        ));
                    }
                    md
                }
                Err(e) => format!("Error: {e}"),
            }
        } else if uri.ends_with("/anti-patterns") {
            let bugfixes = self.store.search(&engram_store::SearchOptions {
                query: String::new(),
                project: Some(project.clone()),
                r#type: Some(engram_core::ObservationType::Bugfix),
                limit: Some(200),
                ..Default::default()
            }).unwrap_or_default();

            let mut file_hits: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for bug in &bugfixes {
                for word in format!("{} {}", bug.title, bug.content).split_whitespace() {
                    if word.contains(".rs") || word.contains(".ts") || word.contains(".go") {
                        *file_hits.entry(word.to_lowercase()).or_insert(0) += 1;
                    }
                }
            }
            let hotspots: Vec<_> = file_hits.into_iter().filter(|(_, c)| *c >= 3).collect();
            if hotspots.is_empty() {
                "✅ No anti-patterns detected.".into()
            } else {
                let mut md = String::from("## ⚠️ Hotspot Files\n\n");
                for (file, count) in &hotspots {
                    md.push_str(&format!("- `{file}`: {count} recurring bugs\n"));
                }
                md
            }
        } else {
            return Err(ErrorData::invalid_params(
                format!("unknown resource: {uri}"),
                None,
            ));
        };

        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(content, uri.clone()),
        ]))
    }
}
