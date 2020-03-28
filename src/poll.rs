use asyncgit::AsyncNotification;
use crossbeam_channel::{unbounded, Receiver};
use crossterm::event::{self, Event};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

///
#[derive(Clone, Copy)]
pub enum QueueEvent {
    Tick,
    GitEvent(AsyncNotification),
    InputEvent(Event),
}

static MAX_POLL_DURATION: Duration = Duration::from_secs(2);
static MIN_POLL_DURATION: Duration = Duration::from_millis(5);
static MAX_BATCHING_DURATION: Duration = Duration::from_millis(25);
static TICK_DURATION: Duration = Duration::from_secs(5);

///
pub fn start_polling_thread() -> Receiver<Vec<QueueEvent>> {
    let (tx, rx) = unbounded();

    let tx1 = tx.clone();
    rayon_core::spawn(move || {
        let mut last_send = Instant::now();
        let mut batch = Vec::new();

        loop {
            let timeout = if batch.is_empty() {
                MAX_POLL_DURATION
            } else {
                MIN_POLL_DURATION
            };
            if let Some(e) = poll(timeout) {
                batch.push(QueueEvent::InputEvent(e));
            }

            if !batch.is_empty()
                && last_send.elapsed() > MAX_BATCHING_DURATION
            {
                tx1.send(batch).expect("send input event failed");
                batch = Vec::new();
                last_send = Instant::now();
            }
        }
    });

    rayon_core::spawn(move || loop {
        tx.send(vec![QueueEvent::Tick])
            .expect("send tick event failed");
        sleep(TICK_DURATION);
    });

    rx
}

///
fn poll(dur: Duration) -> Option<Event> {
    if event::poll(dur).unwrap() {
        let event = event::read().unwrap();
        Some(event)
    } else {
        None
    }
}
