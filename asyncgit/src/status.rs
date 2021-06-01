use crate::{
    error::Result,
    hash,
    sync::{self, status::StatusType},
    AsyncNotification, StatusItem, CWD,
};
use crossbeam_channel::Sender;
use std::{
    hash::Hash,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

fn current_tick() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before unix epoch!")
        .as_millis()
}

#[derive(Default, Hash, Clone)]
pub struct Status {
    pub items: Vec<StatusItem>,
}

///
#[derive(Default, Hash, Copy, Clone, PartialEq)]
pub struct StatusParams {
    tick: u128,
    status_type: StatusType,
}

impl StatusParams {
    ///
    pub fn new(status_type: StatusType) -> Self {
        Self {
            tick: current_tick(),
            status_type,
        }
    }
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
    pub fn last(&mut self) -> Result<Status> {
        let last = self.last.lock()?;
        Ok(last.clone())
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
    }

    ///
    pub fn fetch(
        &mut self,
        params: &StatusParams,
    ) -> Result<Option<Status>> {
        if self.is_pending() {
            log::trace!("request blocked, still pending");
            return Ok(None);
        }

        let hash_request = hash(&params);

        log::trace!(
            "request: [hash: {}] (type: {:?})",
            hash_request,
            params.status_type,
        );

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
        let status_type = params.status_type;

        self.pending.fetch_add(1, Ordering::Relaxed);

        rayon_core::spawn(move || {
            let ok = Self::fetch_helper(
                status_type,
                hash_request,
                &arc_current,
                &arc_last,
            )
            .is_ok();

            arc_pending.fetch_sub(1, Ordering::Relaxed);

            if ok {
                sender
                    .send(AsyncNotification::Status)
                    .expect("error sending status");
            }
        });

        Ok(None)
    }

    fn fetch_helper(
        status_type: StatusType,
        hash_request: u64,
        arc_current: &Arc<Mutex<Request<u64, Status>>>,
        arc_last: &Arc<Mutex<Status>>,
    ) -> Result<()> {
        let res = Self::get_status(status_type)?;
        log::trace!(
            "status fetched: {} (type: {:?})",
            hash_request,
            status_type,
        );

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

    fn get_status(status_type: StatusType) -> Result<Status> {
        Ok(Status {
            items: sync::status::get_status(CWD, status_type)?,
        })
    }
}
