use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    if app.daily_totals.is_empty() || app.daily_totals.iter().all(|(_, v)| *v == 0) {
        let empty = Paragraph::new("no trend data available yet.")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" trends (last 30 days) "),
            );
        frame.render_widget(empty, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let bars: Vec<Bar> = app
        .daily_totals
        .iter()
        .map(|(label, secs)| {
            let minutes = (*secs / 60).max(0) as u64;
            Bar::default()
                .value(minutes)
                .label(Line::from(label.as_str()))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" daily screentime (minutes) "),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(3)
        .bar_gap(0)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White));

    frame.render_widget(chart, chunks[0]);

    // summary stats
    let total_days = app.daily_totals.len() as i64;
    let total_secs: i64 = app.daily_totals.iter().map(|(_, s)| s).sum();
    let avg_secs = if total_days > 0 {
        total_secs / total_days
    } else {
        0
    };
    let max_secs = app.daily_totals.iter().map(|(_, s)| *s).max().unwrap_or(0);
    let active_days = app.daily_totals.iter().filter(|(_, s)| *s > 0).count();

    let stats = Paragraph::new(Line::from(vec![Span::raw(format!(
        "  avg: {}  |  peak: {}  |  active: {}/{} days",
        laches::utils::format_duration_hm(avg_secs),
        laches::utils::format_duration_hm(max_secs),
        active_days,
        total_days,
    ))]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(stats, chunks[1]);
}
