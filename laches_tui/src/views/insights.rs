use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    if app.active_secs == 0 && app.insights.week_secs == 0 {
        super::render_empty(app, frame, area, theme, "insights");
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Min(0),
        ])
        .split(area);

    render_comparisons(app, frame, chunks[0], theme);
    render_averages(app, frame, chunks[1], theme);
    render_details(app, frame, chunks[2], theme);
}

fn fmt_delta(secs: i64, theme: &Theme) -> Span<'static> {
    let dur = laches::utils::format_duration_hm(secs.unsigned_abs() as i64);
    if secs > 0 {
        Span::styled(format!("+{}", dur), Style::default().fg(theme.error))
    } else if secs < 0 {
        Span::styled(
            format!("\u{2212}{}", dur),
            Style::default().fg(theme.success),
        )
    } else {
        Span::styled("same".to_string(), theme.key_desc())
    }
}

fn render_comparisons(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let today_str = laches::utils::format_duration_hm(app.active_secs);
    let yest_str = laches::utils::format_duration_hm(app.insights.yesterday_secs);
    let delta = app.active_secs - app.insights.yesterday_secs;

    let lines = vec![
        Line::from(vec![
            Span::styled("  this day      ", theme.key_hint()),
            Span::styled(
                format!("{:<12}", today_str),
                Style::default().fg(theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("  previous day  ", theme.key_hint()),
            Span::styled(format!("{:<12}", yest_str), theme.key_desc()),
            fmt_delta(delta, theme),
        ]),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" vs previous day ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_averages(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let ins = &app.insights;
    let week_str = laches::utils::format_duration_hm(ins.week_secs);
    let last_week_str = laches::utils::format_duration_hm(ins.last_week_secs);
    let week_delta = ins.week_secs - ins.last_week_secs;
    let avg7 = laches::utils::format_duration_hm(ins.avg_7d);
    let avg30 = laches::utils::format_duration_hm(ins.avg_30d);

    let lines = vec![
        Line::from(vec![
            Span::styled("  this week     ", theme.key_hint()),
            Span::styled(
                format!("{:<12}", week_str),
                Style::default().fg(theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("  last week     ", theme.key_hint()),
            Span::styled(format!("{:<12}", last_week_str), theme.key_desc()),
            fmt_delta(week_delta, theme),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  7d avg        ", theme.key_hint()),
            Span::styled(format!("{:<12}", avg7), theme.key_desc()),
            Span::styled("30d avg  ", theme.key_hint()),
            Span::styled(avg30, theme.key_desc()),
        ]),
        Line::from(""),
    ];

    let block = Block::default().borders(Borders::ALL).title(" weekly ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn render_details(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let ins = &app.insights;
    let mut lines: Vec<Line> = Vec::new();

    let streak_str = if ins.streak > 0 {
        format!(
            "{} day{}",
            ins.streak,
            if ins.streak == 1 { "" } else { "s" }
        )
    } else {
        "none".to_string()
    };
    lines.push(Line::from(vec![
        Span::styled("  streak        ", theme.key_hint()),
        Span::styled(streak_str, Style::default().fg(theme.accent)),
    ]));

    lines.push(Line::from(""));

    if let Some(ref proc) = ins.top_week_process {
        let dur = laches::utils::format_duration_hm(ins.top_week_secs);
        lines.push(Line::from(vec![
            Span::styled("  top (7d)      ", theme.key_hint()),
            Span::styled(format!("{:<20} {}", proc, dur), theme.key_desc()),
        ]));
    }

    if let Some(proc) = app.summaries.first() {
        let dur = laches::utils::format_duration_hm(proc.total_seconds);
        lines.push(Line::from(vec![
            Span::styled("  top (today)   ", theme.key_hint()),
            Span::styled(
                format!("{:<20} {}", proc.process_name, dur),
                theme.key_desc(),
            ),
        ]));
    }

    let block = Block::default().borders(Borders::ALL).title(" details ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}
