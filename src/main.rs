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
use simplelog::*;
use std::{env, fs, fs::File, io};
use tui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    setup_logging();
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
        let events = receiver.recv().unwrap();
        for e in events {
            if let QueueEvent::InputEvent(ev) = e {
                app.event(ev);
            } else {
                app.update();
            }
        }

        terminal.draw(|mut f| app.draw(&mut f))?;

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

fn setup_logging() {
    if env::var("GITUI_LOGGING").is_ok() {
        let mut path = dirs::home_dir().unwrap();
        path.push(".gitui");
        path.push("gitui.log");
        fs::create_dir(path.parent().unwrap()).unwrap_or_default();

        let _ = WriteLogger::init(
            LevelFilter::Trace,
            Config::default(),
            File::create(path).unwrap(),
        );
    }
}
