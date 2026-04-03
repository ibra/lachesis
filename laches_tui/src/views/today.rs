use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // header with total time and current process
    let active = format_duration(app.today_active);
    let idle = if app.today_idle > 0 {
        format!("  idle: {}", format_duration(app.today_idle))
    } else {
        String::new()
    };
    let tracking = app
        .current_process
        .as_ref()
        .map(|p| format!("  |  tracking: {}", p))
        .unwrap_or_default();

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("active: {}", active),
            Style::default().fg(Color::Green).bold(),
        ),
        Span::styled(idle, Style::default().fg(Color::DarkGray)),
        Span::styled(tracking, Style::default().fg(Color::Cyan)),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" today "));
    frame.render_widget(header, chunks[0]);

    // bar chart of top processes
    if app.today_summaries.is_empty() {
        let empty =
            Paragraph::new("no tracked data for today. start the daemon with `laches start`.")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let bars: Vec<Bar> = app
        .today_summaries
        .iter()
        .take(15)
        .map(|s| {
            let minutes = (s.total_seconds / 60).max(1) as u64;
            let label = laches::utils::truncate_str(&s.process_name, 18);
            Bar::default()
                .value(minutes)
                .label(Line::from(label))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" top processes (minutes) "),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(3)
        .bar_gap(1)
        .direction(Direction::Horizontal)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White).bold());

    frame.render_widget(chart, chunks[1]);
}

fn format_duration(seconds: i64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}
