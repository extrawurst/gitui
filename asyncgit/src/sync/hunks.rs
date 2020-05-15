use super::{
    diff::{get_diff_raw, HunkHeader},
    utils::repo,
};
use crate::error::Returns;
use crate::{error::Error, hash};
use git2::{ApplyLocation, ApplyOptions, Diff};
use log::error;
use scopetime::scope_time;

///
pub fn stage_hunk(
    repo_path: &str,
    file_path: String,
    hunk_hash: u64,
) -> Returns<bool> {
    scope_time!("stage_hunk");

    let repo = repo(repo_path)?;

    let diff = get_diff_raw(&repo, &file_path, false, false)?;

    let mut opt = ApplyOptions::new();
    opt.hunk_callback(|hunk| {
        let header = HunkHeader::from(hunk.unwrap());
        hash(&header) == hunk_hash
    });

    repo.apply(&diff, ApplyLocation::Index, Some(&mut opt))?;

    Ok(true)
}

fn find_hunk_index(diff: &Diff, hunk_hash: u64) -> Option<usize> {
    let mut result = None;

    let mut hunk_count = 0;

    let foreach_result = diff.foreach(
        &mut |_, _| true,
        None,
        Some(&mut |_, hunk| {
            let header = HunkHeader::from(hunk);
            if hash(&header) == hunk_hash {
                result = Some(hunk_count);
            }
            hunk_count += 1;
            true
        }),
        None,
    );

    if foreach_result.is_ok() {
        result
    } else {
        None
    }
}

///
pub fn unstage_hunk(
    repo_path: &str,
    file_path: String,
    hunk_hash: u64,
) -> Returns<bool> {
    scope_time!("revert_hunk");

    let repo = repo(repo_path)?;

    let diff = get_diff_raw(&repo, &file_path, true, false)?;
    let diff_count_positive = diff.deltas().len();

    let hunk_index = find_hunk_index(&diff, hunk_hash);

    if hunk_index.is_none() {
        error!("hunk not found");
        return Err(Error::Generic("hunk not found".to_string()));
    }

    let diff = get_diff_raw(&repo, &file_path, true, true)?;

    assert_eq!(diff.deltas().len(), diff_count_positive);

    let mut count = 0;
    {
        let mut hunk_idx = 0;
        let mut opt = ApplyOptions::new();
        opt.hunk_callback(|_hunk| {
            let res = if hunk_idx == hunk_index.unwrap() {
                count += 1;
                true
            } else {
                false
            };

            hunk_idx += 1;

            res
        });
        if repo
            .apply(&diff, ApplyLocation::Index, Some(&mut opt))
            .is_err()
        {
            error!("apply failed");
            return Err(Error::Generic("apply failed".to_string()));
        }
    }

    Ok(count == 1)
}
