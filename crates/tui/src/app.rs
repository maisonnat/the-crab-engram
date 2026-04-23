use std::sync::Arc;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
};

use engram_core::Observation;
use engram_store::{SearchOptions, SqliteStore, Storage};

// ─── Theme ─────────────────────────────────────────────────────────────────
// Consistent visual language for The Crab Engram TUI.

/// Centralized color theme — every widget references these instead of raw colors.
struct Theme;

impl Theme {
    // Branding
    const PRIMARY: Color = Color::Rgb(0, 200, 180); // Teal — the crab's soul
    const ACCENT: Color = Color::Rgb(255, 170, 50); // Warm amber
    const BG_HIGHLIGHT: Color = Color::Rgb(30, 40, 50); // Dark blue-gray selection bg

    // Semantic
    const SUCCESS: Color = Color::Rgb(80, 220, 100);
    const WARNING: Color = Color::Rgb(255, 200, 50);
    const ERROR: Color = Color::Rgb(255, 80, 80);
    const INFO: Color = Color::Rgb(100, 180, 255);
    const MUTED: Color = Color::Rgb(100, 100, 120);
    const DIM: Color = Color::Rgb(60, 60, 75);
    const TEXT: Color = Color::Rgb(220, 220, 230);
    const HEADING: Color = Color::Rgb(0, 220, 200);

    fn type_color(t: &engram_core::ObservationType) -> Color {
        match t {
            engram_core::ObservationType::Bugfix => Self::ERROR,
            engram_core::ObservationType::Architecture => Self::INFO,
            engram_core::ObservationType::Discovery => Self::SUCCESS,
            engram_core::ObservationType::Learning => Color::Rgb(200, 130, 255),
            engram_core::ObservationType::Config => Self::WARNING,
            _ => Self::PRIMARY,
        }
    }

    fn type_badge(t: &engram_core::ObservationType) -> &'static str {
        match t {
            engram_core::ObservationType::Bugfix => "🐛",
            engram_core::ObservationType::Architecture => "🏗️",
            engram_core::ObservationType::Discovery => "💡",
            engram_core::ObservationType::Learning => "📖",
            engram_core::ObservationType::Config => "⚙️",
            _ => "📝",
        }
    }

    fn styled_block(title: &str) -> Block<'_> {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::DIM))
            .title(format!(" {title} "))
            .title_style(
                Style::default()
                    .fg(Self::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
    }

    fn highlight() -> Style {
        Style::default()
            .bg(Self::BG_HIGHLIGHT)
            .fg(Self::TEXT)
            .add_modifier(Modifier::BOLD)
    }

    fn label() -> Style {
        Style::default()
            .fg(Self::MUTED)
            .add_modifier(Modifier::BOLD)
    }

    fn value() -> Style {
        Style::default().fg(Self::TEXT)
    }
}

/// Crab ASCII art banner for the Dashboard.
fn crab_banner() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(Span::styled(
            "              🦀",
            Style::default().fg(Theme::ACCENT),
        )),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("╭", Style::default().fg(Theme::PRIMARY)),
            Span::styled(
                "━━━━━━━━━━━━━━━━━━━━━━━━━",
                Style::default().fg(Theme::PRIMARY),
            ),
            Span::styled("╮", Style::default().fg(Theme::PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("│", Style::default().fg(Theme::PRIMARY)),
            Span::styled("  ⣾⠿⡷", Style::default().fg(Theme::ACCENT)),
            Span::styled(
                "  The Crab Engram  ",
                Style::default()
                    .fg(Theme::HEADING)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⢾⠿⣷  ", Style::default().fg(Theme::ACCENT)),
            Span::styled("│", Style::default().fg(Theme::PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("     ", Style::default()),
            Span::styled("╰", Style::default().fg(Theme::PRIMARY)),
            Span::styled(
                "━━━━━━━━━━━━━━━━━━━━━━━━━",
                Style::default().fg(Theme::PRIMARY),
            ),
            Span::styled("╯", Style::default().fg(Theme::PRIMARY)),
        ]),
        Line::from(""),
    ]
}

/// Application state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Dashboard,
    Search,
    Detail,
    Timeline,
    Capsules,
    Boundaries,
}

/// Whether the app is in text-editing mode or normal navigation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

impl AppState {
    /// All tab states in order.
    const TABS: [AppState; 5] = [
        AppState::Dashboard,
        AppState::Search,
        AppState::Timeline,
        AppState::Capsules,
        AppState::Boundaries,
    ];

    /// Index in the tab bar.
    fn tab_index(self) -> usize {
        match self {
            AppState::Dashboard => 0,
            AppState::Search => 1,
            AppState::Timeline => 2,
            AppState::Capsules => 3,
            AppState::Boundaries => 4,
            AppState::Detail => 1, // Detail is a sub-view of Search
        }
    }

