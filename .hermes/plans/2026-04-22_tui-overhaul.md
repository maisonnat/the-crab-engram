# Plan: TUI Overhaul — Bugs, UX & Implementation

**Date:** 2026-04-22  
**Scope:** `crates/tui/` (597 LOC, 2 files)  
**Stack:** Ratatui 0.29 + Crossterm 0.29  

---

## 1. Bug Report

### Critical Bugs

| # | Bug | Severity | File:Line | Description | Fix |
|---|---|---|---|---|---|
| **B1** | **Enter ambiguity in Search** | 🔴 High | `lib.rs:50-57` | Enter does double duty: "execute search" AND "open detail". If query is empty but results exist, pressing Enter opens detail. But if query is NOT empty, Enter always searches — can never open detail while query has text. | Split: `Enter` = search, `Tab` or `→` = open detail. Or clear query on first Enter then second Enter opens. |
| **B2** | **Timeline tab has no data source** | 🔴 High | `app.rs:366-392` | `draw_timeline` reads from `self.search_results` (which is only populated by Search). Timeline tab always shows "No observations loaded" until user does a search first. | Timeline should load recent observations on tab activation: `store.get_session_context()` or a new `recent_observations()` query. |
| **B3** | **Tab 5 (Boundaries) no keyboard shortcut** | 🟡 Medium | `lib.rs:35-74` | Tabs 1-4 have shortcuts (1-4), but Boundaries has NO key to activate it. Footer says nothing about it either. User can't reach it. | Add `KeyCode::Char('5') => Boundaries` or reorganize tabs. |
| **B4** | **Footer shortcuts are wrong** | 🟡 Medium | `app.rs:455-461` | Footer shows `[s] Search [d] Dashboard` but actual code uses `1`/`2`/`3`/`4` for tab switching, NOT `s`/`d`. Pressing `s` in Dashboard types 's' into search query (no, wait — Search only activates on char input). Actually: pressing ANY char in Dashboard does nothing. `s` and `d` are completely non-functional. | Either implement `s`/`d` shortcuts OR fix footer to show `1-6` for tabs. |
| **B5** | **Search typing captured globally** | 🟡 Medium | `lib.rs:71-73` | `KeyCode::Char(c) if app.state == AppState::Search` — this catches ALL character input only in Search state. But there's no way to type in other views. If we add text input elsewhere, this will conflict. | Architecture: add an `input_mode: Option<InputField>` enum. Only capture chars when `input_mode.is_some()`. |
| **B6** | **Backspace only works in Search** | 🟢 Low | `lib.rs:47-49` | `Backspace` handler is gated to `AppState::Search`. Correct behavior, but inconsistent with `Char` input which is also gated. Just noting this is fine. | No fix needed — just document. |
| **B7** | **No terminal restore on panic** | 🔴 High | `lib.rs:17-84` | If the app panics between `enable_raw_mode()` and `disable_raw_mode()`, the terminal is left in raw mode. User sees invisible/garbled terminal. | Wrap in `scopeguard` or use a `Drop` guard struct that calls `disable_raw_mode` + `LeaveAlternateScreen`. |
| **B8** | **`select_next()`/`select_prev()` defined but unused** | 🟢 Low | `app.rs:85-99` | Methods exist on `App` but are never called. The actual j/k/Up/Down handling is inline in `lib.rs:59-66`. Dead code. | Remove methods or use them from lib.rs. |
| **B9** | **Version mismatch** | 🟢 Low | `main.rs:27` | CLI says `version = "2.0.0"` but workspace is `2.1.0`. Cosmetic but misleading. | Use `env!("CARGO_PKG_VERSION")` or update hardcoded string. |

---

## 2. UX/UI Improvements

### 2.1 Navigation Problems

**Current**: Number keys (1-6) switch tabs. But footer says `[s] [d] [q]`.
**Problem**: Confusing. User has to memorize 6 number keys.

**Proposed**:
- **Global keys**: `q` quit, `Esc` back, `?` help overlay
- **Tab switching**: Left/Right arrows OR `Tab`/`Shift+Tab` (ratatui convention)
- **Vim keys**: `h`/`l` for tab prev/next (also vim convention)
- **Remove number keys** for tabs — they're unergonomic and don't scale

### 2.2 Search UX

**Current**: Type → Enter → results appear → Enter again → opens detail. Can't edit query after searching.

