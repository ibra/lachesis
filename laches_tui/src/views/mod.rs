pub mod sessions;
pub mod timeline;
pub mod today;
pub mod trends;

use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn render_empty(app: &App, frame: &mut Frame, area: Rect, theme: &Theme, title: &str) {
    let lines = if app.daemon_running {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No data recorded for this period.",
                theme.empty_text(),
            )),
            Line::from(Span::styled(
                "  The daemon is running \u{2014} data will appear shortly.",
                theme.empty_text(),
            )),
        ]
    } else if app.is_viewing_today() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No data recorded. The daemon is not running.",
                Style::default().fg(theme.error),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Run ", theme.empty_text()),
                Span::styled("laches start", theme.header_tracking()),
                Span::styled(" to begin tracking.", theme.empty_text()),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No data recorded for this day.",
                theme.empty_text(),
            )),
        ]
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title));
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
