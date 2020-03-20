mod app;
mod clear;
mod components;
mod git_status;
mod git_utils;
mod keys;
mod poll;
mod strings;
mod tui_scrolllist;
mod tui_utils;

use crate::{app::App, poll::QueueEvent};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand, Result,
};
use std::io;
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

    let mut app = App::new();

    let receiver = poll::start_polling_thread();

    loop {
        app.update();

        terminal.draw(|mut f| app.draw(&mut f))?;

        let events = receiver.recv().unwrap();
        for e in events {
            if let QueueEvent::Event(ev) = e {
                app.event(ev);
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
