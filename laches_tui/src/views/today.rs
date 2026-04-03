use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

pub fn render(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(app, frame, chunks[0], theme);

    if app.group_by_tag && !app.tag_groups.is_empty() {
        render_tag_groups(app, frame, chunks[1], theme);
    } else if app.summaries.is_empty() {
        super::render_empty(app, frame, chunks[1], theme, "top processes");
    } else {
        render_process_list(app, frame, chunks[1], theme);
    }

    render_footer(app, frame, chunks[2], theme);
}

fn render_header(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let active = laches::utils::format_duration_hm(app.active_secs);
    let idle = if app.idle_secs > 0 {
        format!(
            "  idle: {}",
            laches::utils::format_duration_hm(app.idle_secs)
        )
    } else {
        String::new()
    };
    let status = if let Some(ref p) = app.current_process {
        let title_suffix = app
            .current_window_title
            .as_ref()
            .map(|t| {
                let short = laches::utils::truncate_str(t, 30);
                format!(" \u{2014} {}", short)
            })
            .unwrap_or_default();
        Span::styled(
            format!("  |  \u{25cf} {}{}", p, title_suffix),
            theme.header_tracking(),
        )
    } else if app.daemon_running {
        Span::styled("  |  \u{25cf} daemon idle", theme.key_desc())
    } else {
        Span::styled("  |  \u{25cb} daemon stopped", theme.error_text())
    };

    let title = if app.is_viewing_today() {
        " today ".to_string()
    } else {
        format!(" {} ", app.viewing_date.format("%Y-%m-%d"))
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(format!(" active: {}", active), theme.header_active()),
        Span::styled(idle, theme.key_desc()),
        status,
    ]))
    .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(header, area);
}

fn render_process_list(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;
    if inner_height == 0 || inner_width == 0 {
        return;
    }

    let total_items = app.summaries.len();
    let scroll = app.scroll_offsets[0].min(total_items.saturating_sub(inner_height));

    let total_secs: i64 = app.summaries.iter().map(|s| s.total_seconds).sum();
    let max_secs = app
        .summaries
        .iter()
        .map(|s| s.total_seconds)
        .max()
        .unwrap_or(1)
        .max(1);

    let name_width = 20.min(inner_width.saturating_sub(22));
    let bar_width = inner_width.saturating_sub(22 + name_width).max(4);

    let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

    for (i, s) in app
        .summaries
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
            Span::styled(format!(" {:>2}. ", rank), theme.rank_style()),
            Span::raw(padded_name),
            Span::raw(" "),
            Span::styled(bar_filled, Style::default().fg(theme.bar_filled)),
            Span::styled(bar_empty, Style::default().fg(theme.bar_empty)),
            Span::raw(format!(" {:>8} ", duration)),
            Span::styled(format!("{:>3}%", pct), theme.pct_style()),
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

fn render_tag_groups(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;
    if inner_height == 0 || inner_width == 0 {
        return;
    }

    let total_items = app.tag_groups.len();
    let scroll = app.scroll_offsets[0].min(total_items.saturating_sub(inner_height));

    let max_secs = app
        .tag_groups
        .iter()
        .map(|g| g.total_seconds)
        .max()
        .unwrap_or(1)
        .max(1);
    let total_secs: i64 = app.tag_groups.iter().map(|g| g.total_seconds).sum();

    let name_width = 20.min(inner_width.saturating_sub(22));
    let bar_width = inner_width.saturating_sub(22 + name_width).max(4);

    let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

    for (i, g) in app
        .tag_groups
        .iter()
        .skip(scroll)
        .take(inner_height)
        .enumerate()
    {
        let rank = scroll + i + 1;
        let label = format!("[{}] ({})", g.tag, g.processes.len());
        let name = laches::utils::truncate_str(&label, name_width);
        let padded_name = format!("{:<width$}", name, width = name_width);

        let filled =
            ((g.total_seconds as f64 / max_secs as f64) * bar_width as f64).round() as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar_filled = "\u{2588}".repeat(filled);
        let bar_empty = "\u{2591}".repeat(empty);

        let duration = laches::utils::format_duration_hm(g.total_seconds);
        let pct = if total_secs > 0 {
            (g.total_seconds as f64 / total_secs as f64 * 100.0).round() as u32
        } else {
            0
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {:>2}. ", rank), theme.rank_style()),
            Span::styled(padded_name, Style::default().fg(theme.accent)),
            Span::raw(" "),
            Span::styled(bar_filled, Style::default().fg(theme.bar_filled)),
            Span::styled(bar_empty, Style::default().fg(theme.bar_empty)),
            Span::raw(format!(" {:>8} ", duration)),
            Span::styled(format!("{:>3}%", pct), theme.pct_style()),
        ]));
    }

    let title = format!(" by tag ({}) ", total_items);
    let block = Block::default().borders(Borders::ALL).title(title);
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);

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

fn render_footer(app: &App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let total_secs: i64 = app.summaries.iter().map(|s| s.total_seconds).sum();
    let session_count: i64 = app.summaries.iter().map(|s| s.session_count).sum();

    let footer = Paragraph::new(Line::from(vec![Span::styled(
        format!(
            " {} processes  |  {} sessions  |  {} total",
            app.summaries.len(),
            session_count,
            laches::utils::format_duration_hm(total_secs),
        ),
        theme.key_desc(),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}
