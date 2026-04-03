use crate::app::App;
use crate::theme::Theme;
use crate::views;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Padding, Paragraph, Tabs},
};

const TAB_TITLES: [&str; 4] = ["today", "timeline", "trends", "sessions"];

pub fn render(app: &App, frame: &mut Frame, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let date_str = if app.is_viewing_today() {
        format!("today ({})", app.viewing_date.format("%a %b %d"))
    } else {
        app.viewing_date.format("%a %b %d, %Y").to_string()
    };
    let title = format!(" lachesis \u{2500} {} ", date_str);
    let tabs = Tabs::new(TAB_TITLES.iter().map(|t| Line::from(*t)))
        .block(Block::default().borders(Borders::ALL).title(title))
        .select(app.tab)
        .style(theme.tab_inactive())
        .highlight_style(theme.tab_active())
        .divider(theme.separator());
    frame.render_widget(tabs, chunks[0]);

    match app.tab {
        0 => views::today::render(app, frame, chunks[1], theme),
        1 => views::timeline::render(app, frame, chunks[1], theme),
        2 => views::trends::render(app, frame, chunks[1], theme),
        3 => views::sessions::render(app, frame, chunks[1], theme),
        _ => {}
    }

    let footer = if let Some(ref err) = app.last_error {
        Line::from(vec![
            Span::styled(" ERROR ", theme.error_label()),
            Span::styled(err.as_str(), theme.error_text()),
        ])
    } else {
        let sep = theme.separator();
        let time_str = chrono::Local::now().format("%H:%M").to_string();
        Line::from(vec![
            Span::styled(" q", theme.key_hint()),
            Span::styled(" quit", theme.key_desc()),
            sep.clone(),
            Span::styled("tab", theme.key_hint()),
            Span::styled(" switch", theme.key_desc()),
            sep.clone(),
            Span::styled("h/l", theme.key_hint()),
            Span::styled(" day", theme.key_desc()),
            sep.clone(),
            Span::styled("j/k", theme.key_hint()),
            Span::styled(" scroll", theme.key_desc()),
            sep.clone(),
            Span::styled("r", theme.key_hint()),
            Span::styled(" refresh", theme.key_desc()),
            sep,
            Span::styled(time_str, theme.key_desc()),
        ])
    };
    frame.render_widget(footer, chunks[2]);

    if app.show_help {
        render_help(frame, theme);
    }
}

fn render_help(frame: &mut Frame, theme: &Theme) {
    let area = frame.area();
    let w = 44.min(area.width.saturating_sub(4));
    let h = 16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let popup = Rect::new(x, y, w, h);

    let bindings = [
        ("q / Esc", "quit"),
        ("1..4", "jump to tab"),
        ("Tab / Shift+Tab", "next / previous tab"),
        ("h / Left", "previous day"),
        ("l / Right", "next day"),
        ("j / Down", "scroll down"),
        ("k / Up", "scroll up"),
        ("g", "group by tag"),
        ("r", "refresh data"),
        ("?", "toggle this help"),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    for (key, desc) in &bindings {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:20}", key), theme.key_hint()),
            Span::styled(*desc, theme.key_desc()),
        ]));
    }

    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" keybindings ")
        .padding(Padding::horizontal(1));
    let help = Paragraph::new(lines).block(block);
    frame.render_widget(help, popup);
}