**Proposed**:
```
Search Mode:
  [Type]          → builds query
  [Enter]         → execute search  
  [Esc]           → clear query / go back
  [↑↓/j/k]       → navigate results
  [Enter] on result → open detail (when query hasn't changed)
  [Tab]           → toggle focus between input and results
  [/]             → start search from any tab (vim convention)
```

### 2.3 Detail View

**Current**: Static display of one observation. No way to navigate to connected observations.

**Proposed**:
- Show connected edges at bottom: `[e] View edges (3)`
- `e` opens edge list → select one → opens that observation
- `[p] Pin/unpin` toggle
- `[t] Timeline` — opens timeline centered on this observation

### 2.4 Dashboard Improvements

**Current**: Stats panel + keyboard shortcuts panel (with wrong shortcuts).

**Proposed**:
```
┌──────────────────────────────────────────────────────────┐
│  The Crab Engram — default                               │
│  [Dashboard] [Search] [Timeline] [Capsules] [Boundaries] │
├──────────────────────┬───────────────────────────────────┤
│  📊 Stats            │  🕐 Recent Activity               │
│  Observations: 41    │  20:30 [manual] "Skill updated..."│
│  Sessions: 11        │  20:28 [discovery] "MCP format..."│
│  Edges: 3            │  20:15 [architecture] "Claude..." │
│  Capsules: 0         │  19:45 [bugfix] "Migration..."    │
│                      │  19:30 [learning] "NTFS patch..."  │
│  By Type:            │                                   │
│   manual: 16         │  ─────────────────────────────── │
│   discovery: 12      │  Press [/] to search, [?] for help│
│   architecture: 7    │                                   │
├──────────────────────┴───────────────────────────────────┤
│  [/] Search  [Tab] Next  [q] Quit  [?] Help  [Esc] Back │
└──────────────────────────────────────────────────────────┘
```

Changes:
- Replace "Keyboard Shortcuts" panel with "Recent Activity" (auto-loaded last 10 observations)
- Move shortcuts to footer only
- Add total capsules count from stats

### 2.5 Timeline Tab

**Current**: Dead tab — requires search first.

**Proposed**: Auto-load recent observations chronologically on tab activation:
```
Timeline (auto-loaded):
  20:35 [manual]     "Verified MCP status"
  20:30 [manual]     "Skill updated with..."
  20:28 [discovery]  "MCP format research..."
  20:15 [arch]       "Claude Code uses..."
  ...
  
  [↑↓] Navigate  [Enter] Open  [d] Date filter
```

### 2.6 Capsules & Boundaries Detail

**Current**: Flat list with truncated text. No way to see full capsule.

**Proposed**: 
- Select capsule → Enter → show full `to_markdown()` output in scrollable view
- Same pattern as Search → Detail

### 2.7 Missing Features

| Feature | Description | Priority |
|---|---|---|
| **Scroll** | Long content (Detail, Capsules) has no scroll. Need `Scrollbar` widget from ratatui 0.29 | High |
| **Help overlay** | `?` shows a modal with all keybindings | Medium |
| **Status bar** | Show last action result: "Found 5 results", "Pinned #42", etc. | Medium |
| **Project switcher** | `p` opens project list to switch context | Low |
| **Pin toggle** | `p` on detail view toggles pinned state | Medium |
| **Delete confirm** | `x` on observation prompts "Delete? y/n" | Low |

---

## 3. Architecture Refactoring

### 3.1 Current Structure (Monolithic)
```
crates/tui/src/
├── lib.rs    (86 lines) — event loop + raw terminal setup
└── app.rs    (511 lines) — state + draw + logic all mixed
```

### 3.2 Proposed Structure (Modular)
```
crates/tui/src/
├── lib.rs          — run_tui() entry point + terminal guard
├── app.rs          — App struct, state machine, business logic only
├── ui/
│   ├── mod.rs      — pub fn draw() dispatcher
│   ├── header.rs   — tab bar rendering
│   ├── footer.rs   — status bar + keybinding hints
│   ├── dashboard.rs — stats + recent activity
│   ├── search.rs   — search input + results list
│   ├── detail.rs   — observation detail + scroll
│   ├── timeline.rs  — chronological view
│   ├── capsules.rs  — capsule list + detail
│   └── boundaries.rs — boundary list
├── handlers.rs     — key event handling (match key → app method)
└── terminal.rs     — TerminalGuard (Drop for restore)
```

### 3.3 State Machine Fix

**Current**: Flat `AppState` enum with no concept of input mode.