    /// Next tab (wraps around).
    fn next(self) -> Self {
        let idx = self.tab_index();
        let next = (idx + 1) % Self::TABS.len();
        Self::TABS[next]
    }

    /// Previous tab (wraps around).
    fn prev(self) -> Self {
        let idx = self.tab_index();
        let prev = if idx == 0 {
            Self::TABS.len() - 1
        } else {
            idx - 1
        };
        Self::TABS[prev]
    }
}

/// TUI application state.
pub struct App {
    pub store: Arc<SqliteStore>,
    pub project: String,
    pub state: AppState,
    pub input_mode: InputMode,
    /// When true, a help overlay is drawn on top of the current view.
    pub show_help: bool,
    pub should_quit: bool,
    pub search_query: String,
    /// The query text at the time of the last search execution.
    /// Used to disambiguate Enter: "search again" vs "open detail".
    pub last_searched_query: String,
    pub search_results: Vec<Observation>,
    pub selected_index: usize,
    pub detail_observation: Option<Observation>,
    /// When viewing a capsule detail, holds the selected capsule.
    pub detail_capsule: Option<engram_core::KnowledgeCapsule>,
    /// Vertical scroll offset for Detail and CapsuleDetail views.
    pub scroll_offset: u16,
    pub stats: Option<engram_store::ProjectStats>,
    pub capsules: Vec<engram_core::KnowledgeCapsule>,
    pub boundaries: Vec<(String, String, String)>,
    /// Timeline loaded with its own data source (not search results).
    pub timeline_observations: Vec<Observation>,
    /// Status message shown in footer (with timestamp for auto-clear).
    pub status_message: Option<(String, std::time::Instant)>,
}

impl App {
    pub fn new(store: Arc<SqliteStore>, project: String) -> Self {
        Self {
            store,
            project,
            state: AppState::Dashboard,
            input_mode: InputMode::Normal,
            show_help: false,
            should_quit: false,
            search_query: String::new(),
            last_searched_query: String::new(),
            search_results: Vec::new(),
            selected_index: 0,
            detail_observation: None,
            detail_capsule: None,
            scroll_offset: 0,
            stats: None,
            capsules: Vec::new(),
            boundaries: Vec::new(),
            timeline_observations: Vec::new(),
            status_message: None,
        }
    }

    /// Refresh all data panels (called on startup).
    pub fn refresh_all(&mut self) {
        self.refresh_stats();
        self.refresh_timeline();
    }

    pub fn refresh_stats(&mut self) {
        self.stats = self.store.get_stats(&self.project).ok();
    }

    pub fn refresh_capsules(&mut self) {
        self.capsules = self.store.list_capsules(None).unwrap_or_default();
    }

    pub fn refresh_boundaries(&mut self) {
        self.boundaries = self.store.get_boundaries().unwrap_or_default();
    }

    /// Load recent observations for the Timeline tab.
    pub fn refresh_timeline(&mut self) {
        if let Ok(ctx) = self.store.get_session_context(&self.project, 20) {
            self.timeline_observations = ctx.observations;
        }
    }

