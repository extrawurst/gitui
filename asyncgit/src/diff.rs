use crate::{hash, sync, AsyncNotification, Diff, CWD};
use crossbeam_channel::Sender;
use log::trace;
use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

///
#[derive(Default, Hash, Clone)]
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
    current: Arc<Mutex<Request<u64, Diff>>>,
    last: Arc<Mutex<Option<LastResult<DiffParams, Diff>>>>,
    sender: Sender<AsyncNotification>,
}

impl AsyncDiff {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Request(0, None))),
            last: Arc::new(Mutex::new(None)),
            sender,
        }
    }

    ///
    pub fn last(&mut self) -> Option<Diff> {
        let last = self.last.lock().unwrap();
        if let Some(res) = last.clone() {
            Some(res.result)
        } else {
            None
        }
    }

    ///
    pub fn refresh(&mut self) {
        if let Some(param) = self.get_last_param() {
            self.clear_current();
            self.request(param);
        }
    }

    ///
    pub fn request(&mut self, params: DiffParams) -> Option<Diff> {
        trace!("request");

        let hash = hash(&params);

        {
            let mut current = self.current.lock().unwrap();

            if current.0 == hash {
                return current.1.clone();
            }

            current.0 = hash;
            current.1 = None;
        }

        let arc_current = Arc::clone(&self.current);
        let arc_last = Arc::clone(&self.last);
        let sender = self.sender.clone();
        rayon_core::spawn(move || {
            let res =
                sync::diff::get_diff(CWD, params.0.clone(), params.1);
            let mut notify = false;
            {
                let mut current = arc_current.lock().unwrap();
                if current.0 == hash {
                    current.1 = Some(res.clone());
                    notify = true;
                }
            }

            {
                let mut last = arc_last.lock().unwrap();
                *last = Some(LastResult {
                    result: res,
                    hash,
                    params,
                });
            }

            if notify {
                sender.send(AsyncNotification::Diff).unwrap();
            }
        });

        None
    }

    fn get_last_param(&self) -> Option<DiffParams> {
        self.last.lock().unwrap().clone().map(|e| e.params)
    }

    fn clear_current(&mut self) {
        let mut current = self.current.lock().unwrap();
        current.0 = 0;
        current.1 = None;
    }
}
