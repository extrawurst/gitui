use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};

pub trait AsyncJob: Send + Sync + Clone {
    fn run(&mut self);
}

#[derive(Debug, Clone)]
pub struct AsyncSingleJob<J: AsyncJob, T: Copy + Send + 'static> {
    next: Arc<Mutex<Option<J>>>,
    last: Arc<Mutex<Option<J>>>,
    sender: Sender<T>,
    pending: Arc<Mutex<()>>,
    notification: T,
}

impl<J: 'static + AsyncJob, T: Copy + Send + 'static>
    AsyncSingleJob<J, T>
{
    ///
    pub fn new(sender: Sender<T>, value: T) -> Self {
        Self {
            next: Arc::new(Mutex::new(None)),
            last: Arc::new(Mutex::new(None)),
            pending: Arc::new(Mutex::new(())),
            notification: value,
            sender,
        }
    }

    ///
    pub fn is_pending(&self) -> bool {
        self.pending.try_lock().is_err()
    }

    /// makes sure `next` is cleared and returns `true` if it actually canceled something
    pub fn cancel(&mut self) -> bool {
        if let Ok(mut next) = self.next.lock() {
            if next.is_some() {
                *next = None;
                return true;
            }
        }

        false
    }

    /// return clone of last result
    pub fn get_last(&self) -> Option<J> {
        if let Ok(last) = self.last.lock() {
            last.clone()
        } else {
            None
        }
    }

    ///
    pub fn spawn(&mut self, task: J) -> bool {
        self.schedule_next(task);
        self.check_for_job()
    }

    ///
    pub fn check_for_job(&self) -> bool {
        if self.is_pending() {
            return false;
        }

        if let Some(task) = self.take_next() {
            let self_arc = self.clone();

            rayon_core::spawn(move || {
                self_arc.run_job(task);
            });

            return true;
        }

        false
    }

    //TODO: return Result
    fn run_job(&self, mut task: J) {
        //limit the pending scope
        {
            let _pending = self.pending.lock().expect("");

            task.run();

            if let Ok(mut last) = self.last.lock() {
                *last = Some(task);
            }

            self.sender.send(self.notification).expect("send failed");
        }

        self.check_for_job();
    }

    ///
    fn schedule_next(&mut self, task: J) {
        if let Ok(mut next) = self.next.lock() {
            *next = Some(task);
        }
    }

    ///
    fn take_next(&self) -> Option<J> {
        if let Ok(mut next) = self.next.lock() {
            next.take()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crossbeam_channel::unbounded;
    use pretty_assertions::assert_eq;
    use std::{
        sync::atomic::AtomicU32, thread::sleep, time::Duration,
    };

    #[derive(Clone)]
    struct TestJob {
        v: Arc<AtomicU32>,
        value_to_add: u32,
    }

    impl AsyncJob for TestJob {
        fn run(&mut self) {
            sleep(Duration::from_millis(100));

            self.v.fetch_add(
                self.value_to_add,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
    }

    type Notificaton = ();

    #[test]
    fn test_overwrite() {
        let (sender, receiver) = unbounded();

        let mut job: AsyncSingleJob<TestJob, Notificaton> =
            AsyncSingleJob::new(sender, ());

        let task = TestJob {
            v: Arc::new(AtomicU32::new(1)),
            value_to_add: 1,
        };

        assert!(job.spawn(task.clone()));
        sleep(Duration::from_millis(1));
        for _ in 0..5 {
            assert!(!job.spawn(task.clone()));
        }

        let _foo = receiver.recv().unwrap();
        let _foo = receiver.recv().unwrap();
        assert!(receiver.is_empty());

        assert_eq!(
            task.v.load(std::sync::atomic::Ordering::Relaxed),
            3
        );
    }

    #[test]
    fn test_cancel() {
        let (sender, receiver) = unbounded();

        let mut job: AsyncSingleJob<TestJob, Notificaton> =
            AsyncSingleJob::new(sender, ());

        let task = TestJob {
            v: Arc::new(AtomicU32::new(1)),
            value_to_add: 1,
        };

        assert!(job.spawn(task.clone()));
        sleep(Duration::from_millis(1));

        for _ in 0..5 {
            assert!(!job.spawn(task.clone()));
        }
        assert!(job.cancel());

        let _foo = receiver.recv().unwrap();

        assert_eq!(
            task.v.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }
}
