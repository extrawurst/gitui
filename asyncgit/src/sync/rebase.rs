use crate::error::{Error, Result};

/// rebase attempt which aborts and undo's rebase if any conflict appears
pub fn conflict_free_rebase(
	repo: &git2::Repository,
	commit: &git2::AnnotatedCommit,
) -> Result<()> {
	let mut rebase = repo.rebase(None, Some(commit), None, None)?;
	let signature =
		crate::sync::commit::signature_allow_undefined_name(repo)?;
	while let Some(op) = rebase.next() {
		let _op = op?;
		// dbg!(op.id());

		if repo.index()?.has_conflicts() {
			rebase.abort()?;
			return Err(Error::RebaseConflict);
		}

		rebase.commit(None, &signature, None)?;
	}
	if repo.index()?.has_conflicts() {
		rebase.abort()?;
		return Err(Error::RebaseConflict);
	}
	rebase.finish(Some(&signature))?;
	Ok(())
}