    pub fn search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        let opts = SearchOptions {
            query: self.search_query.clone(),
            project: Some(self.project.clone()),
            limit: Some(50),
            ..Default::default()
        };
        if let Ok(results) = self.store.search(&opts) {
            let count = results.len();
            self.search_results = results;
            self.selected_index = 0;
            self.last_searched_query = self.search_query.clone();
            self.set_status(format!("Found {count} results"));
        }
    }

    /// Whether the search input should capture character input.
    pub fn is_editing(&self) -> bool {
        self.input_mode == InputMode::Editing
    }

    /// Switch to Search tab and enter editing mode (triggered by `/`).
    pub fn start_search(&mut self) {
        self.state = AppState::Search;
        self.input_mode = InputMode::Editing;
        self.search_query.clear();
        self.selected_index = 0;
    }

    /// Exit editing mode back to normal navigation.
    pub fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Set a status message (auto-clears after ~4 seconds).
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    /// Clear status if older than 4 seconds.
    #[allow(clippy::collapsible_if)]
    pub fn tick_status(&mut self) {
        if let Some((_, ts)) = &self.status_message {
            if ts.elapsed().as_secs() > 4 {
                self.status_message = None;
            }
        }
    }

    /// Scroll detail view up by `amount` lines.
    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll detail view down by `amount` lines.
    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    /// Toggle pinned status of the current detail observation.
    pub fn toggle_pin(&mut self) -> Result<(), String> {
        if let Some(obs) = &self.detail_observation {
            let new_pin = !obs.pinned;
            let obs_id = obs.id;
            let params = engram_store::UpdateObservationParams {
                pinned: Some(new_pin),
                ..Default::default()
            };
            self.store
                .update_observation(obs_id, &params)
                .map_err(|e| e.to_string())?;
            // Update local state
            if let Some(obs) = self.detail_observation.as_mut() {
                obs.pinned = new_pin;
            }
            let msg = if new_pin {
                format!("📌 Pinned #{}", obs_id)
            } else {
                format!("Unpinned #{}", obs_id)
            };
            self.set_status(msg);
            Ok(())
        } else {
            Err("No observation selected".into())
        }
    }

    /// Handle Enter key contextually.
    ///
    /// Disambiguation logic:
    /// - If in Search and query changed since last search → execute search
    /// - If in Search and query unchanged (or empty) with results → open detail
    pub fn handle_enter(&mut self) {
        match self.state {
            AppState::Search => {
                if !self.search_query.is_empty() && self.search_query != self.last_searched_query {
                    // Query changed — execute search
                    self.search();
                    self.input_mode = InputMode::Normal;
                } else if !self.search_results.is_empty() {
                    // Query unchanged or empty — open selected detail
                    self.detail_observation = self.search_results.get(self.selected_index).cloned();
                    if self.detail_observation.is_some() {
                        self.input_mode = InputMode::Normal;
                        self.scroll_offset = 0;
                        self.state = AppState::Detail;
                    }
                }
            }
            AppState::Timeline => {
                if let Some(obs) = self.timeline_observations.get(self.selected_index) {
                    self.detail_observation = Some(obs.clone());
                    self.state = AppState::Detail;
                }
            }
            AppState::Capsules => {
                if let Some(cap) = self.capsules.get(self.selected_index) {
                    self.detail_capsule = Some(cap.clone());
                    self.scroll_offset = 0;
                    self.set_status(format!("Viewing capsule: {}", cap.topic));
                }
            }
            _ => {}
        }
    }

    /// Select next item in the current list context.
    pub fn select_next(&mut self) {
        let max = match self.state {
            AppState::Search | AppState::Detail => self.search_results.len(),
            AppState::Timeline => self.timeline_observations.len(),
            AppState::Capsules => self.capsules.len(),
            AppState::Boundaries => self.boundaries.len(),
            AppState::Dashboard => 0,
        };
        if max > 0 && self.selected_index < max - 1 {
            self.selected_index += 1;
        }
    }

    /// Select previous item in the current list context.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Switch to the next tab (wraps around).
    pub fn next_tab(&mut self) {
        if self.state != AppState::Detail {
            let next = self.state.next();
            self.activate_tab(next);
        }
    }

    /// Switch to the previous tab (wraps around).
    pub fn prev_tab(&mut self) {
        if self.state != AppState::Detail {
            let prev = self.state.prev();
            self.activate_tab(prev);
        }
    }

    /// Activate a tab with side effects (data refresh).
    fn activate_tab(&mut self, state: AppState) {
        match state {
            AppState::Timeline => self.refresh_timeline(),
            AppState::Capsules => self.refresh_capsules(),
            AppState::Boundaries => self.refresh_boundaries(),
            _ => {}
        }
        self.selected_index = 0;
        self.input_mode = InputMode::Normal;
        self.state = state;
    }

    pub fn go_back(&mut self) {
        // First: close any inline detail overlay
        if self.detail_capsule.is_some() {
            self.detail_capsule = None;
            self.scroll_offset = 0;
            return;
        }
        if self.detail_observation.is_some() && self.state == AppState::Timeline {
            self.detail_observation = None;
            self.scroll_offset = 0;
            return;
        }

        match self.state {
            AppState::Detail => {
                self.detail_observation = None;
                self.scroll_offset = 0;
                self.state = AppState::Search;
                self.input_mode = InputMode::Normal;
            }
            AppState::Search if self.input_mode == InputMode::Editing => {
                // First Esc: clear query, stop editing
                self.search_query.clear();
                self.input_mode = InputMode::Normal;
            }
            AppState::Search => {
                self.state = AppState::Dashboard;
            }
            _ => {
                self.state = AppState::Dashboard;
            }
        }
    }
}

