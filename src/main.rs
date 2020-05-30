#![forbid(unsafe_code)]
#![deny(clippy::cargo)]
//TODO: remove once crossterm upgraded to current mio:
//https://github.com/crossterm-rs/crossterm/issues/432
#![allow(clippy::cargo::multiple_crate_versions)]
#![deny(clippy::pedantic)]
#![deny(clippy::result_unwrap_used)]
#![deny(clippy::panic)]
#![allow(clippy::module_name_repetitions)]

mod app;
mod cmdbar;
mod components;
mod keys;
mod poll;
mod queue;
mod spinner;
mod strings;
mod tabs;
mod ui;
mod version;

use crate::{app::App, poll::QueueEvent};
use anyhow::{anyhow, Result};
use asyncgit::AsyncNotification;
use backtrace::Backtrace;
use clap::{
    crate_authors, crate_description, crate_name, crate_version,
    App as ClapApp, Arg,
};
use crossbeam_channel::{tick, unbounded, Receiver, Select};
use crossterm::{
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use scopeguard::defer;
use scopetime::scope_time;
use simplelog::{Config, LevelFilter, WriteLogger};
use spinner::Spinner;
use std::{
    env, fs,
    fs::File,
    io::{self, Write},
    panic,
    path::PathBuf,
    process,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

static TICK_INTERVAL: Duration = Duration::from_secs(5);
static SPINNER_INTERVAL: Duration = Duration::from_millis(50);

fn main() -> Result<()> {
    process_cmdline()?;

    if !valid_path()? {
        eprintln!("invalid git path\nplease run gitui inside of a valid git (non-bare) repository");
        return Ok(());
    }

    setup_terminal()?;
    defer! {
        shutdown_terminal().expect("shutdown failed");
    }

    set_panic_handlers()?;

    let mut terminal = start_terminal(io::stdout())?;

    let (tx_git, rx_git) = unbounded();

    let mut app = App::new(&tx_git);

    let rx_input = poll::start_polling_thread();
    let ticker = tick(TICK_INTERVAL);
    let spinner_ticker = tick(SPINNER_INTERVAL);

    app.update()?;
    draw(&mut terminal, &mut app)?;

    let mut spinner = Spinner::default();

    loop {
        let events: Vec<QueueEvent> = select_event(
            &rx_input,
            &rx_git,
            &ticker,
            &spinner_ticker,
        )?;

        {
            scope_time!("loop");

            let mut needs_draw = true;

            for e in events {
                match e {
                    QueueEvent::InputEvent(ev) => app.event(ev)?,
                    QueueEvent::Tick => app.update()?,
                    QueueEvent::GitEvent(ev) => app.update_git(ev)?,
                    QueueEvent::SpinnerUpdate => {
                        needs_draw = false;
                        spinner.update()
                    }
                }
            }

            if needs_draw {
                draw(&mut terminal, &mut app)?;
            }

            spinner.draw(&mut terminal, app.any_work_pending())?;

            if app.is_quit() {
                break;
            }
        }
    }

    Ok(())
}

fn setup_terminal() -> Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

fn shutdown_terminal() -> Result<()> {
    io::stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn draw<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    terminal.draw(|mut f| {
        if let Err(e) = app.draw(&mut f) {
            log::error!("failed to draw: {:?}", e)
        }
    })
}

fn valid_path() -> Result<bool> {
    Ok(asyncgit::sync::is_repo(asyncgit::CWD)
        && !asyncgit::sync::is_bare_repo(asyncgit::CWD)?)
}

fn select_event(
    rx_input: &Receiver<Vec<QueueEvent>>,
    rx_git: &Receiver<AsyncNotification>,
    rx_ticker: &Receiver<Instant>,
    rx_spinner: &Receiver<Instant>,
) -> Result<Vec<QueueEvent>> {
    let mut events: Vec<QueueEvent> = Vec::new();

    let mut sel = Select::new();

    sel.recv(rx_input);
    sel.recv(rx_git);
    sel.recv(rx_ticker);
    sel.recv(rx_spinner);

    let oper = sel.select();
    let index = oper.index();

    match index {
        0 => oper.recv(rx_input).map(|inputs| events.extend(inputs)),
        1 => oper
            .recv(rx_git)
            .map(|ev| events.push(QueueEvent::GitEvent(ev))),
        2 => oper
            .recv(rx_ticker)
            .map(|_| events.push(QueueEvent::Tick)),
        3 => oper
            .recv(rx_spinner)
            .map(|_| events.push(QueueEvent::SpinnerUpdate)),
        _ => return Err(anyhow!("unknown select source")),
    }?;

    Ok(events)
}

fn start_terminal<W: Write>(
    buf: W,
) -> io::Result<Terminal<CrosstermBackend<W>>> {
    let backend = CrosstermBackend::new(buf);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    Ok(terminal)
}

fn get_app_config_path() -> Result<PathBuf> {
    let mut path = dirs::cache_dir()
        .ok_or_else(|| anyhow!("failed to find os cache dir."))?;

    path.push("gitui");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn setup_logging() -> Result<()> {
    let mut path = get_app_config_path()?;
    path.push("gitui.log");

    let _ = WriteLogger::init(
        LevelFilter::Trace,
        Config::default(),
        File::create(path)?,
    );

    Ok(())
}

fn process_cmdline() -> Result<()> {
    let app = ClapApp::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("logging")
                .help("Stores logging output into a cache directory")
                .short("l")
                .long("logging"),
        );

    let arg_matches = app.get_matches();
    if arg_matches.is_present("logging") {
        setup_logging()?;
    }

    Ok(())
}

fn set_panic_handlers() -> Result<()> {
    // regular panic handler
    panic::set_hook(Box::new(|e| {
        let backtrace = Backtrace::new();
        log::error!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
        shutdown_terminal().expect("shutdown failed inside panic");
        eprintln!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
    }));

    // global threadpool
    rayon_core::ThreadPoolBuilder::new()
        .panic_handler(|e| {
            let backtrace = Backtrace::new();
            log::error!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
            shutdown_terminal()
                .expect("shutdown failed inside panic");
            eprintln!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
            process::abort();
        })
        .num_threads(4)
        .build_global()?;

    Ok(())
}
