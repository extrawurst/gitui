use crate::{hash, sync, AsyncNotification, Diff};
use crossbeam_channel::Sender;
use log::trace;
use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

#[derive(Default, Hash)]
struct DiffRequest(String, bool);

struct Request<R, A>(R, Option<A>);

pub struct AsyncDiff {
    current: Arc<Mutex<Request<u64, Diff>>>,
    sender: Sender<AsyncNotification>,
}

impl AsyncDiff {
    ///
    pub fn new(sender: Sender<AsyncNotification>) -> Self {
        Self {
            current: Arc::new(Mutex::new(Request(0, None))),
            sender,
        }
    }

    ///
    pub fn request(
        &mut self,
        file_path: String,
        stage: bool,
    ) -> Option<Diff> {
        trace!("request");

        let request = DiffRequest(file_path.clone(), stage);

        let hash = hash(&request);

        {
            let mut current = self.current.lock().unwrap();

            if current.0 == hash {
                return current.1.clone();
            }

            current.0 = hash;
            current.1 = None;
        }

        let arc_clone = Arc::clone(&self.current);
        let sender = self.sender.clone();
        rayon_core::spawn(move || {
            let res = sync::diff::get_diff(file_path.clone(), stage);
            let mut notify = false;
            {
                let mut current = arc_clone.lock().unwrap();
                if current.0 == hash {
                    current.1 = Some(res);
                    notify = true;
                }
            }

            if notify {
                sender.send(AsyncNotification::Diff).unwrap();
            }
        });

        None
    }
}
