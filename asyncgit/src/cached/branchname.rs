use crate::{error::Result, sync};
use sync::Head;

///
pub struct BranchName {
    last_result: Option<(Head, String)>,
    repo_path: String,
}

impl BranchName {
    ///
    pub fn new(path: &str) -> Self {
        Self {
            repo_path: path.to_string(),
            last_result: None,
        }
    }

    ///
    pub fn lookup(&mut self) -> Result<String> {
        let current_head =
            sync::get_head_tuple(self.repo_path.as_str())?;

        if let Some((last_head, branch_name)) =
            self.last_result.as_ref()
        {
            if *last_head == current_head {
                return Ok(branch_name.clone());
            }
        }

        self.fetch(current_head)
    }

    fn fetch(&mut self, head: Head) -> Result<String> {
        let name = sync::get_branch_name(self.repo_path.as_str())?;
        self.last_result = Some((head, name.clone()));
        Ok(name)
    }
}
