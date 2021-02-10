use anyhow::Result;
use asyncgit::{
    sync::{self, limit_str, CommitInfo},
    AsyncLog, AsyncNotification, CWD,
};
use bitflags::bitflags;
use crossbeam_channel::{Sender, TryRecvError};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

const FILTER_SLEEP_DURATION: Duration = Duration::from_millis(10);
const FILTER_SLEEP_DURATION_FAILED_LOCK: Duration =
    Duration::from_millis(500);
const SLICE_SIZE: usize = 1200;

bitflags! {
    pub struct FilterBy: u32 {
        const SHA = 0b0000_0001;
        const AUTHOR = 0b0000_0010;
        const MESSAGE = 0b0000_0100;
    }
}

#[derive(PartialEq)]
pub enum FilterStatus {
    Filtering,
    Finished,
}

pub struct AsyncCommitFilterer {
    git_log: AsyncLog,
    filter_strings: Vec<(String, FilterBy)>,
    filtered_commits: Arc<Mutex<Vec<CommitInfo>>>,
    filter_count: Arc<AtomicUsize>,
    filter_finished: Arc<AtomicBool>,
    filter_thread_sender: Option<Sender<bool>>,
    sender: Sender<AsyncNotification>,
}

impl AsyncCommitFilterer {
    pub fn new(
        git_log: AsyncLog,
        sender: &Sender<AsyncNotification>,
    ) -> Self {
        Self {
            filter_strings: Vec::new(),
            git_log: git_log,
            filtered_commits: Arc::new(Mutex::new(Vec::new())),
            filter_count: Arc::new(AtomicUsize::new(0)),
            filter_finished: Arc::new(AtomicBool::new(true)),
            filter_thread_sender: None,
            sender: sender.clone(),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.fetch() == FilterStatus::Filtering
    }

    pub fn clear(
        &mut self,
    ) -> Result<
        (),
        std::sync::PoisonError<
            std::sync::MutexGuard<Vec<CommitInfo>>,
        >,
    > {
        self.filtered_commits.lock()?.clear();
        Ok(())
    }

    pub fn filter(
        vec_commit_info: Vec<CommitInfo>,
        filter_strings: &Vec<(String, FilterBy)>,
    ) -> Vec<CommitInfo> {
        let mut commit_infos = vec_commit_info;
        for (s, filter) in filter_strings {
            commit_infos = commit_infos
                .drain(..)
                .filter(|ci| {
                    if filter.contains(FilterBy::SHA) {
                        if ci
                            .id
                            .to_string()
                            .to_lowercase()
                            .contains(&s.to_lowercase())
                        {
                            return true;
                        }
                    }

                    if filter.contains(FilterBy::AUTHOR) {
                        if ci
                            .author
                            .to_lowercase()
                            .contains(&s.to_lowercase())
                        {
                            return true;
                        }
                    }
                    if filter.contains(FilterBy::MESSAGE) {
                        if ci
                            .message
                            .to_lowercase()
                            .contains(&s.to_lowercase())
                        {
                            return true;
                        }
                    }
                    false
                })
                .collect::<Vec<CommitInfo>>()
        }
        return commit_infos;
    }

    pub fn start_filter(
        &mut self,
        filter_strings: Vec<(String, FilterBy)>,
    ) -> Result<()> {
        self.stop_filter();

        self.clear().expect("Can't fail unless app crashes");
        self.filter_strings = filter_strings.clone();
        self.filter_count.store(0, Ordering::Relaxed);

        let filtered_commits = Arc::clone(&self.filtered_commits);
        let filter_count = Arc::clone(&self.filter_count);
        let async_log = self.git_log.clone();
        let filter_finished = Arc::clone(&self.filter_finished);
        filter_finished.store(false, Ordering::Relaxed);

        let (tx, rx) = crossbeam_channel::unbounded();

        self.filter_thread_sender = Some(tx);
        let async_app_sender = self.sender.clone();

        rayon_core::spawn(move || {
            let mut cur_index: usize = 0;
            loop {
                match rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        break;
                    }
                    Err(TryRecvError::Empty) => {
                        // Get the git_log and start filtering through it
                        match async_log
                            .get_slice(cur_index, SLICE_SIZE)
                        {
                            Ok(ids) => match sync::get_commits_info(
                                CWD,
                                &ids,
                                usize::MAX,
                            ) {
                                Ok(v) => {
                                    if v.len() == 0
                                        && !async_log.is_pending()
                                    {
                                        // Assume finished if log not pending and 0 recieved
                                        filter_finished.store(
                                            true,
                                            Ordering::Relaxed,
                                        );
                                        break;
                                    }

                                    let mut filtered = Self::filter(
                                        v,
                                        &filter_strings,
                                    );
                                    filter_count.fetch_add(
                                        filtered.len(),
                                        Ordering::Relaxed,
                                    );
                                    match filtered_commits.lock() {
                                        Ok(mut fc) => {
                                            fc.append(&mut filtered);
                                            drop(fc);
                                            cur_index += SLICE_SIZE;
                                            async_app_sender.send(AsyncNotification::Log).expect("error sending");
                                            thread::sleep(
                                                FILTER_SLEEP_DURATION,
                                            );
                                        }
                                        Err(_) => {
                                            // Failed to lock `filtered_commits`
                                            thread::sleep(
                                    FILTER_SLEEP_DURATION_FAILED_LOCK,
                                );
                                        }
                                    }
                                }
                                Err(_) => {
                                    // Failed to get commit info
                                    thread::sleep(
                                FILTER_SLEEP_DURATION_FAILED_LOCK,
                            );
                                }
                            },
                            Err(_) => {
                                // Failed to get slice
                                thread::sleep(
                                    FILTER_SLEEP_DURATION_FAILED_LOCK,
                                );
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }

    /// Stop the filter if one was running, otherwise does nothing.
    /// Is it possible to restart from this stage by calling restart
    pub fn stop_filter(&self) {
        // Any error this gives can be safely ignored,
        // it will send if reciever exists, otherwise does nothing
        if let Some(sender) = &self.filter_thread_sender {
            match sender.try_send(true) {
                Ok(_) | Err(_) => {}
            };
        }
        self.filter_finished.store(true, Ordering::Relaxed);
    }

    /// Use if the next item to be filtered is a substring of the previous item.
    /// This then only searches through the previous list
    //pub fn continue_filter(&mut self, _s: String) -> Result<()> {
    //   Ok(())
    //}

    pub fn get_filter_items(
        &mut self,
        start: usize,
        amount: usize,
        message_length_limit: usize,
    ) -> Result<
        Vec<CommitInfo>,
        std::sync::PoisonError<
            std::sync::MutexGuard<Vec<CommitInfo>>,
        >,
    > {
        let fc = self.filtered_commits.lock()?;
        let len = fc.len();
        let min = start.min(len);
        let max = min + amount;
        let max = max.min(len);
        let mut commits_requested = fc[min..max].to_vec();
        for c in &mut commits_requested {
            c.message = limit_str(&c.message, message_length_limit)
                .to_string();
        }
        Ok(commits_requested)
    }

    pub fn count(&self) -> usize {
        self.filter_count.load(Ordering::Relaxed)
    }

    pub fn fetch(&self) -> FilterStatus {
        if self.filter_finished.load(Ordering::Relaxed) {
            FilterStatus::Finished
        } else {
            FilterStatus::Filtering
        }
    }
}
