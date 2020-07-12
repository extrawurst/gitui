use crate::{
    error::Result,
    hash,
    sync::{self},
    AsyncNotification, CWD,
};
use crossbeam_channel::Sender;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use sync::Tags;

///
#[derive(Default, Clone)]
struct TagsResult {
    hash: u64,
    tags: Tags,
}

///
pub struct AsyncTags {
    last: Arc<Mutex<Option<TagsResult>>>,
    sender: Sender<AsyncNotification>,
    pending: Arc<AtomicUsize>,
}

impl AsyncTags {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        Self {
            last: Arc::new(Mutex::new(None)),
            sender: sender.clone(),
            pending: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// last fetched result
    pub fn last(&mut self) -> Result<Option<Tags>> {
        let last = self.last.lock()?;

        Ok(last.clone().map(|last| last.tags))
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0
    }

    ///
    pub fn request(&mut self) -> Result<()> {
        log::trace!("request");

        if self.is_pending() {
            return Ok(());
        }

        let arc_last = Arc::clone(&self.last);
        let sender = self.sender.clone();
        let arc_pending = Arc::clone(&self.pending);
        rayon_core::spawn(move || {
            arc_pending.fetch_add(1, Ordering::Relaxed);

            let notify = AsyncTags::getter(arc_last)
                .expect("error getting tags");

            arc_pending.fetch_sub(1, Ordering::Relaxed);

            if notify {
                sender
                    .send(AsyncNotification::Tags)
                    .expect("error sending notify");
            }
        });

        Ok(())
    }

    fn getter(
        arc_last: Arc<Mutex<Option<TagsResult>>>,
    ) -> Result<bool> {
        let tags = sync::get_tags(CWD)?;

        let hash = hash(&tags);

        if Self::last_hash(arc_last.clone())
            .map(|last| last == hash)
            .unwrap_or_default()
        {
            return Ok(false);
        }

        {
            let mut last = arc_last.lock()?;
            *last = Some(TagsResult { tags, hash });
        }

        Ok(true)
    }

    fn last_hash(
        last: Arc<Mutex<Option<TagsResult>>>,
    ) -> Option<u64> {
        last.lock()
            .ok()
            .and_then(|last| last.as_ref().map(|last| last.hash))
    }
}
