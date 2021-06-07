use crate::{
    error::Result,
    sync::{self, CommitId},
    AsyncGitNotification, StatusItem, CWD,
};
use crossbeam_channel::Sender;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

type ResultType = Vec<StatusItem>;
struct Request<R, A>(R, A);

///
pub struct AsyncCommitFiles {
    current: Arc<Mutex<Option<Request<CommitId, ResultType>>>>,
    sender: Sender<AsyncGitNotification>,
    pending: Arc<AtomicUsize>,
}

impl AsyncCommitFiles {
    ///
    pub fn new(sender: &Sender<AsyncGitNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(None)),
            sender: sender.clone(),
            pending: Arc::new(AtomicUsize::new(0)),
        }
    }

    ///
    pub fn current(
        &mut self,
    ) -> Result<Option<(CommitId, ResultType)>> {
        let c = self.current.lock()?;

        c.as_ref()
            .map_or(Ok(None), |c| Ok(Some((c.0, c.1.clone()))))
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
    }

    ///
    pub fn fetch(&mut self, id: CommitId) -> Result<()> {
        if self.is_pending() {
            return Ok(());
        }

        log::trace!("request: {}", id.to_string());

        {
            let current = self.current.lock()?;
            if let Some(c) = &*current {
                if c.0 == id {
                    return Ok(());
                }
            }
        }

        let arc_current = Arc::clone(&self.current);
        let sender = self.sender.clone();
        let arc_pending = Arc::clone(&self.pending);

        self.pending.fetch_add(1, Ordering::Relaxed);

        rayon_core::spawn(move || {
            Self::fetch_helper(id, &arc_current)
                .expect("failed to fetch");

            arc_pending.fetch_sub(1, Ordering::Relaxed);

            sender
                .send(AsyncGitNotification::CommitFiles)
                .expect("error sending");
        });

        Ok(())
    }

    fn fetch_helper(
        id: CommitId,
        arc_current: &Arc<
            Mutex<Option<Request<CommitId, ResultType>>>,
        >,
    ) -> Result<()> {
        let res = sync::get_commit_files(CWD, id)?;

        log::trace!(
            "get_commit_files: {} ({})",
            id.to_string(),
            res.len()
        );

        {
            let mut current = arc_current.lock()?;
            *current = Some(Request(id, res));
        }

        Ok(())
    }
}