/// Render the application.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);

    match app.state {
        AppState::Dashboard => draw_dashboard(f, app, chunks[1]),
        AppState::Search => draw_search(f, app, chunks[1]),
        AppState::Detail => draw_detail(f, app, chunks[1]),
        AppState::Timeline => {
            if app.detail_observation.is_some() {
                draw_detail(f, app, chunks[1])
            } else {
                draw_timeline(f, app, chunks[1])
            }
        }
        AppState::Capsules => {
            if app.detail_capsule.is_some() {
                draw_capsule_detail(f, app, chunks[1])
            } else {
                draw_capsules(f, app, chunks[1])
            }
        }
        AppState::Boundaries => draw_boundaries(f, app, chunks[1]),
    }

    draw_footer(f, app, chunks[2]);

    // Help overlay drawn on top of everything
    if app.show_help {
        draw_help(f, f.area());
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        "📊 Dashboard",
        "🔍 Search",
        "📅 Timeline",
        "📦 Capsules",
        "🗺️ Boundaries",
    ];
    let selected = app.state.tab_index();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::DIM))
                .title(format!(" 🦀 {} ", app.project))
                .title_style(
                    Style::default()
                        .fg(Theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .select(selected)
        .style(Style::default().fg(Theme::MUTED))
        .highlight_style(
            Style::default()
                .fg(Theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    // Layout: left = banner + stats, right = recent activity
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // ── Left panel: Crab banner + Stats ──────────────────────────────
    let mut left_lines: Vec<Line> = crab_banner();

    if let Some(stats) = &app.stats {
        left_lines.push(Line::from(vec![
            Span::styled("  📝 Observations  ", Theme::label()),
            Span::styled(
                stats.total_observations.to_string(),
                Style::default()
                    .fg(Theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        left_lines.push(Line::from(vec![
            Span::styled("  📂 Sessions      ", Theme::label()),
            Span::styled(
                stats.total_sessions.to_string(),
                Style::default().fg(Theme::PRIMARY),
            ),
        ]));
        left_lines.push(Line::from(vec![
            Span::styled("  🔗 Edges         ", Theme::label()),
            Span::styled(
                stats.total_edges.to_string(),
                Style::default().fg(Theme::PRIMARY),
            ),
        ]));
        left_lines.push(Line::from(vec![
            Span::styled("  📦 Capsules      ", Theme::label()),
            Span::styled(
                stats.total_capsules.to_string(),
                Style::default().fg(Theme::PRIMARY),
            ),
        ]));
        left_lines.push(Line::raw(""));
        left_lines.push(Line::from(Span::styled(
            "  ── By Type ──────────────",
            Style::default().fg(Theme::DIM),
        )));
        for (t, count) in &stats.by_type {
            let obs_type: engram_core::ObservationType =
                t.parse().unwrap_or(engram_core::ObservationType::Manual);
            left_lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", Theme::type_badge(&obs_type)),
                    Style::default(),
                ),
                Span::styled(format!("{:<12}", format!("{t}:")), Theme::label()),
                Span::styled(
                    count.to_string(),
                    Style::default().fg(Theme::type_color(&obs_type)),
                ),
            ]));
        }
    } else {
        left_lines.push(Line::from(Span::styled(
            "  Loading...",
            Style::default().fg(Theme::MUTED),
        )));
    }

    let stats_widget = Paragraph::new(left_lines)
        .block(Theme::styled_block("📊 Project Overview"))
        .wrap(Wrap { trim: true });
    f.render_widget(stats_widget, chunks[0]);

    // ── Right panel: Recent Activity ─────────────────────────────────
    let mut recent_lines: Vec<Line> = Vec::new();
    for obs in app.timeline_observations.iter().take(15) {
        let tc = Theme::type_color(&obs.r#type);
        let badge = Theme::type_badge(&obs.r#type);
        recent_lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", obs.created_at.format("%H:%M")),
                Style::default().fg(Theme::DIM),
            ),
            Span::styled(format!("{badge} "), Style::default()),
            Span::styled(format!("[{}] ", obs.r#type), Style::default().fg(tc)),
            Span::styled(
                obs.title.chars().take(35).collect::<String>(),
                Theme::value(),
            ),
        ]));
    }
    if recent_lines.is_empty() {
        recent_lines.push(Line::from(Span::styled(
            "  No recent activity yet.",
            Style::default().fg(Theme::MUTED),
        )));
    }

    let recent = Paragraph::new(recent_lines)
        .block(Theme::styled_block("🕐 Recent Activity"))
        .wrap(Wrap { trim: true });
    f.render_widget(recent, chunks[1]);
}

fn draw_search(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search input with cursor indicator
    let input_style = Style::default().fg(Theme::ACCENT);
    let search_input = Paragraph::new(Line::from(vec![
        Span::styled(&app.search_query, Theme::value()),
        Span::styled("▎", input_style),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Theme::DIM))
            .title(" 🔍 Search (type + Enter to search, Enter again to open) ")
            .title_style(
                Style::default()
                    .fg(Theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    f.render_widget(search_input, chunks[0]);

    // Results list
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, obs)| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Theme::PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::value()
            };
            let tc = Theme::type_color(&obs.r#type);
            let badge = Theme::type_badge(&obs.r#type);
            ListItem::new(Line::from(vec![
                Span::styled(format!("{badge} "), Style::default()),
                Span::styled(format!("[{}] ", obs.r#type), Style::default().fg(tc)),
                Span::styled(obs.title.to_string(), style),
                Span::styled(
                    format!(" ({}x)", obs.access_count),
                    Style::default().fg(Theme::DIM),
                ),
                Span::styled(" — ", Style::default().fg(Theme::DIM)),
                Span::styled(
                    obs.content.chars().take(60).collect::<String>(),
                    Style::default().fg(Theme::MUTED),
                ),
            ]))
        })
        .collect();

    let results_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::DIM))
                .title(format!(" Results ({}) ", app.search_results.len()))
                .title_style(
                    Style::default()
                        .fg(Theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_style(Theme::highlight());

    f.render_widget(results_list, chunks[1]);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    if let Some(obs) = &app.detail_observation {
        let tc = Theme::type_color(&obs.r#type);
        let badge = Theme::type_badge(&obs.r#type);
        let pin_indicator = if obs.pinned { " 📌" } else { "" };

        let sep = Line::from(Span::styled(
            "  ─────────────────────────────────",
            Style::default().fg(Theme::DIM),
        ));
        let detail_text = vec![
            Line::from(vec![
                Span::styled(format!("{badge} "), Style::default()),
                Span::styled("ID: ", Theme::label()),
                Span::styled(obs.id.to_string(), Theme::value()),
            ]),
            Line::from(vec![
                Span::styled("  Type: ", Theme::label()),
                Span::styled(format!("{}", obs.r#type), Style::default().fg(tc)),
            ]),
            Line::from(vec![
                Span::styled("  Scope: ", Theme::label()),
                Span::styled(format!("{}", obs.scope), Theme::value()),
            ]),
            Line::from(vec![
                Span::styled("  Title: ", Theme::label()),
                Span::styled(
                    obs.title.as_str(),
                    Style::default()
                        .fg(Theme::HEADING)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Topic: ", Theme::label()),
                Span::styled(obs.topic_key.as_deref().unwrap_or("—"), Theme::value()),
            ]),
            Line::from(vec![
                Span::styled("  Access: ", Theme::label()),
                Span::styled(format!("{}x", obs.access_count), Theme::value()),
            ]),
            Line::from(vec![
                Span::styled("  Pinned: ", Theme::label()),
                Span::styled(
                    if obs.pinned { "yes" } else { "no" },
                    Style::default().fg(if obs.pinned {
                        Theme::ACCENT
                    } else {
                        Theme::MUTED
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Provenance: ", Theme::label()),
                Span::styled(
                    format!(
                        "{:?} ({:.0}%)",
                        obs.provenance_source,
                        obs.provenance_confidence * 100.0
                    ),
                    Theme::value(),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Created: ", Theme::label()),
                Span::styled(
                    obs.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                    Theme::value(),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Updated: ", Theme::label()),
                Span::styled(
                    obs.updated_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                    Theme::value(),
                ),
            ]),
            sep,
            Line::from(Span::styled(
                "  Content:",
                Style::default()
                    .fg(Theme::HEADING)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::styled(&obs.content, Theme::value()),
        ];

        let detail = Paragraph::new(detail_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Theme::DIM))
                    .title(format!(" {badge} Observation #{}{pin_indicator} ", obs.id))
                    .title_style(
                        Style::default()
                            .fg(Theme::PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .scroll((app.scroll_offset, 0))
            .wrap(Wrap { trim: true });

        f.render_widget(detail, area);
    } else {
        let msg = Paragraph::new("No observation selected.\nPress Esc to go back.")
            .block(Theme::styled_block("Detail"));
        f.render_widget(msg, area);
    }
}

fn draw_timeline(f: &mut Frame, app: &App, area: Rect) {
    let observations = &app.timeline_observations;
    let mut items: Vec<ListItem> = Vec::new();

    for (i, obs) in observations.iter().take(30).enumerate() {
        let style = if i == app.selected_index {
            Style::default()
                .fg(Theme::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::value()
        };
        let tc = Theme::type_color(&obs.r#type);
        let badge = Theme::type_badge(&obs.r#type);
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("{} ", obs.created_at.format("%m-%d %H:%M")),
                Style::default().fg(Theme::DIM),
            ),
            Span::styled(format!("{badge} "), Style::default()),
            Span::styled(format!("[{}] ", obs.r#type), Style::default().fg(tc)),
            Span::styled(obs.title.clone(), style),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "No observations yet. Use engram to record some!",
            Style::default().fg(Theme::MUTED),
        ))));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::DIM))
                .title(format!(
                    " 📅 Timeline ({} observations) ",
                    observations.len()
                ))
                .title_style(
                    Style::default()
                        .fg(Theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_style(Theme::highlight());
    f.render_widget(list, area);
}

fn draw_capsules(f: &mut Frame, app: &App, area: Rect) {
    let capsules = &app.capsules;
    let mut items: Vec<ListItem> = Vec::new();

    for (i, cap) in capsules.iter().enumerate() {
        let style = if i == app.selected_index {
            Style::default()
                .fg(Theme::PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::value()
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("📦 {} ", cap.topic), style),
            Span::styled(
                format!("({:.0}%, v{}) ", cap.confidence * 100.0, cap.version),
                Style::default().fg(Theme::SUCCESS),
            ),
            Span::styled(
                cap.summary.chars().take(60).collect::<String>(),
                Style::default().fg(Theme::MUTED),
            ),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "No capsules yet. Run `engram consolidate` to create some.",
            Style::default().fg(Theme::MUTED),
        ))));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::DIM))
                .title(" 📦 Knowledge Capsules ")
                .title_style(
                    Style::default()
                        .fg(Theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_style(Theme::highlight());
    f.render_widget(list, area);
}

fn draw_capsule_detail(f: &mut Frame, app: &App, area: Rect) {
    if let Some(cap) = &app.detail_capsule {
        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled("Topic: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&cap.topic),
            ]),
            Line::from(vec![
                Span::styled(
                    "Confidence: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:.0}%", cap.confidence * 100.0),
                    Style::default().fg(Color::Green),
                ),
                Span::raw("  "),
                Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(cap.version.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Project: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(cap.project.as_deref().unwrap_or("—")),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "Summary:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ];

        // Word-wrap summary manually into lines
        for chunk in cap.summary.split('\n') {
            lines.push(Line::raw(chunk.to_string()));
        }

        if !cap.key_decisions.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Key Decisions:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            for d in &cap.key_decisions {
                lines.push(Line::from(format!("  • {}", d)));
            }
        }

        if !cap.best_practices.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Best Practices:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            for bp in &cap.best_practices {
                lines.push(Line::from(format!("  ✓ {}", bp)));
            }
        }

        if !cap.known_issues.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Known Issues:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            for ki in &cap.known_issues {
                lines.push(Line::from(format!("  ⚠ {}", ki)));
            }
        }

        if !cap.anti_patterns.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Anti-patterns:",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for ap in &cap.anti_patterns {
                lines.push(Line::from(format!("  ✗ {}", ap)));
            }
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Source observations: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                cap.source_observations
                    .iter()
                    .map(|id| format!("#{}", id))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(cap.created_at.format("%Y-%m-%d %H:%M UTC").to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "Last consolidated: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                cap.last_consolidated
                    .format("%Y-%m-%d %H:%M UTC")
                    .to_string(),
            ),
        ]));

        let detail = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Capsule: {} ", cap.topic)),
            )
            .scroll((app.scroll_offset, 0))
            .wrap(Wrap { trim: true });

        f.render_widget(detail, area);
    } else {
        let msg = Paragraph::new("No capsule selected.\nPress Esc to go back.").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Capsule Detail "),
        );
        f.render_widget(msg, area);
    }
}

fn draw_boundaries(f: &mut Frame, app: &App, area: Rect) {
    let boundaries = &app.boundaries;
    let mut items: Vec<ListItem> = Vec::new();

    for (domain, level, evidence) in boundaries {
        let level_color = match level.to_lowercase().as_str() {
            "high" => Theme::ERROR,
            "medium" => Theme::WARNING,
            _ => Theme::SUCCESS,
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("🗺️ {domain}: "),
                Style::default()
                    .fg(Theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{level} "), Style::default().fg(level_color)),
            Span::styled(
                evidence.chars().take(40).collect::<String>(),
                Style::default().fg(Theme::MUTED),
            ),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "No knowledge boundaries defined. Boundaries emerge from usage patterns.",
            Style::default().fg(Theme::MUTED),
        ))));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::DIM))
                .title(" 🗺️ Knowledge Boundaries ")
                .title_style(
                    Style::default()
                        .fg(Theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_style(Theme::highlight());
    f.render_widget(list, area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    // Center the help overlay (60% width, ~55% height)
    let help_area = Rect {
        x: area.width / 5,
        y: area.height / 5,
        width: area.width * 3 / 5,
        height: area.height * 3 / 5,
    };

    // Clear background
    let clear = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear, help_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  🦀 The Crab Engram — Keybindings",
            Style::default()
                .fg(Theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Navigation",
            Style::default()
                .fg(Theme::HEADING)
                .add_modifier(Modifier::BOLD),
        )),
        Line::styled(
            "    [←/→] or [Tab/Shift+Tab]  Switch tabs",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [1-5]                      Jump to tab",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [↑/↓] or [j/k]            Navigate list",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [Esc]                      Back / clear",
            Style::default().fg(Theme::TEXT),
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  Search & Detail",
            Style::default()
                .fg(Theme::HEADING)
                .add_modifier(Modifier::BOLD),
        )),
        Line::styled(
            "    [/]                        Start search (any tab)",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [Enter]                    Execute search → open detail",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [p]                        Toggle pin (in detail)",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [PgUp/PgDn]                Scroll detail",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [Esc]                      Clear query → back",
            Style::default().fg(Theme::TEXT),
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  General",
            Style::default()
                .fg(Theme::HEADING)
                .add_modifier(Modifier::BOLD),
        )),
        Line::styled(
            "    [?]                        Toggle this help",
            Style::default().fg(Theme::TEXT),
        ),
        Line::styled(
            "    [q]                        Quit",
            Style::default().fg(Theme::TEXT),
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to close",
            Style::default().fg(Theme::MUTED),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::PRIMARY))
                .title(" ❓ Help ")
                .title_style(
                    Style::default()
                        .fg(Theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .style(Style::default().fg(Theme::TEXT));
    f.render_widget(help, help_area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let hints = match app.state {
        AppState::Dashboard => " [/] Search  [?] Help  [←→] Switch tab  [q] Quit",
        AppState::Search => {
            if app.is_editing() {
                " [Enter] Search  [Esc] Clear  [q] Quit"
            } else {
                " [Enter] Open  [↑↓] Navigate  [/] Edit  [Esc] Back  [q] Quit"
            }
        }
        AppState::Detail => " [p] Pin  [PgUp/PgDn] Scroll  [Esc] Back  [?] Help  [q] Quit",
        AppState::Timeline => {
            " [Enter] Open  [↑↓] Navigate  [←→] Switch tab  [/] Search  [Esc] Back"
        }
        AppState::Capsules => {
            " [Enter] Open  [↑↓] Navigate  [←→] Switch tab  [/] Search  [Esc] Back"
        }
        AppState::Boundaries => " [←→] Switch tab  [/] Search  [?] Help  [Esc] Back",
    };

    let footer_text = if let Some((msg, _ts)) = &app.status_message {
        Line::from(vec![
            Span::styled(" ℹ ", Style::default().fg(Theme::ACCENT)),
            Span::styled(msg.as_str(), Style::default().fg(Theme::TEXT)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                " Tabs:",
                Style::default()
                    .fg(Theme::MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" [1-5] ", Style::default().fg(Theme::TEXT)),
            Span::styled("│", Style::default().fg(Theme::DIM)),
            Span::styled(hints, Style::default().fg(Theme::MUTED)),
        ])
    };

    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Theme::MUTED))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::DIM)),
        );
    f.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_store::SqliteStore;

    #[test]
    fn app_creation() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let app = App::new(store, "test".into());
        assert_eq!(app.state, AppState::Dashboard);
        assert!(!app.should_quit);
        assert!(app.last_searched_query.is_empty());
    }

    #[test]
    fn app_navigation() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());

        // Tab cycling
        assert_eq!(app.state, AppState::Dashboard);
        app.next_tab();
        assert_eq!(app.state, AppState::Search);
        app.next_tab();
        assert_eq!(app.state, AppState::Timeline);
        app.next_tab();
        assert_eq!(app.state, AppState::Capsules);
        app.next_tab();
        assert_eq!(app.state, AppState::Boundaries);
        app.next_tab(); // wraps
        assert_eq!(app.state, AppState::Dashboard);

        // Prev
        app.prev_tab();
        assert_eq!(app.state, AppState::Boundaries);

        // Back from Search
        app.state = AppState::Search;
        app.go_back();
        assert_eq!(app.state, AppState::Dashboard);
    }

    #[test]
    fn app_search_enter_disambiguation() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let sid = store.create_session("test").unwrap();
        store
            .insert_observation(&engram_store::AddObservationParams {
                r#type: engram_core::ObservationType::Bugfix,
                scope: engram_core::Scope::Project,
                title: "Test bug".into(),
                content: "Found an error".into(),
                session_id: sid,
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        let mut app = App::new(store, "test".into());
        app.state = AppState::Search;

        // First Enter with query → should search
        app.search_query = "error".into();
        app.handle_enter();
        assert!(!app.search_results.is_empty());
        assert_eq!(app.state, AppState::Search); // stays in search

        // Second Enter (same query) → should open detail
        app.handle_enter();
        assert_eq!(app.state, AppState::Detail);
        assert!(app.detail_observation.is_some());
    }

    #[test]
    fn app_search() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let sid = store.create_session("test").unwrap();
        store
            .insert_observation(&engram_store::AddObservationParams {
                r#type: engram_core::ObservationType::Bugfix,
                scope: engram_core::Scope::Project,
                title: "Test bug".into(),
                content: "Found an error".into(),
                session_id: sid,
                project: "test".into(),
                ..Default::default()
            })
            .unwrap();

        let mut app = App::new(store, "test".into());
        app.search_query = "error".into();
        app.search();
        assert!(!app.search_results.is_empty());
        assert_eq!(app.last_searched_query, "error");
    }

    #[test]
    fn app_detail_back_goes_to_search() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        app.state = AppState::Detail;
        app.go_back();
        assert_eq!(app.state, AppState::Search);
        assert!(app.detail_observation.is_none());
    }

    #[test]
    fn app_select_navigation_respects_context() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store.clone(), "test".into());

        // No results → can't move
        app.select_next();
        assert_eq!(app.selected_index, 0);

        // Add results
        let sid = store.create_session("test").unwrap();
        for i in 0..5 {
            store
                .insert_observation(&engram_store::AddObservationParams {
                    r#type: engram_core::ObservationType::Manual,
                    scope: engram_core::Scope::Project,
                    title: format!("Obs {i}"),
                    content: format!("Content {i}"),
                    session_id: sid.clone(),
                    project: "test".into(),
                    ..Default::default()
                })
                .unwrap();
        }
        app.state = AppState::Search;
        app.search_query = "Content".into();
        app.search();
        assert_eq!(app.search_results.len(), 5);

        // Navigate
        app.select_next();
        assert_eq!(app.selected_index, 1);
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_prev();
        assert_eq!(app.selected_index, 1);

        // Can't go below 0
        app.select_prev();
        app.select_prev();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn tab_index_matches() {
        assert_eq!(AppState::Dashboard.tab_index(), 0);
        assert_eq!(AppState::Search.tab_index(), 1);
        assert_eq!(AppState::Timeline.tab_index(), 2);
        assert_eq!(AppState::Capsules.tab_index(), 3);
        assert_eq!(AppState::Boundaries.tab_index(), 4);
    }

    #[test]
    fn input_mode_initial_state() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let app = App::new(store, "test".into());
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.show_help);
    }

    #[test]
    fn start_search_switches_mode() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        assert_eq!(app.state, AppState::Dashboard);
        assert_eq!(app.input_mode, InputMode::Normal);

        app.start_search();
        assert_eq!(app.state, AppState::Search);
        assert_eq!(app.input_mode, InputMode::Editing);
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn stop_editing_returns_to_normal() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        app.start_search();
        assert_eq!(app.input_mode, InputMode::Editing);
        app.stop_editing();
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn help_toggle() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        assert!(!app.show_help);
        app.show_help = true;
        assert!(app.show_help);
    }

    #[test]
    fn esc_in_search_clears_editing_first() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        app.start_search();
        app.search_query = "hello".into();
        assert_eq!(app.input_mode, InputMode::Editing);

        // First Esc: clear query, stop editing
        app.go_back();
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.search_query.is_empty());
        assert_eq!(app.state, AppState::Search); // still in search tab

        // Second Esc: go back to dashboard
        app.go_back();
        assert_eq!(app.state, AppState::Dashboard);
    }

    #[test]
    fn tab_switch_resets_input_mode() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        app.start_search();
        assert_eq!(app.input_mode, InputMode::Editing);

        // Switch tab via next_tab — should reset input mode
        app.next_tab();
        assert_eq!(app.state, AppState::Timeline);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn scroll_up_down() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        assert_eq!(app.scroll_offset, 0);

        app.scroll_down(5);
        assert_eq!(app.scroll_offset, 5);

        app.scroll_down(3);
        assert_eq!(app.scroll_offset, 8);

        app.scroll_up(3);
        assert_eq!(app.scroll_offset, 5);

        // Can't go below 0
        app.scroll_up(10);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn status_message_auto_clears() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        assert!(app.status_message.is_none());

        app.set_status("test msg");
        assert!(app.status_message.is_some());

        // Immediate tick should not clear
        app.tick_status();
        assert!(app.status_message.is_some());
    }

    #[test]
    fn capsule_detail_enter_esc() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        app.state = AppState::Capsules;
        assert!(app.detail_capsule.is_none());

        // No capsules — Enter should not crash
        app.handle_enter();
        assert!(app.detail_capsule.is_none());
    }

    #[test]
    fn go_back_clears_scroll_offset() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        app.state = AppState::Detail;
        app.detail_observation = Some(engram_core::Observation::new(
            engram_core::ObservationType::Discovery,
            engram_core::Scope::Project,
            "test".into(),
            "content".into(),
            "session".into(),
            "project".into(),
            None,
        ));
        app.scroll_offset = 10;

        app.go_back();
        assert_eq!(app.scroll_offset, 0);
        assert!(app.detail_observation.is_none());
        assert_eq!(app.state, AppState::Search);
    }

    #[test]
    fn pin_toggle_no_observation() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());
        // No detail observation — should return error
        let result = app.toggle_pin();
        assert!(result.is_err());
    }
}
