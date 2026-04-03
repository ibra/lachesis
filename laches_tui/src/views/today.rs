use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(app, frame, chunks[0]);

    if app.today_summaries.is_empty() {
        let empty =
            Paragraph::new(" no tracked data for today. start the daemon with `laches start`.")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" top processes "),
                );
        frame.render_widget(empty, chunks[1]);
    } else {
        render_process_list(app, frame, chunks[1]);
    }

    render_footer(app, frame, chunks[2]);
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let active = laches::utils::format_duration_hm(app.today_active);
    let idle = if app.today_idle > 0 {
        format!(
            "  idle: {}",
            laches::utils::format_duration_hm(app.today_idle)
        )
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
            format!(" active: {}", active),
            Style::default().fg(Color::Green).bold(),
        ),
        Span::styled(idle, Style::default().fg(Color::DarkGray)),
        Span::styled(tracking, Style::default().fg(Color::Cyan)),
    ]))
    .block(Block::default().borders(Borders::ALL).title(" today "));
    frame.render_widget(header, area);
}

fn render_process_list(app: &App, frame: &mut Frame, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;
    if inner_height == 0 || inner_width == 0 {
        return;
    }

    let total_items = app.today_summaries.len();
    let scroll = app.scroll_offsets[0].min(total_items.saturating_sub(inner_height));

    let total_secs: i64 = app.today_summaries.iter().map(|s| s.total_seconds).sum();
    let max_secs = app
        .today_summaries
        .iter()
        .map(|s| s.total_seconds)
        .max()
        .unwrap_or(1)
        .max(1);

    // calculate how much space we have for the bar
    // format: " {rank:>2}. {name:<name_w} {bar} {duration:>8} {pct:>4}%"
    // fixed chars: 2(rank) + 2(". ") + 1(" ") + 8(duration) + 1(" ") + 4(pct) + 1("%") + 1(" ") = 20
    let name_width = 20.min(inner_width.saturating_sub(22));
    let bar_width = inner_width.saturating_sub(22 + name_width).max(4);

    let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

    for (i, s) in app
        .today_summaries
        .iter()
        .skip(scroll)
        .take(inner_height)
        .enumerate()
    {
        let rank = scroll + i + 1;
        let name = laches::utils::truncate_str(&s.process_name, name_width);
        let padded_name = format!("{:<width$}", name, width = name_width);

        let filled =
            ((s.total_seconds as f64 / max_secs as f64) * bar_width as f64).round() as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar_filled = "\u{2588}".repeat(filled);
        let bar_empty = "\u{2591}".repeat(empty);

        let duration = laches::utils::format_duration_hm(s.total_seconds);
        let pct = if total_secs > 0 {
            (s.total_seconds as f64 / total_secs as f64 * 100.0).round() as u32
        } else {
            0
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:>2}. ", rank),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(padded_name),
            Span::raw(" "),
            Span::styled(bar_filled, Style::default().fg(Color::Cyan)),
            Span::styled(bar_empty, Style::default().fg(Color::DarkGray)),
            Span::raw(format!(" {:>8} ", duration)),
            Span::styled(format!("{:>3}%", pct), Style::default().fg(Color::DarkGray)),
        ]));
    }

    let title = if total_items > inner_height {
        format!(
            " top processes ({}-{} of {}) ",
            scroll + 1,
            (scroll + inner_height).min(total_items),
            total_items
        )
    } else {
        format!(" top processes ({}) ", total_items)
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);

    // scrollbar
    if total_items > inner_height {
        let mut scrollbar_state =
            ScrollbarState::new(total_items.saturating_sub(inner_height)).position(scroll);
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

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let total_secs: i64 = app.today_summaries.iter().map(|s| s.total_seconds).sum();
    let session_count: i64 = app.today_summaries.iter().map(|s| s.session_count).sum();

    let footer = Paragraph::new(Line::from(vec![Span::styled(
        format!(
            " {} processes  |  {} sessions  |  {} total",
            app.today_summaries.len(),
            session_count,
            laches::utils::format_duration_hm(total_secs),
        ),
        Style::default().fg(Color::DarkGray),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}
