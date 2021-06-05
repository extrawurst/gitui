use crate::notify_mutex::NotifyableMutex;
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use crossterm::event::{self, Event};
use std::{
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

static POLL_DURATION: Duration = Duration::from_millis(1000);

///
#[derive(Clone, Copy, Debug)]
pub enum InputState {
    Paused,
    Polling,
}

///
#[derive(Clone, Copy, Debug)]
pub enum InputEvent {
    Input(Event),
    State(InputState),
}

///
pub struct Input {
    desired_state: Arc<NotifyableMutex<bool>>,
    current_state: Arc<AtomicBool>,
    receiver: Receiver<InputEvent>,
}

impl Input {
    ///
    pub fn new() -> Self {
        let (tx, rx) = unbounded();

        let desired_state = Arc::new(NotifyableMutex::new(true));
        let current_state = Arc::new(AtomicBool::new(true));

        let arc_desired = Arc::clone(&desired_state);
        let arc_current = Arc::clone(&current_state);

        thread::spawn(move || {
            if let Err(e) =
                Self::input_loop(&arc_desired, &arc_current, &tx)
            {
                log::error!("input thread error: {}", e);
                process::abort();
            }
        });

        Self {
            receiver: rx,
            desired_state,
            current_state,
        }
    }

    ///
    pub fn receiver(&self) -> Receiver<InputEvent> {
        self.receiver.clone()
    }

    ///
    pub fn set_polling(&mut self, enabled: bool) {
        self.desired_state.set_and_notify(enabled);
    }

    fn shall_poll(&self) -> bool {
        self.desired_state.get()
    }

    ///
    pub fn is_state_changing(&self) -> bool {
        self.shall_poll()
            != self.current_state.load(Ordering::Relaxed)
    }

    fn poll(dur: Duration) -> anyhow::Result<Option<Event>> {
        if event::poll(dur)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }

    fn input_loop(
        arc_desired: &Arc<NotifyableMutex<bool>>,
        arc_current: &Arc<AtomicBool>,
        tx: &Sender<InputEvent>,
    ) -> Result<()> {
        loop {
            if arc_desired.get() {
                if !arc_current.load(Ordering::Relaxed) {
                    log::info!("input polling resumed");

                    tx.send(InputEvent::State(InputState::Polling))?;
                }
                arc_current.store(true, Ordering::Relaxed);

                if let Some(e) = Self::poll(POLL_DURATION)? {
                    tx.send(InputEvent::Input(e))?;
                }
            } else {
                if arc_current.load(Ordering::Relaxed) {
                    log::info!("input polling suspended");

                    tx.send(InputEvent::State(InputState::Paused))?;
                }

                arc_current.store(false, Ordering::Relaxed);

                arc_desired.wait(true);
            }
        }
    }
}
