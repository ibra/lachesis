use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::collections::HashMap;

/// Assign a color to each process for the timeline.
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

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(5)])
        .split(area);

    let sessions: Vec<_> = app
        .today_sessions
        .iter()
        .filter(|s| !s.idle)
        .rev() // chronological order
        .collect();

    if sessions.is_empty() {
        let empty = Paragraph::new("no sessions today.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" timeline "));
        frame.render_widget(empty, area);
        return;
    }

    // build color map
    let mut color_map: HashMap<&str, Color> = HashMap::new();
    let mut color_idx = 0;
    for s in &sessions {
        if !color_map.contains_key(s.process_name.as_str()) {
            color_map.insert(&s.process_name, PALETTE[color_idx % PALETTE.len()]);
            color_idx += 1;
        }
    }

    // render timeline as colored blocks across the available width
    // each character represents a time slice of the day
    let inner_width = chunks[0].width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return;
    }

    // find the time range (first session start to now)
    let first_start = sessions.first().and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(&s.start_time, "%Y-%m-%dT%H:%M:%S").ok()
    });
    let now = chrono::Local::now().naive_local();

    let (range_start, range_secs) = match first_start {
        Some(fs) => {
            let secs = (now - fs).num_seconds().max(1);
            (fs, secs as f64)
        }
        None => return,
    };

    // build the timeline row
    let mut timeline_spans: Vec<Span> = Vec::with_capacity(inner_width);
    for col in 0..inner_width {
        let col_time = range_start
            + chrono::Duration::seconds((col as f64 / inner_width as f64 * range_secs) as i64);

        // find which session this time falls in
        let mut found = false;
        for s in &sessions {
            let ss = chrono::NaiveDateTime::parse_from_str(&s.start_time, "%Y-%m-%dT%H:%M:%S");
            let se = s
                .end_time
                .as_ref()
                .and_then(|e| chrono::NaiveDateTime::parse_from_str(e, "%Y-%m-%dT%H:%M:%S").ok())
                .unwrap_or(now);

            if let Ok(ss) = ss {
                if col_time >= ss && col_time < se {
                    let color = color_map
                        .get(s.process_name.as_str())
                        .copied()
                        .unwrap_or(Color::White);
                    timeline_spans.push(Span::styled("\u{2588}", Style::default().fg(color)));
                    found = true;
                    break;
                }
            }
        }
        if !found {
            timeline_spans.push(Span::styled(
                "\u{2591}",
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    // time axis labels
    let start_label = range_start.format("%H:%M").to_string();
    let end_label = now.format("%H:%M").to_string();

    let mut lines = vec![
        Line::from(timeline_spans),
        Line::from(vec![
            Span::raw(start_label),
            Span::raw(" ".repeat(inner_width.saturating_sub(10))),
            Span::raw(end_label),
        ]),
    ];

    // pad to fill area
    while lines.len() < chunks[0].height.saturating_sub(2) as usize {
        lines.push(Line::from(""));
    }

    let timeline =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" timeline "));
    frame.render_widget(timeline, chunks[0]);

    // legend
    let mut legend_spans: Vec<Span> = Vec::new();
    for (name, color) in &color_map {
        legend_spans.push(Span::styled("\u{2588} ", Style::default().fg(*color)));
        legend_spans.push(Span::raw(format!("{}  ", name)));
    }

    let legend = Paragraph::new(Line::from(legend_spans))
        .block(Block::default().borders(Borders::ALL).title(" legend "));
    frame.render_widget(legend, chunks[1]);
}
