use crossterm::event::{self, Event};
use std::time::Duration;

///
pub enum PollResult {
    Timeout,
    Event(Event),
}

///
pub fn poll(dur: Duration) -> PollResult {
    if event::poll(dur).unwrap() {
        // It's guaranteed that read() wont block if `poll` returns `Ok(true)`
        let event = event::read().unwrap();

        PollResult::Event(event)
    } else {
        PollResult::Timeout
    }
}
