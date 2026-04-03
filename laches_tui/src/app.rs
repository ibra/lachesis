use laches::db::{date_range_for_day, last_n_days_range, Database, ProcessSummary, Session};
use std::path::PathBuf;

const TAB_COUNT: usize = 4;

pub struct Insights {
    pub yesterday_secs: i64,
    pub week_secs: i64,
    pub last_week_secs: i64,
    pub avg_7d: i64,
    pub avg_30d: i64,
    pub streak: u32,
    pub top_week_process: Option<String>,
    pub top_week_secs: i64,
}

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
    pub earliest_date: Option<chrono::NaiveDate>,

    pub scroll_offsets: [usize; TAB_COUNT],

    pub summaries: Vec<ProcessSummary>,
    pub sessions: Vec<Session>,
    pub active_secs: i64,
    pub idle_secs: i64,
    pub insights: Insights,
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
            earliest_date: None,
            scroll_offsets: [0; TAB_COUNT],
            summaries: Vec::new(),
            sessions: Vec::new(),
            active_secs: 0,
            idle_secs: 0,
            insights: Insights {
                yesterday_secs: 0,
                week_secs: 0,
                last_week_secs: 0,
                avg_7d: 0,
                avg_30d: 0,
                streak: 0,
                top_week_process: None,
                top_week_secs: 0,
            },
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
            if let Some(earliest) = self.earliest_date {
                if d < earliest {
                    return;
                }
            }
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

        self.compute_insights(&db_totals);

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

        if self.earliest_date.is_none() {
            self.earliest_date = self
                .db
                .get_earliest_session_date()
                .ok()
                .flatten()
                .and_then(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());
        }

        self.rebuild_tag_groups();
    }

    fn compute_insights(&mut self, daily_map: &std::collections::HashMap<String, i64>) {
        let vd = self.viewing_date;

        let prev = vd.pred_opt().unwrap_or(vd);
        let prev_key = prev.format("%Y-%m-%d").to_string();
        self.insights.yesterday_secs = daily_map.get(&prev_key).copied().unwrap_or_else(|| {
            let (s, e) = date_range_for_day(&prev_key).unwrap_or_default();
            self.db.query_total_active_seconds(&s, &e).unwrap_or(0)
        });

        let (w7s, w7e) = last_n_days_range(7);
        let week_total = self.db.query_total_active_seconds(&w7s, &w7e).unwrap_or(0);
        self.insights.week_secs = week_total;
        self.insights.avg_7d = week_total / 7;

        let (w14s, _) = last_n_days_range(14);
        let two_week_total = self.db.query_total_active_seconds(&w14s, &w7s).unwrap_or(0);
        self.insights.last_week_secs = two_week_total;

        let (m30s, m30e) = last_n_days_range(30);
        let month_total = self
            .db
            .query_total_active_seconds(&m30s, &m30e)
            .unwrap_or(0);
        self.insights.avg_30d = month_total / 30;

        let mut streak: u32 = 0;
        let mut check = vd;
        loop {
            let key = check.format("%Y-%m-%d").to_string();
            let has_data = daily_map.get(&key).copied().unwrap_or_else(|| {
                let (s, e) = date_range_for_day(&key).unwrap_or_default();
                self.db.query_total_active_seconds(&s, &e).unwrap_or(0)
            });
            if has_data > 0 {
                streak += 1;
            } else {
                break;
            }
            match check.pred_opt() {
                Some(d) => check = d,
                None => break,
            }
        }
        self.insights.streak = streak;

        let week_summaries = self
            .db
            .query_process_summaries(&w7s, &w7e, None)
            .unwrap_or_default();
        if let Some(top) = week_summaries.first() {
            self.insights.top_week_process = Some(top.process_name.clone());
            self.insights.top_week_secs = top.total_seconds;
        } else {
            self.insights.top_week_process = None;
            self.insights.top_week_secs = 0;
        }
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
}
