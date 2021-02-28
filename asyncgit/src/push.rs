use crate::{
    error::{Error, Result},
    sync::{
        cred::BasicAuthCredential, remotes::push::push,
        remotes::push::ProgressNotification,
    },
    AsyncNotification, RemoteProgress, CWD,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use thread::JoinHandle;

///
#[derive(Default, Clone, Debug)]
pub struct PushRequest {
    ///
    pub remote: String,
    ///
    pub branch: String,
    ///
    pub force: bool,
    ///
    pub basic_credential: Option<BasicAuthCredential>,
}

#[derive(Default, Clone, Debug)]
struct PushState {
    request: PushRequest,
}

///
pub struct AsyncPush {
    state: Arc<Mutex<Option<PushState>>>,
    last_result: Arc<Mutex<Option<String>>>,
    progress: Arc<Mutex<Option<ProgressNotification>>>,
    sender: Sender<AsyncNotification>,
}

impl AsyncPush {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
            last_result: Arc::new(Mutex::new(None)),
            progress: Arc::new(Mutex::new(None)),
            sender: sender.clone(),
        }
    }

    ///
    pub fn is_pending(&self) -> Result<bool> {
        let state = self.state.lock()?;
        Ok(state.is_some())
    }

    ///
    pub fn last_result(&self) -> Result<Option<String>> {
        let res = self.last_result.lock()?;
        Ok(res.clone())
    }

    ///
    pub fn progress(&self) -> Result<Option<RemoteProgress>> {
        let res = self.progress.lock()?;
        Ok(res.as_ref().map(|progress| progress.clone().into()))
    }

    ///
    pub fn request(&mut self, params: PushRequest) -> Result<()> {
        log::trace!("request");

        if self.is_pending()? {
            return Ok(());
        }

        self.set_request(&params)?;
        Self::set_progress(self.progress.clone(), None)?;

        let arc_state = Arc::clone(&self.state);
        let arc_res = Arc::clone(&self.last_result);
        let arc_progress = Arc::clone(&self.progress);
        let sender = self.sender.clone();

        thread::spawn(move || {
            let (progress_sender, receiver) = unbounded();

            let handle = Self::spawn_receiver_thread(
                sender.clone(),
                receiver,
                arc_progress,
            );

            let res = push(
                CWD,
                params.remote.as_str(),
                params.branch.as_str(),
                params.force,
                params.basic_credential,
                Some(progress_sender.clone()),
            );

            progress_sender
                .send(ProgressNotification::Done)
                .expect("closing send failed");

            handle.join().expect("joining thread failed");

            Self::set_result(arc_res, res).expect("result error");

            Self::clear_request(arc_state).expect("clear error");

            sender
                .send(AsyncNotification::Push)
                .expect("error sending push");
        });

        Ok(())
    }

    pub(crate) fn spawn_receiver_thread(
        sender: Sender<AsyncNotification>,
        receiver: Receiver<ProgressNotification>,
        progress: Arc<Mutex<Option<ProgressNotification>>>,
    ) -> JoinHandle<()> {
        log::info!("push progress receiver spawned");

        thread::spawn(move || loop {
            let incoming = receiver.recv();
            match incoming {
                Ok(update) => {
                    Self::set_progress(
                        progress.clone(),
                        Some(update.clone()),
                    )
                    .expect("set prgoress failed");
                    sender
                        .send(AsyncNotification::Push)
                        .expect("error sending push");

                    //NOTE: for better debugging
                    thread::sleep(Duration::from_millis(300));

                    if let ProgressNotification::Done = update {
                        break;
                    }
                }
                Err(e) => {
                    log::error!(
                        "push progress receiver error: {}",
                        e
                    );
                    break;
                }
            }
        })
    }

    fn set_request(&self, params: &PushRequest) -> Result<()> {
        let mut state = self.state.lock()?;

        if state.is_some() {
            return Err(Error::Generic("pending request".into()));
        }

        *state = Some(PushState {
            request: params.clone(),
        });

        Ok(())
    }

    fn clear_request(
        state: Arc<Mutex<Option<PushState>>>,
    ) -> Result<()> {
        let mut state = state.lock()?;

        *state = None;

        Ok(())
    }

    pub(crate) fn set_progress(
        progress: Arc<Mutex<Option<ProgressNotification>>>,
        state: Option<ProgressNotification>,
    ) -> Result<()> {
        let simple_progress: Option<RemoteProgress> =
            state.as_ref().map(|prog| prog.clone().into());
        log::info!("remote progress: {:?}", simple_progress);
        let mut progress = progress.lock()?;

        *progress = state;

        Ok(())
    }

    fn set_result(
        arc_result: Arc<Mutex<Option<String>>>,
        res: Result<()>,
    ) -> Result<()> {
        let mut last_res = arc_result.lock()?;

        *last_res = match res {
            Ok(_) => None,
            Err(e) => {
                log::error!("push error: {}", e);
                Some(e.to_string())
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_progress::RemoteProgressState;

    #[test]
    fn test_progress_zero_total() {
        let prog =
            RemoteProgress::new(RemoteProgressState::Pushing, 1, 0);

        assert_eq!(prog.progress, 100);
    }

    #[test]
    fn test_progress_rounding() {
        let prog =
            RemoteProgress::new(RemoteProgressState::Pushing, 2, 10);

        assert_eq!(prog.progress, 20);
    }
}
