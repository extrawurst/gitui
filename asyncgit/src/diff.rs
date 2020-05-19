use crate::{
    error::Result, hash, sync, AsyncNotification, FileDiff, CWD,
};
use crossbeam_channel::Sender;
use log::trace;
use std::{
    hash::Hash,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

///
#[derive(Default, Hash, Clone, PartialEq)]
pub struct DiffParams(pub String, pub bool);

struct Request<R, A>(R, Option<A>);

#[derive(Default, Clone)]
struct LastResult<P, R> {
    params: P,
    hash: u64,
    result: R,
}

///
pub struct AsyncDiff {
    current: Arc<Mutex<Request<u64, FileDiff>>>,
    last: Arc<Mutex<Option<LastResult<DiffParams, FileDiff>>>>,
    sender: Sender<AsyncNotification>,
    pending: Arc<AtomicUsize>,
}

impl AsyncDiff {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Request(0, None))),
            last: Arc::new(Mutex::new(None)),
            sender,
            pending: Arc::new(AtomicUsize::new(0)),
        }
    }

    ///
    pub fn last(&mut self) -> Result<Option<(DiffParams, FileDiff)>> {
        let last = self.last.lock()?;

        Ok(match last.clone() {
            Some(res) => Some((res.params, res.result)),
            None => None,
        })
    }

    ///
    pub fn refresh(&mut self) -> Result<()> {
        if let Ok(Some(param)) = self.get_last_param() {
            self.clear_current()?;
            self.request(param)?;
        }
        Ok(())
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
    }

    ///
    pub fn request(
        &mut self,
        params: DiffParams,
    ) -> Result<Option<FileDiff>> {
        trace!("request");

        let hash = hash(&params);

        {
            let mut current = self.current.lock()?;

            if current.0 == hash {
                return Ok(current.1.clone());
            }

            current.0 = hash;
            current.1 = None;
        }

        let arc_current = Arc::clone(&self.current);
        let arc_last = Arc::clone(&self.last);
        let sender = self.sender.clone();
        let arc_pending = Arc::clone(&self.pending);
        rayon_core::spawn(move || {
            arc_pending.fetch_add(1, Ordering::Relaxed);

            let notify = AsyncDiff::get_diff_helper(
                params,
                arc_last,
                arc_current,
                hash,
            )
            .expect("error getting diff");

            arc_pending.fetch_sub(1, Ordering::Relaxed);

            if notify {
                sender
                    .send(AsyncNotification::Diff)
                    .expect("error sending diff");
            }
        });

        Ok(None)
    }

    fn get_diff_helper(
        params: DiffParams,
        arc_last: Arc<
            Mutex<Option<LastResult<DiffParams, FileDiff>>>,
        >,
        arc_current: Arc<Mutex<Request<u64, FileDiff>>>,
        hash: u64,
    ) -> Result<bool> {
        let res =
            sync::diff::get_diff(CWD, params.0.clone(), params.1)?;

        let mut notify = false;
        {
            let mut current = arc_current.lock()?;
            if current.0 == hash {
                current.1 = Some(res.clone());
                notify = true;
            }
        }

        {
            let mut last = arc_last.lock()?;
            *last = Some(LastResult {
                result: res,
                hash,
                params,
            });
        }

        Ok(notify)
    }

    fn get_last_param(&self) -> Result<Option<DiffParams>> {
        Ok(self.last.lock()?.clone().map(|e| e.params))
    }

    fn clear_current(&mut self) -> Result<()> {
        let mut current = self.current.lock()?;
        current.0 = 0;
        current.1 = None;
        Ok(())
    }
}
