//!

use crate::sync::remotes::push::ProgressNotification;
use git2::PackBuilderStage;
use std::cmp;

///
#[derive(Clone, Debug)]
pub enum RemoteProgressState {
    ///
    PackingAddingObject,
    ///
    PackingDeltafiction,
    ///
    Pushing,
    ///
    Done,
}

///
#[derive(Clone, Debug)]
pub struct RemoteProgress {
    ///
    pub state: RemoteProgressState,
    ///
    pub progress: u8,
}

impl RemoteProgress {
    ///
    pub fn new(
        state: RemoteProgressState,
        current: usize,
        total: usize,
    ) -> Self {
        let total = cmp::max(current, total) as f32;
        let progress = current as f32 / total * 100.0;
        let progress = progress as u8;
        Self { state, progress }
    }
}

impl From<ProgressNotification> for RemoteProgress {
    fn from(progress: ProgressNotification) -> Self {
        match progress {
            ProgressNotification::Packing {
                stage,
                current,
                total,
            } => match stage {
                PackBuilderStage::AddingObjects => {
                    RemoteProgress::new(
                        RemoteProgressState::PackingAddingObject,
                        current,
                        total,
                    )
                }
                PackBuilderStage::Deltafication => {
                    RemoteProgress::new(
                        RemoteProgressState::PackingDeltafiction,
                        current,
                        total,
                    )
                }
            },
            ProgressNotification::PushTransfer {
                current,
                total,
                ..
            } => RemoteProgress::new(
                RemoteProgressState::Pushing,
                current,
                total,
            ),
            _ => RemoteProgress::new(RemoteProgressState::Done, 1, 1),
        }
    }
}
