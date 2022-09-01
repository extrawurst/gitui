use anyhow::Result;
use crossbeam_channel::{unbounded, Sender};
use notify::{Error, RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{
	new_debouncer, DebouncedEvent, Debouncer,
};
use std::{
	path::Path, sync::mpsc::RecvError, thread, time::Duration,
};

pub struct RepoWatcher {
	receiver: crossbeam_channel::Receiver<()>,
	#[allow(dead_code)]
	debouncer: Debouncer<RecommendedWatcher>,
}

impl RepoWatcher {
	pub fn new(workdir: &str) -> Result<Self> {
		let (tx, rx) = std::sync::mpsc::channel();

		let mut debouncer =
			new_debouncer(Duration::from_secs(2), None, tx)?;

		debouncer
			.watcher()
			.watch(Path::new(workdir), RecursiveMode::Recursive)?;

		let (out_tx, out_rx) = unbounded();

		thread::spawn(move || {
			if let Err(e) = Self::forwarder(&rx, &out_tx) {
				//maybe we need to restart the forwarder now?
				log::error!("notify receive error: {}", e);
			}
		});

		Ok(Self {
			debouncer,
			receiver: out_rx,
		})
	}

	///
	pub fn receiver(&self) -> crossbeam_channel::Receiver<()> {
		self.receiver.clone()
	}

	fn forwarder(
		receiver: &std::sync::mpsc::Receiver<
			Result<Vec<DebouncedEvent>, Vec<Error>>,
		>,
		sender: &Sender<()>,
	) -> Result<(), RecvError> {
		loop {
			let ev = receiver.recv()?;

			if let Ok(ev) = ev {
				log::debug!("notify events: {}", ev.len());

				for (idx, ev) in ev.iter().enumerate() {
					log::debug!("notify [{}]: {:?}", idx, ev);
				}

				if !ev.is_empty() {
					sender.send(()).expect("send error");
				}
			}
		}
	}
}
