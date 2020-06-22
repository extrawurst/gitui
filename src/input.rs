use crossbeam_channel::{unbounded, Receiver};
use crossterm::event::{self, Event};
use std::{
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
    desired_state: Arc<AtomicBool>,
    receiver: Receiver<InputEvent>,
}

impl Input {
    ///
    pub fn new() -> Self {
        let (tx, rx) = unbounded();

        let desired_state = Arc::new(AtomicBool::new(true));

        let arc_desired = Arc::clone(&desired_state);

        thread::spawn(move || {
            let mut current_state = true;
            loop {
                //TODO: use condvar to not busy wait
                if arc_desired.load(Ordering::Relaxed) {
                    if !current_state {
                        tx.send(InputEvent::State(
                            InputState::Polling,
                        ))
                        .expect("send failed");
                    }
                    current_state = true;

                    if let Some(e) = Self::poll(POLL_DURATION)
                        .expect("failed to pull events.")
                    {
                        tx.send(InputEvent::Input(e))
                            .expect("send input event failed");
                    }
                } else {
                    if current_state {
                        tx.send(InputEvent::State(
                            InputState::Paused,
                        ))
                        .expect("send failed");
                    }
                    current_state = false;
                }
            }
        });

        Self {
            receiver: rx,
            desired_state,
        }
    }

    ///
    pub fn receiver(&self) -> Receiver<InputEvent> {
        self.receiver.clone()
    }

    ///
    pub fn set_polling(&mut self, enabled: bool) {
        self.desired_state.store(enabled, Ordering::Relaxed);
    }

    fn poll(dur: Duration) -> anyhow::Result<Option<Event>> {
        if event::poll(dur)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}
