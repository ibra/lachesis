mod app;
mod theme;
mod views;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{io, time::Duration};
use theme::Theme;

fn main() -> io::Result<()> {
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("lachesis"),
        None => {
            eprintln!("error: failed to get configuration directory");
            std::process::exit(1);
        }
    };

    let machine_id = laches::config::get_machine_id(&config_dir);
    let data_dir = laches::config::data_dir(&config_dir);
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).ok();
    }

    let db_path = laches::config::machine_db_path(&config_dir, &machine_id);
    let db = match laches::db::Database::open(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    // set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // catch panics and restore terminal
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let theme = Theme::default();
    let mut app = App::new(&db);
    let result = run(&mut terminal, &mut app, &theme);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    theme: &Theme,
) -> io::Result<()> {
    app.refresh_data();

    loop {
        terminal.draw(|f| app.render(f, theme))?;

        // poll for events with a timeout so we can refresh data periodically
        if event::poll(Duration::from_secs(5))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('1') => app.set_tab(0),
                    KeyCode::Char('2') => app.set_tab(1),
                    KeyCode::Char('3') => app.set_tab(2),
                    KeyCode::Char('4') => app.set_tab(3),
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.prev_tab(),
                    KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
                    KeyCode::Char('r') => app.refresh_data(),
                    _ => {}
                }
            }
        } else {
            // timeout - refresh data
            app.refresh_data();
        }
    }
}
