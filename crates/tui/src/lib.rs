pub mod app;

pub use app::{App, AppState, InputMode, draw};

use engram_store::SqliteStore;

/// RAII guard that restores the terminal on Drop (including panics).
struct TerminalGuard;

impl TerminalGuard {
    fn init() -> anyhow::Result<Self> {
        use crossterm::{
            execute,
            terminal::{EnterAlternateScreen, enable_raw_mode},
        };
        use std::io;

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        use crossterm::{
            execute,
            terminal::{LeaveAlternateScreen, disable_raw_mode},
        };
        use std::io;

        // Best-effort restore — ignore errors (terminal may already be gone)
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Run the interactive TUI. Blocks until user quits.
pub fn run_tui(store: SqliteStore, project: &str) -> anyhow::Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use ratatui::{Terminal, backend::CrosstermBackend};
    use std::io;

    // TerminalGuard ensures cleanup even on panic
    let _guard = TerminalGuard::init()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(std::sync::Arc::new(store), project.to_string());
    app.refresh_all();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                // ── Help overlay ──────────────────────────────────────
                KeyCode::Char('?') if !app.is_editing() => {
                    app.show_help = !app.show_help;
                }

                // ── Dismiss help on any key when visible ────────────
                _ if app.show_help => {
                    app.show_help = false;
                    continue;
                }

                // ── Quick search from any tab (/ vim convention) ────
                KeyCode::Char('/') if !app.is_editing() => {
                    app.start_search();
                }

                // ── Tab switching ─────────────────────────────────────
                KeyCode::Char('1') | KeyCode::F(1) => {
                    app.state = AppState::Dashboard;
                }
                KeyCode::Char('2') | KeyCode::F(2) => {
                    app.state = AppState::Search;
                }
                KeyCode::Char('3') | KeyCode::F(3) => {
                    app.state = AppState::Timeline;
                    app.refresh_timeline();
                }
                KeyCode::Char('4') | KeyCode::F(4) => {
                    app.state = AppState::Capsules;
                    app.refresh_capsules();
                }
                KeyCode::Char('5') | KeyCode::F(5) => {
                    app.state = AppState::Boundaries;
                    app.refresh_boundaries();
                }
                KeyCode::Right | KeyCode::Tab => {
                    app.next_tab();
                }
                KeyCode::Left => {
                    app.prev_tab();
                }

                // ── Quit ──────────────────────────────────────────────
                KeyCode::Char('q') if !app.is_editing() => {
                    app.should_quit = true;
                }

                // ── Back navigation ──────────────────────────────────
                KeyCode::Esc => {
                    app.go_back();
                }

                // ── Search input ─────────────────────────────────────
                KeyCode::Backspace if app.is_editing() => {
                    app.search_query.pop();
                }
                KeyCode::Char(c) if app.is_editing() => {
                    app.search_query.push(c);
                }
                KeyCode::Enter => {
                    app.handle_enter();
                }

                // ── List navigation ─────────────────────────────────
                KeyCode::Up | KeyCode::Char('k') => {
                    app.select_prev();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.select_next();
                }

                // ── Detail view: scroll & pin ───────────────────────
                KeyCode::PageUp => {
                    app.scroll_up(5);
                }
                KeyCode::PageDown => {
                    app.scroll_down(5);
                }
                KeyCode::Char('p') if !app.is_editing() => {
                    if let Err(e) = app.toggle_pin() {
                        app.set_status(format!("Error: {e}"));
                    }
                }

                _ => {}
            }
        }

        if app.should_quit {
            break;
        }

        // Auto-clear status message after 4 seconds
        app.tick_status();
    }

    Ok(())
}
