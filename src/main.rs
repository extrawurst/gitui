#![forbid(unsafe_code)]
#![deny(clippy::cargo)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::multiple_crate_versions)]

mod app;
mod cmdbar;
mod components;
mod input;
mod keys;
mod notify_mutex;
mod profiler;
mod queue;
mod spinner;
mod strings;
mod tabs;
mod ui;
mod version;

use crate::app::App;
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
use input::{Input, InputEvent, InputState};
use profiler::Profiler;
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
static SPINNER_INTERVAL: Duration = Duration::from_millis(80);

///
#[derive(Clone, Copy)]
pub enum QueueEvent {
    Tick,
    SpinnerUpdate,
    GitEvent(AsyncNotification),
    InputEvent(InputEvent),
}

fn main() -> Result<()> {
    process_cmdline()?;

    let _profiler = Profiler::new();

    if !valid_path()? {
        eprintln!("invalid path\nplease run gitui inside of a non-bare git repository");
        return Ok(());
    }

    // TODO: Remove this when upgrading from v0.8.x is unlikely
    // Only run this migration on macOS, as it's the only platform where the config needs to be moved
    if cfg!(target_os = "macos") {
        migrate_config()?;
    }

    setup_terminal()?;
    defer! {
        shutdown_terminal().expect("shutdown failed");
    }

    set_panic_handlers()?;

    let mut terminal = start_terminal(io::stdout())?;

    let (tx_git, rx_git) = unbounded();

    let input = Input::new();

    let rx_input = input.receiver();
    let ticker = tick(TICK_INTERVAL);
    let spinner_ticker = tick(SPINNER_INTERVAL);

    let mut app = App::new(&tx_git, input);

    let mut spinner = Spinner::default();
    let mut first_update = true;

    loop {
        let event = if first_update {
            first_update = false;
            QueueEvent::Tick
        } else {
            select_event(
                &rx_input,
                &rx_git,
                &ticker,
                &spinner_ticker,
            )?
        };

        {
            if let QueueEvent::SpinnerUpdate = event {
                spinner.update();
                spinner.draw(&mut terminal)?;
                continue;
            }

            scope_time!("loop");

            match event {
                QueueEvent::InputEvent(ev) => {
                    if let InputEvent::State(InputState::Polling) = ev
                    {
                        //Note: external ed closed, we need to re-hide cursor
                        terminal.hide_cursor()?;
                    }
                    app.event(ev)?
                }
                QueueEvent::Tick => app.update()?,
                QueueEvent::GitEvent(ev)
                    if ev != AsyncNotification::FinishUnchanged =>
                {
                    app.update_git(ev)?
                }
                QueueEvent::GitEvent(..) => (),
                QueueEvent::SpinnerUpdate => unreachable!(),
            }

            draw(&mut terminal, &app)?;

            spinner.set_state(app.any_work_pending());
            spinner.draw(&mut terminal)?;

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
    app: &App,
) -> io::Result<()> {
    if app.requires_redraw() {
        terminal.resize(terminal.size()?)?;
    }

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
    rx_input: &Receiver<InputEvent>,
    rx_git: &Receiver<AsyncNotification>,
    rx_ticker: &Receiver<Instant>,
    rx_spinner: &Receiver<Instant>,
) -> Result<QueueEvent> {
    let mut sel = Select::new();

    sel.recv(rx_input);
    sel.recv(rx_git);
    sel.recv(rx_ticker);
    sel.recv(rx_spinner);

    let oper = sel.select();
    let index = oper.index();

    let ev = match index {
        0 => oper.recv(rx_input).map(QueueEvent::InputEvent),
        1 => oper.recv(rx_git).map(QueueEvent::GitEvent),
        2 => oper.recv(rx_ticker).map(|_| QueueEvent::Tick),
        3 => oper.recv(rx_spinner).map(|_| QueueEvent::SpinnerUpdate),
        _ => return Err(anyhow!("unknown select source")),
    }?;

    Ok(ev)
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

fn get_app_cache_path() -> Result<PathBuf> {
    let mut path = dirs::cache_dir()
        .ok_or_else(|| anyhow!("failed to find os cache dir."))?;

    path.push("gitui");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn get_app_config_path() -> Result<PathBuf> {
    let mut path = dirs::config_dir()
        .ok_or_else(|| anyhow!("failed to find os config dir."))?;

    path.push("gitui");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn migrate_config() -> Result<()> {
    let mut path = dirs::preference_dir().ok_or_else(|| {
        anyhow!("failed to find os preference dir.")
    })?;

    path.push("gitui");
    if !path.exists() {
        return Ok(());
    }

    let config_path = get_app_config_path()?;
    let entries = path.read_dir()?.flatten();
    for entry in entries {
        let mut config_path = config_path.clone();
        config_path.push(entry.file_name());
        fs::rename(entry.path(), config_path)?;
    }

    let _ = fs::remove_dir(path);

    Ok(())
}

fn setup_logging() -> Result<()> {
    let mut path = get_app_cache_path()?;
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
        )
        .arg(
            Arg::with_name("directory")
                .help("Set the working directory")
                .short("d")
                .long("directory")
                .takes_value(true),
        );

    let arg_matches = app.get_matches();
    if arg_matches.is_present("logging") {
        setup_logging()?;
    }

    if arg_matches.is_present("directory") {
        let directory =
            arg_matches.value_of("directory").unwrap_or(".");
        env::set_current_dir(directory)?;
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
