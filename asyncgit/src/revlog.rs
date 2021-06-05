use crate::{
    error::Result,
    sync::{utils::repo, CommitId, LogWalker},
    AsyncNotification, CWD,
};
use crossbeam_channel::Sender;
use git2::Oid;
use scopetime::scope_time;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

///
#[derive(PartialEq)]
pub enum FetchStatus {
    /// previous fetch still running
    Pending,
    /// no change expected
    NoChange,
    /// new walk was started
    Started,
}

///
pub struct AsyncLog {
    current: Arc<Mutex<Vec<CommitId>>>,
    sender: Sender<AsyncNotification>,
    pending: Arc<AtomicBool>,
    background: Arc<AtomicBool>,
}

static LIMIT_COUNT: usize = 3000;
static SLEEP_FOREGROUND: Duration = Duration::from_millis(2);
static SLEEP_BACKGROUND: Duration = Duration::from_millis(1000);

impl AsyncLog {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Vec::new())),
            sender: sender.clone(),
            pending: Arc::new(AtomicBool::new(false)),
            background: Arc::new(AtomicBool::new(false)),
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
    ) -> Result<Vec<CommitId>> {
        let list = self.current.lock()?;
        let list_len = list.len();
        let min = start_index.min(list_len);
        let max = min + amount;
        let max = max.min(list_len);
        Ok(list[min..max].to_vec())
    }

    ///
    pub fn position(&self, id: CommitId) -> Result<Option<usize>> {
        let list = self.current.lock()?;
        let position = list.iter().position(|&x| x == id);

        Ok(position)
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }

    ///
    pub fn set_background(&mut self) {
        self.background.store(true, Ordering::Relaxed);
    }

    ///
    fn current_head(&self) -> Result<CommitId> {
        Ok(self
            .current
            .lock()?
            .first()
            .map_or(Oid::zero().into(), |f| *f))
    }

    ///
    fn head_changed(&self) -> Result<bool> {
        if let Ok(head) = repo(CWD)?.head() {
            if let Some(head) = head.target() {
                return Ok(head != self.current_head()?.into());
            }
        }
        Ok(false)
    }

    ///
    pub fn fetch(&mut self) -> Result<FetchStatus> {
        self.background.store(false, Ordering::Relaxed);

        if self.is_pending() {
            return Ok(FetchStatus::Pending);
        }

        if !self.head_changed()? {
            return Ok(FetchStatus::NoChange);
        }

        self.clear()?;

        let arc_current = Arc::clone(&self.current);
        let sender = self.sender.clone();
        let arc_pending = Arc::clone(&self.pending);
        let arc_background = Arc::clone(&self.background);

        self.pending.store(true, Ordering::Relaxed);

        rayon_core::spawn(move || {
            scope_time!("async::revlog");

            Self::fetch_helper(
                &arc_current,
                &arc_background,
                &sender,
            )
            .expect("failed to fetch");

            arc_pending.store(false, Ordering::Relaxed);

            Self::notify(&sender);
        });

        Ok(FetchStatus::Started)
    }

    fn fetch_helper(
        arc_current: &Arc<Mutex<Vec<CommitId>>>,
        arc_background: &Arc<AtomicBool>,
        sender: &Sender<AsyncNotification>,
    ) -> Result<()> {
        let mut entries = Vec::with_capacity(LIMIT_COUNT);
        let r = repo(CWD)?;
        let mut walker = LogWalker::new(&r, LIMIT_COUNT)?;
        loop {
            entries.clear();
            let res_is_err = walker.read(&mut entries).is_err();

            if !res_is_err {
                let mut current = arc_current.lock()?;
                current.extend(entries.iter());
            }

            if res_is_err || entries.len() <= 1 {
                break;
            }
            Self::notify(sender);

            let sleep_duration =
                if arc_background.load(Ordering::Relaxed) {
                    SLEEP_BACKGROUND
                } else {
                    SLEEP_FOREGROUND
                };
            thread::sleep(sleep_duration);
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
