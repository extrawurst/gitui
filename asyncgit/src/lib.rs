mod diff;

use crossbeam_channel::Sender;
pub use diff::{get_diff, Diff, DiffLine, DiffLineType};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

#[derive(Default, Hash)]
struct DiffRequest(String, bool);

struct Request<R, A>(R, Option<A>);

pub struct AsyncDiff {
    current: Arc<Mutex<Request<u64, Diff>>>,
    sender: Sender<()>,
}

impl AsyncDiff {
    ///
    pub fn new(sender: Sender<()>) -> Self {
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
        let request = DiffRequest(file_path.clone(), stage);

        let mut hasher = DefaultHasher::new();
        request.hash(&mut hasher);
        let hash = hasher.finish();

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
            let res = get_diff(file_path.clone(), stage);
            let mut notify = false;
            {
                let mut current = arc_clone.lock().unwrap();
                if current.0 == hash {
                    current.1 = Some(res);
                    notify = true;
                }
            }

            if notify {
                sender.send(()).unwrap();
            }
        });

        None
    }
}
