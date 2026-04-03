use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    prelude::*,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    if app.daily_totals.is_empty() || app.daily_totals.iter().all(|(_, v)| *v == 0) {
        super::render_empty(app, frame, area, theme, "trends (last 30 days)");
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let inner_width = chunks[0].width.saturating_sub(2) as usize;
    let num_bars = app.daily_totals.len();

    let bar_width = if num_bars > 0 {
        ((inner_width + 1) / num_bars).saturating_sub(1).max(1) as u16
    } else {
        1
    };

    let bar_gap = if bar_width >= 2 { 1 } else { 0 };

    let show_labels = bar_width >= 5;
    let label_interval = if show_labels {
        1
    } else if bar_width >= 3 {
        2
    } else {
        0
    };

    let bars: Vec<Bar> = app
        .daily_totals
        .iter()
        .enumerate()
        .map(|(i, (label, secs))| {
            let minutes = (*secs / 60).max(0) as u64;
            let show_this_label = label_interval > 0 && i % label_interval == 0;
            let bar_label = if show_this_label {
                if bar_width >= 5 {
                    label.clone()
                } else {
                    label.split('/').next_back().unwrap_or("").to_string()
                }
            } else {
                String::new()
            };
            Bar::default()
                .value(minutes)
                .label(Line::from(bar_label))
                .style(Style::default().fg(if *secs > 0 {
                    theme.bar_filled
                } else {
                    theme.bar_empty
                }))
        })
        .collect();

    let chart = BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" daily screentime (minutes) "),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .bar_style(Style::default().fg(theme.bar_filled))
        .value_style(theme.bar_value());

    frame.render_widget(chart, chunks[0]);

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
        "  avg: {}  |  peak: {}  |  active: {}/{} days  |  total: {}",
        laches::utils::format_duration_hm(avg_secs),
        laches::utils::format_duration_hm(max_secs),
        active_days,
        total_days,
        laches::utils::format_duration_hm(total_secs),
    ))]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(stats, chunks[1]);
}
