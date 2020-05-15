use crate::{
    error::Returns, hash, sync, AsyncNotification, StatusItem, CWD,
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
use sync::status::StatusType;

#[derive(Default, Hash, Clone)]
pub struct Status {
    pub work_dir: Vec<StatusItem>,
    pub stage: Vec<StatusItem>,
}

struct Request<R, A>(R, Option<A>);

///
pub struct AsyncStatus {
    current: Arc<Mutex<Request<u64, Status>>>,
    last: Arc<Mutex<Status>>,
    sender: Sender<AsyncNotification>,
    pending: Arc<AtomicUsize>,
}

impl AsyncStatus {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Request(0, None))),
            last: Arc::new(Mutex::new(Status::default())),
            sender,
            pending: Arc::new(AtomicUsize::new(0)),
        }
    }

    ///
    pub fn last(&mut self) -> Returns<Status> {
        let last = self.last.lock()?;
        Ok(last.clone())
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
    }

    ///
    pub fn fetch(&mut self, request: u64) -> Returns<Option<Status>> {
        let hash_request = hash(&request);

        trace!("request: {} [hash: {}]", request, hash_request);

        {
            let mut current = self.current.lock()?;

            if current.0 == hash_request {
                return Ok(current.1.clone());
            }

            current.0 = hash_request;
            current.1 = None;
        }

        let arc_current = Arc::clone(&self.current);
        let arc_last = Arc::clone(&self.last);
        let sender = self.sender.clone();
        let arc_pending = Arc::clone(&self.pending);
        rayon_core::spawn(move || {
            arc_pending.fetch_add(1, Ordering::Relaxed);

            AsyncStatus::fetch_helper(
                hash_request,
                arc_current,
                arc_last,
            )
            .expect("failed to fetch status");

            arc_pending.fetch_sub(1, Ordering::Relaxed);

            sender
                .send(AsyncNotification::Status)
                .expect("error sending status");
        });

        Ok(None)
    }

    fn fetch_helper(
        hash_request: u64,
        arc_current: Arc<Mutex<Request<u64, Status>>>,
        arc_last: Arc<Mutex<Status>>,
    ) -> Returns<()> {
        let res = Self::get_status()?;
        trace!("status fetched: {}", hash(&res));

        {
            let mut current = arc_current.lock()?;
            if current.0 == hash_request {
                current.1 = Some(res.clone());
            }
        }

        {
            let mut last = arc_last.lock()?;
            *last = res;
        }

        Ok(())
    }

    fn get_status() -> Returns<Status> {
        let work_dir =
            sync::status::get_status(CWD, StatusType::WorkingDir)?;
        let stage = sync::status::get_status(CWD, StatusType::Stage)?;

        Ok(Status { stage, work_dir })
    }
}
