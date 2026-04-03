use crate::app::App;
use crate::theme::Theme;
use chrono::Timelike;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use laches::db::TIMESTAMP_FORMAT;

struct TimelineEntry {
    process_name: String,
    start: chrono::NaiveDateTime,
    end: chrono::NaiveDateTime,
}

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let sessions: Vec<_> = app.sessions.iter().filter(|s| !s.idle).collect();

    if sessions.is_empty() {
        super::render_empty(app, frame, area, theme, "timeline");
        return;
    }

    let now = if app.is_viewing_today() {
        chrono::Local::now().naive_local()
    } else {
        app.viewing_date
            .succ_opt()
            .unwrap_or(app.viewing_date)
            .and_hms_opt(0, 0, 0)
            .unwrap()
    };

    let entries: Vec<TimelineEntry> = sessions
        .iter()
        .rev()
        .filter_map(|s| {
            let start =
                chrono::NaiveDateTime::parse_from_str(&s.start_time, TIMESTAMP_FORMAT).ok()?;
            let end = s
                .end_time
                .as_ref()
                .and_then(|e| chrono::NaiveDateTime::parse_from_str(e, TIMESTAMP_FORMAT).ok())
                .unwrap_or(now);
            Some(TimelineEntry {
                process_name: s.process_name.clone(),
                start,
                end,
            })
        })
        .collect();

    if entries.is_empty() {
        return;
    }

    let mut color_map: Vec<(String, Color)> = Vec::new();
    for entry in &entries {
        if !color_map
            .iter()
            .any(|(name, _)| name == &entry.process_name)
        {
            let color = theme.palette[color_map.len() % theme.palette.len()];
            color_map.push((entry.process_name.clone(), color));
        }
    }

    let legend_items_per_row = |width: usize| -> usize {
        if width == 0 {
            return 1;
        }
        let avg_item_width = 16;
        (width / avg_item_width).max(1)
    };
    let inner_w = area.width.saturating_sub(2) as usize;
    let items_per_row = legend_items_per_row(inner_w);
    let legend_rows = color_map.len().div_ceil(items_per_row).max(1);
    let legend_height = (legend_rows as u16 + 2).min(area.height.saturating_sub(5));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(legend_height)])
        .split(area);

    let inner_width = chunks[0].width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return;
    }

    let range_start = entries.first().map(|e| e.start).unwrap();
    let range_secs = (now - range_start).num_seconds().max(1) as f64;

    let mut col_colors: Vec<Option<Color>> = vec![None; inner_width];
    for entry in &entries {
        let color = color_map
            .iter()
            .find(|(name, _)| name == &entry.process_name)
            .map(|(_, c)| *c)
            .unwrap_or(Color::White);

        let start_col = ((entry.start - range_start).num_seconds().max(0) as f64 / range_secs
            * inner_width as f64) as usize;
        let end_col = ((entry.end - range_start).num_seconds().max(0) as f64 / range_secs
            * inner_width as f64)
            .ceil() as usize;

        let start_col = start_col.min(inner_width);
        let end_col = end_col.min(inner_width);

        for slot in col_colors.iter_mut().take(end_col).skip(start_col) {
            if slot.is_none() {
                *slot = Some(color);
            }
        }
    }

    let bar_row: Vec<Span> = col_colors
        .iter()
        .map(|c| match c {
            Some(color) => Span::styled("\u{2588}", Style::default().fg(*color)),
            None => Span::styled(" ", Style::default().fg(theme.muted)),
        })
        .collect();

    let fill_height = chunks[0].height.saturating_sub(2) as usize;
    let bar_rows = fill_height.saturating_sub(1).clamp(1, 3);

    let mut lines: Vec<Line> = Vec::new();
    for _ in 0..bar_rows {
        lines.push(Line::from(bar_row.clone()));
    }

    let start_label = range_start.format("%H:%M").to_string();
    let end_label = now.format("%H:%M").to_string();

    let total_hours = (range_secs / 3600.0).ceil() as usize;
    let mut hour_markers: Vec<(usize, String)> = Vec::new();
    if total_hours > 1 {
        let first_hour = range_start.time().hour() + 1;
        for h in 0..total_hours {
            let hour = (first_hour + h as u32) % 24;
            let hour_time = range_start.date().and_hms_opt(hour, 0, 0);
            if let Some(ht) = hour_time {
                let secs_from_start = (ht - range_start).num_seconds();
                if secs_from_start > 0 && (secs_from_start as f64) < range_secs {
                    let col =
                        (secs_from_start as f64 / range_secs * inner_width as f64).round() as usize;
                    if col > 0 && col < inner_width.saturating_sub(4) {
                        hour_markers.push((col, format!("{:02}:00", hour)));
                    }
                }
            }
        }
    }

    let mut axis_chars = vec![' '; inner_width];
    let start_bytes: Vec<char> = start_label.chars().collect();
    for (i, ch) in start_bytes.iter().enumerate() {
        if i < inner_width {
            axis_chars[i] = *ch;
        }
    }
    let end_bytes: Vec<char> = end_label.chars().collect();
    let end_start = inner_width.saturating_sub(end_bytes.len());
    for (i, ch) in end_bytes.iter().enumerate() {
        axis_chars[end_start + i] = *ch;
    }
    for (col, label) in &hour_markers {
        let label_chars: Vec<char> = label.chars().collect();
        let start_pos = col.saturating_sub(label_chars.len() / 2);
        let mut can_place = true;
        for i in 0..label_chars.len() {
            let pos = start_pos + i;
            if pos >= inner_width || axis_chars[pos] != ' ' {
                can_place = false;
                break;
            }
        }
        if can_place {
            for (i, ch) in label_chars.iter().enumerate() {
                axis_chars[start_pos + i] = *ch;
            }
        }
    }

    let axis_str: String = axis_chars.into_iter().collect();
    lines.push(Line::from(Span::styled(axis_str, theme.key_desc())));

    while lines.len() < fill_height {
        lines.push(Line::from(""));
    }

    let timeline =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" timeline "));
    frame.render_widget(timeline, chunks[0]);

    let mut legend_lines: Vec<Line> = Vec::new();
    let mut current_spans: Vec<Span> = Vec::new();
    let mut current_width = 0;

    for (name, color) in &color_map {
        let item_width = name.len() + 4;
        if current_width + item_width > inner_w && !current_spans.is_empty() {
            legend_lines.push(Line::from(std::mem::take(&mut current_spans)));
            current_width = 0;
        }
        current_spans.push(Span::styled("\u{2588} ", Style::default().fg(*color)));
        current_spans.push(Span::raw(format!("{}  ", name)));
        current_width += item_width;
    }
    if !current_spans.is_empty() {
        legend_lines.push(Line::from(current_spans));
    }

    let legend = Paragraph::new(legend_lines)
        .block(Block::default().borders(Borders::ALL).title(" legend "));
    frame.render_widget(legend, chunks[1]);
}
