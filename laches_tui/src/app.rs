use laches::db::{today_range, Database, ProcessSummary, Session};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Tabs},
};

use crate::views;

const TAB_TITLES: [&str; 4] = ["today", "timeline", "trends", "sessions"];
const TAB_COUNT: usize = TAB_TITLES.len();

pub struct App<'a> {
    pub db: &'a Database,
    pub tab: usize,

    /// Per-view scroll offsets, indexed by tab number.
    /// Each view manages its own scroll position independently.
    pub scroll_offsets: [usize; TAB_COUNT],

    // cached data (refreshed periodically)
    pub today_summaries: Vec<ProcessSummary>,
    pub today_sessions: Vec<Session>,
    pub today_active: i64,
    pub today_idle: i64,
    pub daily_totals: Vec<(String, i64)>,
    pub current_process: Option<String>,
}

impl<'a> App<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            db,
            tab: 0,
            scroll_offsets: [0; TAB_COUNT],
            today_summaries: Vec::new(),
            today_sessions: Vec::new(),
            today_active: 0,
            today_idle: 0,
            daily_totals: Vec::new(),
            current_process: None,
        }
    }

    pub fn set_tab(&mut self, tab: usize) {
        if tab < TAB_COUNT {
            self.tab = tab;
        }
    }

    pub fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % TAB_COUNT;
    }

    pub fn prev_tab(&mut self) {
        self.tab = if self.tab == 0 {
            TAB_COUNT - 1
        } else {
            self.tab - 1
        };
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offsets[self.tab] = self.scroll_offsets[self.tab].saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offsets[self.tab] = self.scroll_offsets[self.tab].saturating_add(1);
    }

    pub fn refresh_data(&mut self) {
        let (today_start, today_end) = today_range();

        self.today_summaries = self
            .db
            .query_process_summaries(&today_start, &today_end, None)
            .unwrap_or_default();

        self.today_sessions = self
            .db
            .query_sessions(&today_start, &today_end)
            .unwrap_or_default();

        self.today_active = self
            .db
            .query_total_active_seconds(&today_start, &today_end)
            .unwrap_or(0);

        self.today_idle = self
            .db
            .query_total_idle_seconds(&today_start, &today_end)
            .unwrap_or(0);

        // daily totals for the last 30 days (single aggregated query)
        self.daily_totals.clear();
        let today = chrono::Local::now().date_naive();
        let start_day = today - chrono::Duration::days(29);
        let (range_start, _) =
            laches::db::date_range_for_day(&start_day.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
        let (_, range_end) = laches::db::date_range_for_day(&today.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        let db_totals: std::collections::HashMap<String, i64> = self
            .db
            .query_daily_totals(&range_start, &range_end)
            .unwrap_or_default()
            .into_iter()
            .collect();

        for i in (0..30).rev() {
            let date = today - chrono::Duration::days(i);
            let date_key = date.format("%Y-%m-%d").to_string();
            let total = db_totals.get(&date_key).copied().unwrap_or(0);
            self.daily_totals
                .push((date.format("%m/%d").to_string(), total));
        }

        // current process (open session)
        self.current_process = self
            .db
            .get_open_session()
            .ok()
            .flatten()
            .filter(|s| !s.idle)
            .map(|s| s.process_name);
    }

    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        // tab bar with date in title
        let date_str = chrono::Local::now().format("%a %b %d").to_string();
        let title = format!(" lachesis \u{2500} {} ", date_str);
        let tabs = Tabs::new(TAB_TITLES.iter().map(|t| Line::from(*t)))
            .block(Block::default().borders(Borders::ALL).title(title))
            .select(self.tab)
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(Style::default().fg(Color::Cyan).bold())
            .divider(Span::styled(
                " \u{2502} ",
                Style::default().fg(Color::DarkGray),
            ));
        frame.render_widget(tabs, chunks[0]);

        // active view
        match self.tab {
            0 => views::today::render(self, frame, chunks[1]),
            1 => views::timeline::render(self, frame, chunks[1]),
            2 => views::trends::render(self, frame, chunks[1]),
            3 => views::sessions::render(self, frame, chunks[1]),
            _ => {}
        }

        // footer with structured key hints and time
        let sep = Span::styled(" \u{2502} ", Style::default().fg(Color::DarkGray));
        let time_str = chrono::Local::now().format("%H:%M").to_string();
        let footer = Line::from(vec![
            Span::styled(" q", Style::default().bold()),
            Span::styled(" quit", Style::default().fg(Color::DarkGray)),
            sep.clone(),
            Span::styled("tab", Style::default().bold()),
            Span::styled(" switch", Style::default().fg(Color::DarkGray)),
            sep.clone(),
            Span::styled("j/k", Style::default().bold()),
            Span::styled(" scroll", Style::default().fg(Color::DarkGray)),
            sep.clone(),
            Span::styled("r", Style::default().bold()),
            Span::styled(" refresh", Style::default().fg(Color::DarkGray)),
            sep,
            Span::styled(time_str, Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(footer, chunks[2]);
    }
}
