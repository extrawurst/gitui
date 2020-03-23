use crate::{hash, sync, AsyncNotification, StatusItem};
use crossbeam_channel::Sender;
use log::trace;
use std::{
    hash::Hash,
    sync::{Arc, Mutex},
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
}

impl AsyncStatus {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Request(0, None))),
            last: Arc::new(Mutex::new(Status::default())),
            sender,
        }
    }

    ///
    pub fn last(&mut self) -> Status {
        let last = self.last.lock().unwrap();
        last.clone()
    }

    ///
    pub fn fetch(&mut self, request: u64) -> Option<Status> {
        let hash_request = hash(&request);

        trace!("request: {} [hash: {}]", request, hash_request);

        {
            let mut current = self.current.lock().unwrap();

            if current.0 == hash_request {
                return current.1.clone();
            }

            current.0 = hash_request;
            current.1 = None;
        }

        let arc_current = Arc::clone(&self.current);
        let arc_last = Arc::clone(&self.last);
        let sender = self.sender.clone();
        rayon_core::spawn(move || {
            let res = Self::get_status();
            trace!("status fetched: {}", hash(&res));
            let mut notify = false;
            {
                let mut current = arc_current.lock().unwrap();
                if current.0 == hash_request {
                    current.1 = Some(res.clone());
                    notify = true;
                }
            }

            {
                let mut last = arc_last.lock().unwrap();
                *last = res;
            }

            if notify {
                sender.send(AsyncNotification::Status).unwrap();
            }
        });

        None
    }

    fn get_status() -> Status {
        let work_dir =
            sync::status::get_index(StatusType::WorkingDir);
        let stage = sync::status::get_index(StatusType::Stage);

        Status { stage, work_dir }
    }
}
