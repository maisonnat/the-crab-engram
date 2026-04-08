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

/// TUI application state.
pub struct App {
    pub store: Arc<SqliteStore>,
    pub project: String,
    pub state: AppState,
    pub should_quit: bool,
    pub search_query: String,
    pub search_results: Vec<Observation>,
    pub selected_index: usize,
    pub detail_observation: Option<Observation>,
    pub stats: Option<engram_store::ProjectStats>,
    pub capsules: Vec<engram_core::KnowledgeCapsule>,
    pub boundaries: Vec<(String, String, String)>,
}

impl App {
    pub fn new(store: Arc<SqliteStore>, project: String) -> Self {
        Self {
            store,
            project,
            state: AppState::Dashboard,
            should_quit: false,
            search_query: String::new(),
            search_results: Vec::new(),
            selected_index: 0,
            detail_observation: None,
            stats: None,
            capsules: Vec::new(),
            boundaries: Vec::new(),
        }
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
            self.search_results = results;
            self.selected_index = 0;
        }
    }

    pub fn select_next(&mut self) {
        let max = match self.state {
            AppState::Search => self.search_results.len(),
            _ => 0,
        };
        if max > 0 && self.selected_index < max - 1 {
            self.selected_index += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn open_detail(&mut self) {
        if let Some(obs) = self.search_results.get(self.selected_index) {
            self.detail_observation = Some(obs.clone());
            self.state = AppState::Detail;
        }
    }

    pub fn go_back(&mut self) {
        match self.state {
            AppState::Detail => {
                self.detail_observation = None;
                self.state = AppState::Search;
            }
            AppState::Search => {
                self.state = AppState::Dashboard;
            }
            _ => {}
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
        AppState::Timeline => draw_timeline(f, app, chunks[1]),
        AppState::Capsules => draw_capsules(f, app, chunks[1]),
        AppState::Boundaries => draw_boundaries(f, app, chunks[1]),
    }

    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        "Dashboard",
        "Search",
        "Detail",
        "Timeline",
        "Capsules",
        "Boundaries",
    ];
    let selected = match app.state {
        AppState::Dashboard => 0,
        AppState::Search => 1,
        AppState::Detail => 2,
        AppState::Timeline => 3,
        AppState::Capsules => 4,
        AppState::Boundaries => 5,
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" engram — {} ", app.project)),
        )
        .select(selected)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Stats panel
    let stats_text = if let Some(stats) = &app.stats {
        let mut lines = vec![
            Line::from(vec![
                Span::raw("Observations: "),
                Span::styled(
                    stats.total_observations.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("Sessions: "),
                Span::styled(
                    stats.total_sessions.to_string(),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::raw("Edges: "),
                Span::styled(
                    stats.total_edges.to_string(),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "By Type:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ];
        for (t, count) in &stats.by_type {
            lines.push(Line::from(format!("  {t}: {count}")));
        }
        lines
    } else {
        vec![Line::from("Loading...")]
    };

    let stats_widget = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title(" Stats "))
        .wrap(Wrap { trim: true });
    f.render_widget(stats_widget, chunks[0]);

    // Help panel
    let help = Paragraph::new(vec![
        Line::from("  [s] Search"),
        Line::from("  [d] Dashboard"),
        Line::from("  [q] Quit"),
        Line::from(""),
        Line::from("  Navigate: ↑/↓"),
        Line::from("  Open: Enter"),
        Line::from("  Back: Esc"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Keyboard Shortcuts "),
    );
    f.render_widget(help, chunks[1]);
}

fn draw_search(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search input
    let search_input = Paragraph::new(app.search_query.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search (type + Enter) "),
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
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!(
                "[{}] {} ({}x) — {}",
                obs.r#type,
                obs.title,
                obs.access_count,
                obs.content.chars().take(80).collect::<String>()
            ))
            .style(style)
        })
        .collect();

    let results_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Results ({}) ", app.search_results.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(results_list, chunks[1]);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    if let Some(obs) = &app.detail_observation {
        let detail_text = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(obs.id.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}", obs.r#type), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Scope: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", obs.scope)),
            ]),
            Line::from(vec![
                Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&obs.title),
            ]),
            Line::from(vec![
                Span::styled("Topic: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(obs.topic_key.as_deref().unwrap_or("—")),
            ]),
            Line::from(vec![
                Span::styled("Access: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}x", obs.access_count)),
            ]),
            Line::from(vec![
                Span::styled("Pinned: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if obs.pinned { "yes" } else { "no" }),
            ]),
            Line::from(vec![
                Span::styled(
                    "Provenance: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(
                    "{:?} ({:.0}%)",
                    obs.provenance_source,
                    obs.provenance_confidence * 100.0
                )),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "Content:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::raw(&obs.content),
        ];

        let detail = Paragraph::new(detail_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Observation #{} ", obs.id)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(detail, area);
    }
}

fn draw_timeline(f: &mut Frame, app: &App, area: Rect) {
    let observations = &app.search_results;
    let mut items: Vec<ListItem> = Vec::new();

    for obs in observations.iter().take(20) {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("[{}] ", obs.created_at.format("%H:%M")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("[{}] ", obs.r#type),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(obs.title.clone()),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(
            "No observations loaded. Press [s] to search.",
        ));
    }

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Timeline "));
    f.render_widget(list, area);
}

fn draw_capsules(f: &mut Frame, app: &App, area: Rect) {
    let capsules = &app.capsules;
    let mut items: Vec<ListItem> = Vec::new();

    for cap in capsules {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("📌 {} ", cap.topic),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({:.0}%, v{}) ", cap.confidence * 100.0, cap.version),
                Style::default().fg(Color::Green),
            ),
            Span::raw(cap.summary.chars().take(60).collect::<String>()),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new("No capsules found."));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Knowledge Capsules "),
    );
    f.render_widget(list, area);
}

fn draw_boundaries(f: &mut Frame, app: &App, area: Rect) {
    let boundaries = &app.boundaries;
    let mut items: Vec<ListItem> = Vec::new();

    for (domain, level, evidence) in boundaries {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("🗺️ {domain}: "),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{level} "), Style::default().fg(Color::Yellow)),
            Span::raw(evidence.chars().take(40).collect::<String>()),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new("No knowledge boundaries defined."));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Knowledge Boundaries "),
    );
    f.render_widget(list, area);
}

fn draw_footer(f: &mut Frame, _app: &App, area: Rect) {
    let footer = Paragraph::new(
        " [s] Search  [d] Dashboard  [q] Quit  [Esc] Back  [↑↓] Navigate  [Enter] Open ",
    )
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::ALL));
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
    }

    #[test]
    fn app_navigation() {
        let store = Arc::new(SqliteStore::in_memory().unwrap());
        let mut app = App::new(store, "test".into());

        app.state = AppState::Search;
        app.select_next();
        assert_eq!(app.selected_index, 0); // No results, can't move

        app.go_back();
        assert_eq!(app.state, AppState::Dashboard);
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
    }
}
