mod app;
mod clear;
mod commit;
mod git_status;
mod git_utils;
mod poll;
mod tui_utils;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand, Result,
};
use poll::PollResult;
use std::{io, time::Duration};
use tui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    enable_raw_mode()?;
    io::stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    terminal.clear()?;

    let mut app = App::default();

    loop {
        app.update();

        terminal.draw(|mut f| app.draw(&mut f))?;

        loop {
            if let PollResult::Event(e) =
                poll::poll(Duration::from_millis(10))
            {
                app.event(e);
            } else {
                break;
            }
        }

        if app.is_quit() {
            break;
        }
    }

    io::stdout()
        .execute(LeaveAlternateScreen)?
        .execute(DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}
