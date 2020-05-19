#![forbid(unsafe_code)]
// #![warn(clippy::cargo)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod app;
mod components;
mod keys;
mod poll;
mod queue;
mod spinner;
mod strings;
mod tabs;
mod ui;
mod version;

use crate::ui::style::Mode;
use crate::{app::App, poll::QueueEvent};
use asyncgit::AsyncNotification;
use backtrace::Backtrace;
use crossbeam_channel::{tick, unbounded, Receiver, Select};
use crossterm::{
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand, Result,
};
use io::Write;
use log::error;
use scopeguard::defer;
use scopetime::scope_time;
use simplelog::{Config, LevelFilter, WriteLogger};
use spinner::Spinner;
use std::path::PathBuf;
use std::{
    env, fs,
    fs::File,
    io, panic,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

static TICK_INTERVAL: Duration = Duration::from_secs(5);
static SPINNER_INTERVAL: Duration = Duration::from_millis(50);

fn main() -> Result<()> {
    setup_logging();

    if invalid_path() {
        eprintln!("invalid git path\nplease run gitui inside of a git repository");
        return Ok(());
    }

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    defer! {
        io::stdout().execute(LeaveAlternateScreen).unwrap();
        disable_raw_mode().unwrap();
    }

    set_panic_handlers();

    let mut terminal = start_terminal(io::stdout())?;

    let (tx_git, rx_git) = unbounded();

    let mode = match env::var("GITUI_LIGHT") {
        Ok(_) => Mode::Light,
        _ => Mode::Dark,
    };

    let mut app = App::new(&tx_git, mode);

    let rx_input = poll::start_polling_thread();
    let ticker = tick(TICK_INTERVAL);
    let spinner_ticker = tick(SPINNER_INTERVAL);

    app.update();
    draw(&mut terminal, &mut app)?;

    let mut spinner = Spinner::new();

    loop {
        let events: Vec<QueueEvent> = select_event(
            &rx_input,
            &rx_git,
            &ticker,
            &spinner_ticker,
        );

        {
            scope_time!("loop");

            let mut needs_draw = true;

            for e in events {
                match e {
                    QueueEvent::InputEvent(ev) => app.event(ev),
                    QueueEvent::Tick => app.update(),
                    QueueEvent::GitEvent(ev) => app.update_git(ev),
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

fn draw<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    terminal.draw(|mut f| app.draw(&mut f))
}

fn invalid_path() -> bool {
    !asyncgit::is_repo(asyncgit::CWD)
}

fn select_event(
    rx_input: &Receiver<Vec<QueueEvent>>,
    rx_git: &Receiver<AsyncNotification>,
    rx_ticker: &Receiver<Instant>,
    rx_spinner: &Receiver<Instant>,
) -> Vec<QueueEvent> {
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
        _ => panic!("unknown select source"),
    }
    .unwrap();

    events
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

#[must_use]
pub fn get_app_config_path() -> PathBuf {
    let mut path = dirs::cache_dir().unwrap();
    path.push("gitui");
    fs::create_dir_all(&path).unwrap();
    path
}

fn setup_logging() {
    if env::var("GITUI_LOGGING").is_ok() {
        let mut path = get_app_config_path();
        path.push("gitui.log");

        let _ = WriteLogger::init(
            LevelFilter::Trace,
            Config::default(),
            File::create(path).unwrap(),
        );
    }
}

fn set_panic_handlers() {
    // regular panic handler
    panic::set_hook(Box::new(|e| {
        let backtrace = Backtrace::new();
        error!("panic: {:?}\ntrace:\n{:?}", e, backtrace);
    }));

    // global threadpool
    rayon_core::ThreadPoolBuilder::new()
        .panic_handler(|e| {
            error!("thread panic: {:?}", e);
            panic!(e)
        })
        .num_threads(4)
        .build_global()
        .unwrap();
}
