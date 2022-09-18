use anyhow::Result;
use asyncgit::sync::{
	diff::DiffOptions, repo_dir, RepoPathRef,
	ShowUntrackedFilesConfig,
};
use ron::{
	de::from_bytes,
	ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::{
	cell::RefCell,
	fs::File,
	io::{Read, Write},
	path::PathBuf,
	rc::Rc,
};

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
struct OptionsData {
	pub tab: usize,
}

#[derive(Clone)]
pub struct Options {
	//TODO: un-pub and use getters/setters and move into persisted data
	pub status_show_untracked: Option<ShowUntrackedFilesConfig>,
	pub diff: DiffOptions,

	repo: RepoPathRef,
	data: OptionsData,
}

pub type SharedOptions = Rc<RefCell<Options>>;

impl Options {
	pub fn new(repo: RepoPathRef) -> SharedOptions {
		Rc::new(RefCell::new(Self {
			data: Self::read(&repo).unwrap_or_default(),
			diff: DiffOptions::default(),
			status_show_untracked: None,
			repo,
		}))
	}

	pub fn set_current_tab(&mut self, tab: usize) {
		self.data.tab = tab;
		self.save();
	}

	pub const fn current_tab(&self) -> usize {
		self.data.tab
	}

	pub const fn diff_options(&self) -> DiffOptions {
		self.diff
	}

	fn save(&self) {
		if let Err(e) = self.save_failable() {
			log::error!("options save error: {}", e);
		}
	}

	fn read(repo: &RepoPathRef) -> Result<OptionsData> {
		let dir = Self::options_file(repo)?;

		let mut f = File::open(dir)?;
		let mut buffer = Vec::new();
		f.read_to_end(&mut buffer)?;
		Ok(from_bytes(&buffer)?)
	}

	fn save_failable(&self) -> Result<()> {
		let dir = Self::options_file(&self.repo)?;

		let mut file = File::create(&dir)?;
		let data =
			to_string_pretty(&self.data, PrettyConfig::default())?;
		file.write_all(data.as_bytes())?;

		Ok(())
	}

	fn options_file(repo: &RepoPathRef) -> Result<PathBuf> {
		let dir = repo_dir(&repo.borrow())?;
		let dir = dir.join("gitui");
		Ok(dir)
	}
}
