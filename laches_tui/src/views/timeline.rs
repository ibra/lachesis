use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// Color palette for process timeline blocks.
const PALETTE: [Color; 8] = [
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Magenta,
    Color::Blue,
    Color::Red,
    Color::LightCyan,
    Color::LightGreen,
];

use laches::db::TIMESTAMP_FORMAT;

/// Parsed session with pre-computed timestamps for efficient timeline rendering.
struct TimelineEntry {
    process_name: String,
    start: chrono::NaiveDateTime,
    end: chrono::NaiveDateTime,
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let sessions: Vec<_> = app.today_sessions.iter().filter(|s| !s.idle).collect();

    if sessions.is_empty() {
        let empty = Paragraph::new(" no sessions today.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" timeline "));
        frame.render_widget(empty, area);
        return;
    }

    let now = chrono::Local::now().naive_local();

    // parse all session timestamps once upfront
    let entries: Vec<TimelineEntry> = sessions
        .iter()
        .rev() // chronological order
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

    // build color map using Vec to preserve insertion order (deterministic legend)
    let mut color_map: Vec<(String, Color)> = Vec::new();
    for entry in &entries {
        if !color_map
            .iter()
            .any(|(name, _)| name == &entry.process_name)
        {
            let color = PALETTE[color_map.len() % PALETTE.len()];
            color_map.push((entry.process_name.clone(), color));
        }
    }

    // compute legend height dynamically based on content
    let legend_items_per_row = |width: usize| -> usize {
        // each legend item: "█ name  " ~= name.len() + 4
        if width == 0 {
            return 1;
        }
        let avg_item_width = 16; // reasonable average
        (width / avg_item_width).max(1)
    };
    let inner_w = area.width.saturating_sub(2) as usize;
    let items_per_row = legend_items_per_row(inner_w);
    let legend_rows = ((color_map.len() + items_per_row - 1) / items_per_row).max(1);
    let legend_height = (legend_rows as u16 + 2).min(area.height.saturating_sub(5)); // +2 for borders

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(legend_height)])
        .split(area);

    // --- timeline rendering ---
    let inner_width = chunks[0].width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return;
    }

    let range_start = entries.first().map(|e| e.start).unwrap();
    let range_secs = (now - range_start).num_seconds().max(1) as f64;

    // pre-compute: for each column, find the session index (O(entries) total)
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

        for col in start_col..end_col {
            if col_colors[col].is_none() {
                col_colors[col] = Some(color);
            }
        }
    }

    // build timeline spans from pre-computed array
    let timeline_spans: Vec<Span> = col_colors
        .iter()
        .map(|c| match c {
            Some(color) => Span::styled("\u{2588}", Style::default().fg(*color)),
            None => Span::styled("\u{2591}", Style::default().fg(Color::DarkGray)),
        })
        .collect();

    // time axis labels — exact alignment
    let start_label = range_start.format("%H:%M").to_string();
    let end_label = now.format("%H:%M").to_string();
    let gap = inner_width.saturating_sub(start_label.len() + end_label.len());
    let axis_line = Line::from(vec![
        Span::styled(start_label, Style::default().fg(Color::DarkGray)),
        Span::raw(" ".repeat(gap)),
        Span::styled(end_label, Style::default().fg(Color::DarkGray)),
    ]);

    let mut lines = vec![Line::from(timeline_spans), axis_line];

    // pad to fill area
    let fill_height = chunks[0].height.saturating_sub(2) as usize;
    while lines.len() < fill_height {
        lines.push(Line::from(""));
    }

    let timeline =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" timeline "));
    frame.render_widget(timeline, chunks[0]);

    // --- legend rendering ---
    let mut legend_lines: Vec<Line> = Vec::new();
    let mut current_spans: Vec<Span> = Vec::new();
    let mut current_width = 0;

    for (name, color) in &color_map {
        let item_width = name.len() + 4; // "█ name  "
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