**Proposed**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Dashboard,
    Search,
    Detail,
    Timeline,
    Capsules,
    Boundaries,
    Help,  // NEW: overlay
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,              // Navigation keys active
    Editing(SearchField), // Text input captured
}

pub enum SearchField {
    Query,
}
```

---

## 4. Implementation Plan — Atomic Subtasks

### Phase 1: Bug Fixes (no structural changes)

#### T1.1 — Terminal Guard (B7)
- [ ] Create `TerminalGuard` struct in `lib.rs`
- [ ] Implement `Drop` that calls `disable_raw_mode()` + `LeaveAlternateScreen`
- [ ] Wrap terminal setup in guard creation
- **Files**: `lib.rs`
- **LOC**: ~20 added

#### T1.2 — Fix Footer Shortcuts (B4)
- [ ] Update `draw_footer()` to show actual working keys: `[1-6] Tabs  [↑↓] Nav  [Enter] Open  [Esc] Back  [q] Quit`
- **Files**: `app.rs:455-461`
- **LOC**: ~5 changed

#### T1.3 — Fix Enter Ambiguity in Search (B1)
- [ ] Change behavior: first Enter = search, subsequent Enter = open detail (only if query unchanged since last search)
- [ ] Add `last_searched_query: String` to `App`
- [ ] On Enter: if `search_query != last_searched_query` → search. Else → open detail.
- **Files**: `lib.rs:50-57`, `app.rs`
- **LOC**: ~15 changed

#### T1.4 — Add Boundaries Shortcut (B3)
- [ ] Add `KeyCode::Char('5')` handler for Boundaries tab
- **Files**: `lib.rs`
- **LOC**: ~4 added

#### T1.5 — Timeline Auto-Load (B2)
- [ ] Add `timeline_observations: Vec<Observation>` to `App`
- [ ] Add `refresh_timeline()` method that calls `store.get_session_context(&project, 20)`
- [ ] Update `draw_timeline()` to use `self.timeline_observations` instead of `self.search_results`
- [ ] Call `refresh_timeline()` on Timeline tab activation
- **Files**: `app.rs`, `lib.rs`
- **LOC**: ~20 changed

#### T1.6 — Remove Dead Code (B8)
- [ ] Remove unused `select_next()` and `select_prev()` methods (or wire them up)
- **Files**: `app.rs:85-99`
- **LOC**: ~15 removed

#### T1.7 — Fix Version String (B9)
- [ ] Change hardcoded `"2.0.0"` to `env!("CARGO_PKG_VERSION")` in CLI parser
- **Files**: `src/main.rs:27`
- **LOC**: ~1 changed

### Phase 2: Navigation Overhaul

#### T2.1 — Add Left/Right Tab Navigation
- [ ] Add `KeyCode::Left` / `KeyCode::Right` handlers to cycle tabs
- [ ] Add `KeyCode::Tab` / `KeyCode::BackTab` for next/prev tab
- [ ] Create `impl AppState { fn next(self) -> Self }` and `fn prev(self) -> Self`
- **Files**: `lib.rs`, `app.rs`
- **LOC**: ~25 added

#### T2.2 — Add `/` to Start Search from Any Tab
- [ ] `KeyCode::Char('/')` in Normal mode → switch to Search + focus input
- **Files**: `lib.rs`
- **LOC**: ~5 added

#### T2.3 — Add `?` Help Overlay
- [ ] Add `AppState::Help` variant
- [ ] `KeyCode::Char('?')` toggles Help state
- [ ] `draw_help()` renders a centered modal with all keybindings
- [ ] Help overlay drawn ON TOP of current view (not replacing it)
- **Files**: `app.rs`, `lib.rs`
- **LOC**: ~60 added

#### T2.4 — Input Mode Architecture
- [ ] Add `InputMode` enum to `app.rs`
- [ ] Add `input_mode: InputMode` field to `App`
- [ ] Gate char/backspace handlers on `InputMode::Editing`
- [ ] All other keys work in `InputMode::Normal`
- **Files**: `app.rs`, `lib.rs`
- **LOC**: ~30 changed

### Phase 3: UI Improvements

#### T3.1 — Replace Help Panel with Recent Activity
- [ ] Add `recent_observations: Vec<Observation>` to `App`
- [ ] Add `refresh_recent()` that loads last 10 observations
- [ ] Redraw Dashboard right panel as "Recent Activity" with colored type badges
- **Files**: `app.rs`
- **LOC**: ~40 changed

#### T3.2 — Add Scroll Support to Detail View
- [ ] Add `scroll_offset: u16` to `App`
- [ ] `KeyCode::PageDown` / `KeyCode::PageUp` to scroll
- [ ] Use ratatui `Scrollbar` widget on detail view
- [ ] Reset scroll on detail open
- **Files**: `app.rs`, `lib.rs`
- **LOC**: ~40 added

#### T3.3 — Capsule Detail View
- [ ] Add `detail_capsule: Option<KnowledgeCapsule>` to `App`
- [ ] `Enter` on capsule → shows full `to_markdown()` output in scrollable view
- [ ] `Esc` returns to capsule list
- **Files**: `app.rs`, `lib.rs`
- **LOC**: ~30 added

#### T3.4 — Pin Toggle in Detail View
- [ ] `KeyCode::Char('p')` in Detail view → toggle `obs.pinned`
- [ ] Show visual indicator when pinned (📌 in header)
- [ ] Call `store.update_observation()` to persist
- **Files**: `app.rs`, `lib.rs`
- **LOC**: ~15 added

#### T3.5 — Status Message Bar
- [ ] Add `status_message: Option<(String, Instant)>` to `App`
- [ ] Show message in footer area for 3 seconds (configurable)
- [ ] Methods: `app.set_status("Found 5 results")`
- [ ] Auto-clear after timeout
- **Files**: `app.rs`
- **LOC**: ~20 added

### Phase 4: Visual Polish

#### T4.1 — Color Theme Consistency
- [ ] Define a `Theme` struct with named colors (primary, secondary, accent, muted, bg)
- [ ] Apply consistent colors: Cyan = primary, Yellow = warning, Green = success, DarkGray = muted
- [ ] Type badges colored by category (Bugfix = Red, Architecture = Blue, etc.)
- **Files**: new `ui/theme.rs`
- **LOC**: ~40 added

#### T4.2 — Add Crab ASCII Art to Dashboard
- [ ] Small crab ASCII art in dashboard header or welcome
- [ ] Fun branding touch 🦀
- **Files**: `app.rs` or `ui/dashboard.rs`
- **LOC**: ~10 added

### Phase 5: Modular Refactor (optional, after all features)

#### T5.1 — Split app.rs into ui/ modules
- [ ] Move each `draw_*` function to its own file under `ui/`
- [ ] Move key handling to `handlers.rs`
- [ ] Move terminal setup to `terminal.rs`
- [ ] Keep `app.rs` as pure state + logic
- **Files**: Restructure `crates/tui/src/`
- **LOC**: ~0 net change (pure refactor)

---

## 5. Execution Priority

**Do first** (unblocks everything):
1. T1.1 Terminal Guard — prevents terminal corruption
2. T1.3 Enter Ambiguity — most annoying UX bug
3. T1.5 Timeline Auto-Load — makes Timeline tab useful

**Do second** (navigation):
4. T2.1 Left/Right tab nav
5. T2.4 Input Mode architecture
6. T2.2 `/` to search from anywhere

**Do third** (UI improvements):
7. T3.1 Recent Activity on Dashboard
8. T3.2 Scroll support
9. T3.3 Capsule detail
10. T2.3 Help overlay

**Do last** (polish):
11. T4.1 Color theme
12. T4.2 Crab art
13. T5.1 Modular refactor

**Quick wins to batch**:
- T1.2 + T1.4 + T1.6 + T1.7 → single commit (footer + shortcut + dead code + version)

---

## 6. Testing Strategy

For each subtask:
- Existing tests: `app_creation`, `app_navigation`, `app_search` — ensure they pass
- New test per feature:
  - T1.3: Test Enter logic with same/different query
  - T2.1: Test tab cycling wraps around
  - T3.2: Test scroll offset clamps correctly
- Visual test: `cargo run -- tui` manual verification
- CI: `cargo test -p engram-tui` passes

---

## 7. Files Summary

| File | Action | Phase |
|---|---|---|
| `crates/tui/src/lib.rs` | Modify (terminal guard + event handling) | 1, 2 |
| `crates/tui/src/app.rs` | Modify (all fixes + features) | 1-4 |
| `crates/tui/src/ui/mod.rs` | Create | 5 |
| `crates/tui/src/ui/*.rs` | Create (7 files) | 5 |
| `crates/tui/src/handlers.rs` | Create | 5 |
| `crates/tui/src/terminal.rs` | Create | 5 |
| `src/main.rs` | Modify (version fix) | 1 |

**Total new LOC estimate**: ~300-400 lines (features)  
**Total modified LOC**: ~100 lines (bug fixes)  
