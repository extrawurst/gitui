use crate::{error::Result, sync, AsyncNotification, CWD};
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
    pub fn count(&mut self) -> Result<usize> {
        Ok(self.current.lock()?.len())
    }

    ///
    pub fn get_slice(
        &self,
        start_index: usize,
        amount: usize,
    ) -> Result<Vec<Oid>> {
        let list = self.current.lock()?;
        let list_len = list.len();
        let min = start_index.min(list_len);
        let max = min + amount;
        let max = max.min(list_len);
        Ok(Vec::from_iter(list[min..max].iter().cloned()))
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }

    ///
    pub fn fetch(&mut self) -> Result<()> {
        if !self.is_pending() {
            self.clear()?;

            let arc_current = Arc::clone(&self.current);
            let sender = self.sender.clone();
            let arc_pending = Arc::clone(&self.pending);

            rayon_core::spawn(move || {
                scope_time!("async::revlog");
                arc_pending.store(true, Ordering::Relaxed);
                AsyncLog::fetch_helper(arc_current, &sender)
                    .expect("failed to fetch");
                arc_pending.store(false, Ordering::Relaxed);
                Self::notify(&sender);
            });
        }
        Ok(())
    }

    fn fetch_helper(
        arc_current: Arc<Mutex<Vec<Oid>>>,
        sender: &Sender<AsyncNotification>,
    ) -> Result<()> {
        let mut entries = Vec::with_capacity(LIMIT_COUNT);
        let r = repo(CWD)?;
        let mut walker = LogWalker::new(&r);
        loop {
            entries.clear();
            let res_is_err =
                walker.read(&mut entries, LIMIT_COUNT).is_err();

            if !res_is_err {
                let mut current = arc_current.lock()?;
                current.extend(entries.iter());
            }

            if res_is_err || entries.len() <= 1 {
                break;
            } else {
                Self::notify(&sender);
            }
        }

        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        self.current.lock()?.clear();
        Ok(())
    }

    fn notify(sender: &Sender<AsyncNotification>) {
        sender.send(AsyncNotification::Log).expect("error sending");
    }
}
