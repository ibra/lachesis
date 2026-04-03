use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let non_idle: Vec<_> = app.sessions.iter().filter(|s| !s.idle).collect();

    if non_idle.is_empty() {
        super::render_empty(app, frame, area, theme, "sessions");
        return;
    }

    let max_visible = area.height.saturating_sub(4) as usize;
    if max_visible == 0 {
        return;
    }
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
                match laches::utils::session_duration_secs(&s.start_time, et) {
                    Some(secs) => laches::utils::format_duration_short(secs),
                    None => "?".to_string(),
                }
            } else {
                "\u{25cf} active".to_string()
            };

            let title = s.window_title.as_deref().unwrap_or("");
            let title_display = laches::utils::truncate_str(title, 40);

            let time_range = format!("{}\u{2013}{}", start, end);

            let style = if s.end_time.is_none() {
                theme.active_row()
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

    let header = Row::new(vec![
        Cell::from("time").style(theme.column_header()),
        Cell::from("process").style(theme.column_header()),
        Cell::from("duration").style(theme.column_header()),
        Cell::from("window title").style(theme.column_header()),
    ])
    .bottom_margin(1);

    let widths = [
        Constraint::Length(11),
        Constraint::Length(22),
        Constraint::Length(12),
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

    if non_idle.len() > max_visible {
        let mut scrollbar_state =
            ScrollbarState::new(non_idle.len().saturating_sub(max_visible)).position(scroll);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}
