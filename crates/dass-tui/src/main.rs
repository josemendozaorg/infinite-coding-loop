use std::io;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use anyhow::Result;
use std::time::Duration;

mod app;
mod ui;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create App State
    let mut app = App::default();

    // Run Loop
    let res = run_app(&mut terminal, &mut app).await;

    // Restore Terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if crossterm::event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Quit via Char 'q' or Ctrl+C
                if let KeyCode::Char('q') = key.code {
                    app.on_key('q');
                } else if let KeyCode::Char('c') = key.code {
                    if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                       app.on_key('q');
                    }
                } else if let KeyCode::Char(c) = key.code {
                     app.on_key(c);
                }
            }
        }

        app.on_tick();

        if app.should_quit {
            return Ok(());
        }
    }
}
