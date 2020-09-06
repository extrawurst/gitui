use crate::{
    error::{Error, Result},
    sync, AsyncNotification, CWD,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use sync::ProgressNotification;

#[derive(Clone, Debug)]
enum PushStates {
    None,
    // Packing,
    // Pushing(usize, usize),
}

impl Default for PushStates {
    fn default() -> Self {
        PushStates::None
    }
}

///
#[derive(Default, Clone, Debug)]
pub struct PushRequest {
    ///
    pub remote: String,
    ///
    pub branch: String,
}

#[derive(Default, Clone, Debug)]
struct PushState {
    request: PushRequest,
    state: PushStates,
}

///
pub struct AsyncPush {
    state: Arc<Mutex<Option<PushState>>>,
    last_result: Arc<Mutex<Option<String>>>,
    sender: Sender<AsyncNotification>,
}

impl AsyncPush {
    ///
    pub fn new(sender: &Sender<AsyncNotification>) -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
            last_result: Arc::new(Mutex::new(None)),
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
    pub fn request(&mut self, params: PushRequest) -> Result<()> {
        log::trace!("request");

        if self.is_pending()? {
            return Ok(());
        }

        self.set_request(&params)?;

        let arc_state = Arc::clone(&self.state);
        let arc_res = Arc::clone(&self.last_result);
        let sender = self.sender.clone();

        thread::spawn(move || {
            let (progress_sender, receiver) = unbounded();

            Self::spawn_receiver_thread(receiver);

            let res = sync::push(
                CWD,
                params.remote.as_str(),
                params.branch.as_str(),
                progress_sender.clone(),
            );

            progress_sender
                .send(ProgressNotification::Done)
                .expect("closing send failed");

            Self::set_result(arc_res, res).expect("result error");

            Self::clear_request(arc_state).expect("clear error");

            sender
                .send(AsyncNotification::Push)
                .expect("error sending push");
        });

        Ok(())
    }

    fn spawn_receiver_thread(
        receiver: Receiver<ProgressNotification>,
    ) {
        log::info!("push progress receiver spawned");

        thread::spawn(move || loop {
            let incoming = receiver.recv();
            log::info!("push progress received: {:?}", incoming);
            match incoming {
                Ok(update) => match update {
                    ProgressNotification::Done => break,
                    _ => (),
                },
                Err(e) => {
                    log::error!(
                        "push progress receiver error: {}",
                        e
                    );
                    break;
                }
            }
        });
    }

    fn set_request(&self, params: &PushRequest) -> Result<()> {
        let mut state = self.state.lock()?;

        if state.is_some() {
            return Err(Error::Generic("pending request".into()));
        }

        *state = Some(PushState {
            request: params.clone(),
            ..PushState::default()
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
