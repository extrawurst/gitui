use crate::{sync, AsyncNotification, CWD};
use crossbeam_channel::Sender;
use git2::Oid;
use scopetime::scope_time;
use std::{
    iter::FromIterator,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use sync::{utils::repo, LogWalker};

///
pub struct AsyncLog {
    current: Arc<Mutex<Vec<Oid>>>,
    sender: Sender<AsyncNotification>,
    pending: Arc<AtomicBool>,
}

static LIMIT_COUNT: usize = 1000;

impl AsyncLog {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Vec::new())),
            sender,
            pending: Arc::new(AtomicBool::new(false)),
        }
    }

    ///
    pub fn count(&mut self) -> usize {
        self.current.lock().unwrap().len()
    }

    ///
    pub fn get_slice(&self, limit: usize) -> Vec<Oid> {
        let list = self.current.lock().unwrap();
        let max = limit.min(list.len());
        Vec::from_iter(list[0..max].iter().cloned())
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }

    ///
    pub fn fetch(&mut self) {
        if !self.is_pending() {
            self.clear();

            let arc_current = Arc::clone(&self.current);
            let sender = self.sender.clone();
            let arc_pending = Arc::clone(&self.pending);
            rayon_core::spawn(move || {
                arc_pending.store(true, Ordering::Relaxed);

                scope_time!("async::revlog");

                let mut entries = Vec::with_capacity(LIMIT_COUNT);
                let r = repo(CWD);
                let mut walker = LogWalker::new(&r);
                loop {
                    entries.clear();
                    walker.read(&mut entries, LIMIT_COUNT);

                    let is_done = entries.len() <= 1;

                    {
                        let mut current = arc_current.lock().unwrap();
                        current.extend(entries.iter());
                    }

                    if is_done {
                        break;
                    } else {
                        Self::notify(&sender);
                    }
                }

                arc_pending.store(false, Ordering::Relaxed);

                Self::notify(&sender);
            });
        }
    }

    fn clear(&mut self) {
        self.current.lock().unwrap().clear();
    }

    fn notify(sender: &Sender<AsyncNotification>) {
        sender.send(AsyncNotification::Log).expect("error sending");
    }
}
