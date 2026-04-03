use laches::db::{date_range_for_day, Database, ProcessSummary, Session};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Padding, Paragraph, Tabs},
};
use std::path::PathBuf;

use crate::theme::Theme;
use crate::views;

const TAB_TITLES: [&str; 4] = ["today", "timeline", "trends", "sessions"];
const TAB_COUNT: usize = TAB_TITLES.len();

pub struct TagGroup {
    pub tag: String,
    pub total_seconds: i64,
    pub processes: Vec<String>,
}

pub struct App<'a> {
    pub db: &'a Database,
    pub config_dir: PathBuf,
    pub tab: usize,
    pub viewing_date: chrono::NaiveDate,

    pub scroll_offsets: [usize; TAB_COUNT],

    pub summaries: Vec<ProcessSummary>,
    pub sessions: Vec<Session>,
    pub active_secs: i64,
    pub idle_secs: i64,
    pub daily_totals: Vec<(String, i64)>,
    pub current_process: Option<String>,
    pub current_window_title: Option<String>,
    pub daemon_running: bool,
    pub show_help: bool,
    pub group_by_tag: bool,
    pub tag_groups: Vec<TagGroup>,
    pub last_error: Option<String>,
}

impl<'a> App<'a> {
    pub fn new(db: &'a Database, config_dir: PathBuf) -> Self {
        Self {
            db,
            config_dir,
            tab: 0,
            viewing_date: chrono::Local::now().date_naive(),
            scroll_offsets: [0; TAB_COUNT],
            summaries: Vec::new(),
            sessions: Vec::new(),
            active_secs: 0,
            idle_secs: 0,
            daily_totals: Vec::new(),
            current_process: None,
            current_window_title: None,
            daemon_running: false,
            show_help: false,
            group_by_tag: false,
            tag_groups: Vec::new(),
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

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn toggle_group_by_tag(&mut self) {
        self.group_by_tag = !self.group_by_tag;
        self.scroll_offsets[0] = 0;
        self.rebuild_tag_groups();
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
            0 => {
                if self.group_by_tag {
                    self.tag_groups.len()
                } else {
                    self.summaries.len()
                }
            }
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

        if self.is_viewing_today() {
            let open = self
                .db
                .get_open_session()
                .ok()
                .flatten()
                .filter(|s| !s.idle);
            self.current_process = open.map(|s| s.process_name);

            let tracker = laches::platform::create_tracker();
            if let Some(info) = tracker.get_focused_window() {
                self.current_window_title = info.window_title;
            } else {
                self.current_window_title = None;
            }
        } else {
            self.current_process = None;
            self.current_window_title = None;
        }

        self.daemon_running = laches::process::is_daemon_running(&self.config_dir);
        self.rebuild_tag_groups();
    }

    fn rebuild_tag_groups(&mut self) {
        self.tag_groups.clear();
        if !self.group_by_tag {
            return;
        }

        let all_tags = self.db.get_all_tags().unwrap_or_default();

        let mut tag_map: std::collections::HashMap<String, (i64, Vec<String>)> =
            std::collections::HashMap::new();

        let mut tagged_processes: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for (process_name, tag) in &all_tags {
            tagged_processes.insert(process_name.clone());
            if let Some(s) = self
                .summaries
                .iter()
                .find(|s| &s.process_name == process_name)
            {
                let entry = tag_map.entry(tag.clone()).or_insert((0, Vec::new()));
                entry.0 += s.total_seconds;
                if !entry.1.contains(&s.process_name) {
                    entry.1.push(s.process_name.clone());
                }
            }
        }

        let mut untagged_secs: i64 = 0;
        let mut untagged_procs: Vec<String> = Vec::new();
        for s in &self.summaries {
            if !tagged_processes.contains(&s.process_name) {
                untagged_secs += s.total_seconds;
                untagged_procs.push(s.process_name.clone());
            }
        }

        let mut groups: Vec<TagGroup> = tag_map
            .into_iter()
            .map(|(tag, (total, procs))| TagGroup {
                tag,
                total_seconds: total,
                processes: procs,
            })
            .collect();

        groups.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));

        if !untagged_procs.is_empty() {
            groups.push(TagGroup {
                tag: "untagged".to_string(),
                total_seconds: untagged_secs,
                processes: untagged_procs,
            });
        }

        self.tag_groups = groups;
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

        if self.show_help {
            self.render_help(frame, theme);
        }
    }

    fn render_help(&self, frame: &mut Frame, theme: &Theme) {
        let area = frame.area();
        let w = 44.min(area.width.saturating_sub(4));
        let h = 16.min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(w)) / 2;
        let y = (area.height.saturating_sub(h)) / 2;
        let popup = Rect::new(x, y, w, h);

        let bindings = [
            ("q / Esc", "quit"),
            ("1..4", "jump to tab"),
            ("Tab / Shift+Tab", "next / previous tab"),
            ("h / Left", "previous day"),
            ("l / Right", "next day"),
            ("j / Down", "scroll down"),
            ("k / Up", "scroll up"),
            ("g", "group by tag"),
            ("r", "refresh data"),
            ("?", "toggle this help"),
        ];

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));
        for (key, desc) in &bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:20}", key), theme.key_hint()),
                Span::styled(*desc, theme.key_desc()),
            ]));
        }

        frame.render_widget(Clear, popup);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(" keybindings ")
            .padding(Padding::horizontal(1));
        let help = Paragraph::new(lines).block(block);
        frame.render_widget(help, popup);
    }
}
