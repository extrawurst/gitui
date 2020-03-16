mod app;
mod poll;
mod git_utils;

use app::App;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand, Result,
};
use std::{io, time::Duration};
use tui::{backend::CrosstermBackend, Terminal};
use poll::PollResult;

fn main() -> Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    terminal.clear()?;

    let mut app = App::default();
    app.fetch_status();

    loop {
        terminal.draw(|mut f| app.draw(&mut f))?;

        if let PollResult::Event(e) = poll::poll(Duration::from_millis(200)) {
            app.event(e);
        }

        if app.is_quit() {
            break;
        }
    }

    io::stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
