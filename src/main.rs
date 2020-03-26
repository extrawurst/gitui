mod app;
mod components;
mod keys;
mod poll;
mod strings;
mod ui;

use crate::{app::App, poll::QueueEvent};
use crossbeam_channel::{select, unbounded};
use crossterm::{
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand, Result,
};
use log::error;
use scopetime::scope_time;
use simplelog::*;
use std::{env, fs, fs::File, io};
use tui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    setup_logging();
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    terminal.clear()?;

    let (tx_git, rx_git) = unbounded();

    let mut app = App::new(tx_git);

    rayon_core::ThreadPoolBuilder::new()
        .panic_handler(|e| {
            error!("thread panic: {:?}", e);
            panic!(e)
        })
        .build_global()
        .unwrap();

    let rx_input = poll::start_polling_thread();

    app.update();

    loop {
        let mut events: Vec<QueueEvent> = Vec::new();
        select! {
            recv(rx_input) -> inputs => events.append(&mut inputs.unwrap()),
            recv(rx_git) -> ev => events.push(QueueEvent::GitEvent(ev.unwrap())),
        }

        {
            scope_time!("loop");

            for e in events {
                match e {
                    QueueEvent::InputEvent(ev) => app.event(ev),
                    QueueEvent::Tick => app.update(),
                    QueueEvent::GitEvent(ev) => app.update_git(ev),
                }
            }

            terminal.draw(|mut f| app.draw(&mut f))?;

            if app.is_quit() {
                break;
            }
        }
    }

    io::stdout().execute(LeaveAlternateScreen)?;
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
