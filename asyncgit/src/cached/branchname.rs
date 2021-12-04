use crate::{
	error::Result,
	sync::{self, branch::get_branch_name, RepoPath},
};
use sync::Head;

///
pub struct BranchName {
	last_result: Option<(Head, String)>,
	repo_path: RepoPath,
}

impl BranchName {
	///
	pub fn new(path: &str) -> Self {
		Self {
			repo_path: path.into(),
			last_result: None,
		}
	}

	///
	pub fn lookup(&mut self) -> Result<String> {
		let current_head = sync::get_head_tuple(&self.repo_path)?;

		if let Some((last_head, branch_name)) =
			self.last_result.as_ref()
		{
			if *last_head == current_head {
				return Ok(branch_name.clone());
			}
		}

		self.fetch(current_head)
	}

	///
	pub fn last(&self) -> Option<String> {
		self.last_result.as_ref().map(|last| last.1.clone())
	}

	fn fetch(&mut self, head: Head) -> Result<String> {
		let name = get_branch_name(&self.repo_path)?;
		self.last_result = Some((head, name.clone()));
		Ok(name)
	}
}
