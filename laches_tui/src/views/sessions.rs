use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let non_idle: Vec<_> = app.today_sessions.iter().filter(|s| !s.idle).collect();

    if non_idle.is_empty() {
        let empty = Paragraph::new("no sessions today.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" sessions "));
        frame.render_widget(empty, area);
        return;
    }

    // account for borders (2) + header row (1) + header bottom_margin (1) = 4
    let max_visible = area.height.saturating_sub(4) as usize;
    let scroll = app.scroll_offsets[3].min(non_idle.len().saturating_sub(max_visible));

    let rows: Vec<Row> = non_idle
        .iter()
        .skip(scroll)
        .take(max_visible)
        .map(|s| {
            let start = s.start_time.get(11..16).unwrap_or("?");
            let end = s
                .end_time
                .as_ref()
                .and_then(|e| e.get(11..16))
                .unwrap_or("now");

            let duration = if let Some(ref et) = s.end_time {
                let st = chrono::NaiveDateTime::parse_from_str(&s.start_time, "%Y-%m-%dT%H:%M:%S");
                let en = chrono::NaiveDateTime::parse_from_str(et, "%Y-%m-%dT%H:%M:%S");
                if let (Ok(st), Ok(en)) = (st, en) {
                    let secs = (en - st).num_seconds().max(0);
                    laches::utils::format_duration_short(secs)
                } else {
                    "?".to_string()
                }
            } else {
                "active".to_string()
            };

            let title = s.window_title.as_deref().unwrap_or("");
            let title_display = laches::utils::truncate_str(title, 35);

            let time_range = format!("{}-{}", start, end);

            let style = if s.end_time.is_none() {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(time_range),
                Cell::from(s.process_name.clone()),
                Cell::from(duration),
                Cell::from(title_display),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["time", "process", "duration", "window title"])
        .style(Style::default().fg(Color::Cyan).bold())
        .bottom_margin(1);

    let widths = [
        Constraint::Length(11),
        Constraint::Length(22),
        Constraint::Length(10),
        Constraint::Min(20),
    ];

    let scroll_info = if non_idle.len() > max_visible {
        format!(
            " sessions ({}-{} of {}) ",
            scroll + 1,
            (scroll + max_visible).min(non_idle.len()),
            non_idle.len()
        )
    } else {
        format!(" sessions ({}) ", non_idle.len())
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(scroll_info));

    frame.render_widget(table, area);
}
