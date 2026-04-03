use laches::db::{date_range_for_day, Database, ProcessSummary, Session};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Tabs},
};

use crate::theme::Theme;
use crate::views;

const TAB_TITLES: [&str; 4] = ["today", "timeline", "trends", "sessions"];
const TAB_COUNT: usize = TAB_TITLES.len();

pub struct App<'a> {
    pub db: &'a Database,
    pub tab: usize,
    pub viewing_date: chrono::NaiveDate,

    pub scroll_offsets: [usize; TAB_COUNT],

    pub summaries: Vec<ProcessSummary>,
    pub sessions: Vec<Session>,
    pub active_secs: i64,
    pub idle_secs: i64,
    pub daily_totals: Vec<(String, i64)>,
    pub current_process: Option<String>,
    pub last_error: Option<String>,
}

impl<'a> App<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self {
            db,
            tab: 0,
            viewing_date: chrono::Local::now().date_naive(),
            scroll_offsets: [0; TAB_COUNT],
            summaries: Vec::new(),
            sessions: Vec::new(),
            active_secs: 0,
            idle_secs: 0,
            daily_totals: Vec::new(),
            current_process: None,
            last_error: None,
        }
    }

    pub fn is_viewing_today(&self) -> bool {
        self.viewing_date == chrono::Local::now().date_naive()
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

    pub fn prev_day(&mut self) {
        if let Some(d) = self.viewing_date.pred_opt() {
            self.viewing_date = d;
            self.scroll_offsets = [0; TAB_COUNT];
            self.refresh_data();
        }
    }

    pub fn next_day(&mut self) {
        let today = chrono::Local::now().date_naive();
        if let Some(d) = self.viewing_date.succ_opt() {
            if d <= today {
                self.viewing_date = d;
                self.scroll_offsets = [0; TAB_COUNT];
                self.refresh_data();
            }
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offsets[self.tab] = self.scroll_offsets[self.tab].saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        let max = self.scrollable_item_count(self.tab);
        if self.scroll_offsets[self.tab] < max {
            self.scroll_offsets[self.tab] += 1;
        }
    }

    fn scrollable_item_count(&self, tab: usize) -> usize {
        match tab {
            0 => self.summaries.len(),
            3 => self.sessions.iter().filter(|s| !s.idle).count(),
            _ => 0,
        }
    }

    pub fn refresh_data(&mut self) {
        self.last_error = None;

        let date_str = self.viewing_date.format("%Y-%m-%d").to_string();
        let (day_start, day_end) = match date_range_for_day(&date_str) {
            Some(r) => r,
            None => {
                self.last_error = Some(format!("invalid date: {}", date_str));
                return;
            }
        };

        match self.db.query_process_summaries(&day_start, &day_end, None) {
            Ok(v) => self.summaries = v,
            Err(e) => {
                self.last_error = Some(format!("query failed: {}", e));
                return;
            }
        }

        match self.db.query_sessions(&day_start, &day_end) {
            Ok(v) => self.sessions = v,
            Err(e) => {
                self.last_error = Some(format!("query failed: {}", e));
                return;
            }
        }

        self.active_secs = self
            .db
            .query_total_active_seconds(&day_start, &day_end)
            .unwrap_or(0);

        self.idle_secs = self
            .db
            .query_total_idle_seconds(&day_start, &day_end)
            .unwrap_or(0);

        self.daily_totals.clear();
        let today = chrono::Local::now().date_naive();
        let start_day = today - chrono::Duration::days(29);
        let (range_start, _) =
            date_range_for_day(&start_day.format("%Y-%m-%d").to_string()).unwrap_or_default();
        let (_, range_end) =
            date_range_for_day(&today.format("%Y-%m-%d").to_string()).unwrap_or_default();

        let db_totals: std::collections::HashMap<String, i64> =
            match self.db.query_daily_totals(&range_start, &range_end) {
                Ok(v) => v.into_iter().collect(),
                Err(e) => {
                    self.last_error = Some(format!("query failed: {}", e));
                    return;
                }
            };

        for i in (0..30).rev() {
            let date = today - chrono::Duration::days(i);
            let date_key = date.format("%Y-%m-%d").to_string();
            let total = db_totals.get(&date_key).copied().unwrap_or(0);
            self.daily_totals
                .push((date.format("%m/%d").to_string(), total));
        }

        self.current_process = if self.is_viewing_today() {
            self.db
                .get_open_session()
                .ok()
                .flatten()
                .filter(|s| !s.idle)
                .map(|s| s.process_name)
        } else {
            None
        };
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        let date_str = if self.is_viewing_today() {
            format!("today ({})", self.viewing_date.format("%a %b %d"))
        } else {
            self.viewing_date.format("%a %b %d, %Y").to_string()
        };
        let title = format!(" lachesis \u{2500} {} ", date_str);
        let tabs = Tabs::new(TAB_TITLES.iter().map(|t| Line::from(*t)))
            .block(Block::default().borders(Borders::ALL).title(title))
            .select(self.tab)
            .style(theme.tab_inactive())
            .highlight_style(theme.tab_active())
            .divider(theme.separator());
        frame.render_widget(tabs, chunks[0]);

        match self.tab {
            0 => views::today::render(self, frame, chunks[1], theme),
            1 => views::timeline::render(self, frame, chunks[1], theme),
            2 => views::trends::render(self, frame, chunks[1], theme),
            3 => views::sessions::render(self, frame, chunks[1], theme),
            _ => {}
        }

        let footer = if let Some(ref err) = self.last_error {
            Line::from(vec![
                Span::styled(" ERROR ", theme.error_label()),
                Span::styled(err.as_str(), theme.error_text()),
            ])
        } else {
            let sep = theme.separator();
            let time_str = chrono::Local::now().format("%H:%M").to_string();
            Line::from(vec![
                Span::styled(" q", theme.key_hint()),
                Span::styled(" quit", theme.key_desc()),
                sep.clone(),
                Span::styled("tab", theme.key_hint()),
                Span::styled(" switch", theme.key_desc()),
                sep.clone(),
                Span::styled("h/l", theme.key_hint()),
                Span::styled(" day", theme.key_desc()),
                sep.clone(),
                Span::styled("j/k", theme.key_hint()),
                Span::styled(" scroll", theme.key_desc()),
                sep.clone(),
                Span::styled("r", theme.key_hint()),
                Span::styled(" refresh", theme.key_desc()),
                sep,
                Span::styled(time_str, theme.key_desc()),
            ])
        };
        frame.render_widget(footer, chunks[2]);
    }
}
