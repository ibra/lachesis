use ratatui::prelude::*;

pub struct Theme {
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub muted: Color,
    pub text: Color,
    pub bar_filled: Color,
    pub bar_empty: Color,
    pub palette: [Color; 8],
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Color::Cyan,
            success: Color::Green,
            error: Color::Red,
            muted: Color::DarkGray,
            text: Color::White,
            bar_filled: Color::Cyan,
            bar_empty: Color::DarkGray,
            palette: [
                Color::Cyan,
                Color::Green,
                Color::Yellow,
                Color::Magenta,
                Color::Blue,
                Color::Red,
                Color::LightCyan,
                Color::LightGreen,
            ],
        }
    }
}

impl Theme {
    pub fn tab_active(&self) -> Style {
        Style::default().fg(self.accent).bold()
    }

    pub fn tab_inactive(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn key_hint(&self) -> Style {
        Style::default().bold()
    }

    pub fn key_desc(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn separator(&self) -> Span<'static> {
        Span::styled(" \u{2502} ", Style::default().fg(self.muted))
    }

    pub fn header_active(&self) -> Style {
        Style::default().fg(self.success).bold()
    }

    pub fn header_tracking(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn rank_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn pct_style(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn column_header(&self) -> Style {
        Style::default().fg(self.accent).bold()
    }

    pub fn active_row(&self) -> Style {
        Style::default().fg(self.success).bold()
    }

    pub fn bar_value(&self) -> Style {
        Style::default().fg(self.text).add_modifier(Modifier::BOLD)
    }

    pub fn empty_text(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn error_label(&self) -> Style {
        Style::default().fg(self.error).bold()
    }

    pub fn error_text(&self) -> Style {
        Style::default().fg(self.error)
    }
}
