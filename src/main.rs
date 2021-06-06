#![forbid(unsafe_code)]
#![deny(
    unused_imports,
    unused_must_use,
    dead_code,
    unstable_name_collisions,
    unused_assignments
)]
#![deny(clippy::all, clippy::perf, clippy::nursery, clippy::pedantic)]
#![deny(clippy::filetype_is_file)]
#![deny(clippy::cargo)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::match_like_matches_macro)]
#![deny(clippy::needless_update)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::multiple_crate_versions)]
//TODO:
// #![deny(clippy::expect_used)]

mod app;
mod args;
mod bug_report;
mod clipboard;
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

use crate::{app::App, args::process_cmdline};
use anyhow::{bail, Result};
use asyncgit::AsyncNotification;
use backtrace::Backtrace;
use crossbeam_channel::{tick, unbounded, Receiver, Select};
use crossterm::{
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use input::{Input, InputEvent, InputState};
use keys::KeyConfig;
use profiler::Profiler;
use scopeguard::defer;
use scopetime::scope_time;
use spinner::Spinner;
use std::{
    io::{self, Write},
    panic, process,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use ui::style::Theme;

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
    let cliargs = process_cmdline()?;

    let _profiler = Profiler::new();

    if !valid_path()? {
        eprintln!("invalid path\nplease run gitui inside of a non-bare git repository");
        return Ok(());
    }

    let key_config = KeyConfig::init(KeyConfig::get_config_file()?)
        .map_err(|e| eprintln!("KeyConfig loading error: {}", e))
        .unwrap_or_default();
    let theme = Theme::init(cliargs.theme)
        .map_err(|e| eprintln!("Theme loading error: {}", e))
        .unwrap_or_default();

    setup_terminal()?;
    defer! {
        shutdown_terminal();
    }

    set_panic_handlers()?;

    let mut terminal = start_terminal(io::stdout())?;

    let (tx_git, rx_git) = unbounded();

    let input = Input::new();

    let rx_input = input.receiver();
    let ticker = tick(TICK_INTERVAL);
    let spinner_ticker = tick(SPINNER_INTERVAL);

    let mut app = App::new(&tx_git, input, theme, key_config);

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
                    app.event(ev)?;
                }
                QueueEvent::Tick => app.update()?,
                QueueEvent::GitEvent(ev)
                    if ev != AsyncNotification::FinishUnchanged =>
                {
                    app.update_git(ev)?;
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

fn shutdown_terminal() {
    let leave_screen =
        io::stdout().execute(LeaveAlternateScreen).map(|_f| ());

    if let Err(e) = leave_screen {
        eprintln!("leave_screen failed:\n{}", e);
    }

    let leave_raw_mode = disable_raw_mode();

    if let Err(e) = leave_raw_mode {
        eprintln!("leave_raw_mode failed:\n{}", e);
    }
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
            log::error!("failed to draw: {:?}", e);
        }
    })?;

    Ok(())
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
        _ => bail!("unknown select source"),
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

fn set_panic_handlers() -> Result<()> {
    // regular panic handler
    panic::set_hook(Box::new(|e| {
        let backtrace = Backtrace::new();
        //TODO: create macro to do both in one
        log::error!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
        eprintln!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
        shutdown_terminal();
    }));

    // global threadpool
    rayon_core::ThreadPoolBuilder::new()
        .panic_handler(|e| {
            let backtrace = Backtrace::new();
            //TODO: create macro to do both in one
            log::error!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
            eprintln!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
            shutdown_terminal();
            process::abort();
        })
        .num_threads(4)
        .build_global()?;

    Ok(())
}
