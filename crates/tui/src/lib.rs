pub mod app;

pub use app::{draw, App, AppState};

use engram_store::SqliteStore;

/// Run the interactive TUI. Blocks until user quits.
pub fn run_tui(store: SqliteStore, project: &str) -> anyhow::Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::io;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(std::sync::Arc::new(store), project.to_string());
    app.refresh_stats();

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char('1') => app.state = AppState::Dashboard,
                    KeyCode::Char('2') => app.state = AppState::Search,
                    KeyCode::Char('3') => {
                        app.state = AppState::Capsules;
                        app.refresh_capsules();
                    }
                    KeyCode::Char('4') => {
                        app.state = AppState::Boundaries;
                        app.refresh_boundaries();
                    }
                    KeyCode::Backspace => {
                        if app.state == AppState::Search {
                            app.search_query.pop();
                        }
                    }
                    KeyCode::Enter => {
                        if app.state == AppState::Search && !app.search_query.is_empty() {
                            app.search();
                        } else if app.state == AppState::Search && !app.search_results.is_empty() {
                            app.state = AppState::Detail;
                            app.detail_observation =
                                app.search_results.get(app.selected_index).cloned();
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_index > 0 {
                            app.selected_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.selected_index < app.search_results.len().saturating_sub(1) {
                            app.selected_index += 1;
                        }
                    }
                    KeyCode::Esc => {
                        app.state = AppState::Dashboard;
                        app.detail_observation = None;
                    }
                    KeyCode::Char(c) if app.state == AppState::Search => {
                        app.search_query.push(c);
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
