use crossbeam_channel::Sender;

pub struct History {
	visibility_txs: Vec<Sender<bool>>,
}

impl History {
	pub const fn new() -> Self {
		Self {
			visibility_txs: Vec::new(),
		}
	}

	fn send_to_last(&self, visibility: bool) {
		self.visibility_txs.last().map(|tx| tx.send(visibility));
	}

	pub fn push(&mut self, visibility_tx: Sender<bool>) {
		self.send_to_last(false);
		self.visibility_txs.push(visibility_tx);
		self.send_to_last(true);
	}

	pub fn pop(&mut self) {
		self.send_to_last(false);
		self.visibility_txs.pop();
		self.send_to_last(true);
	}
}
