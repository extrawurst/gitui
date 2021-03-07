use super::diff::{get_diff_raw, HunkHeader};
use crate::error::{Error, Result};
use git2::{Diff, DiffLine, Patch, Repository};

//
pub(crate) struct HunkLines<'a> {
    pub hunk: HunkHeader,
    pub lines: Vec<DiffLine<'a>>,
}

///
pub(crate) fn get_file_diff_patch_and_hunklines<'a>(
    repo: &'a Repository,
    file: &str,
    is_staged: bool,
    reverse: bool,
) -> Result<(Patch<'a>, Vec<HunkLines<'a>>)> {
    let diff =
        get_diff_raw(&repo, file, is_staged, reverse, Some(0))?;
    let patches = get_patches(&diff)?;
    if patches.len() != 1 {
        return Err(Error::Generic(String::from("patch error")));
    }

    let patch =
        patches.into_iter().next().expect("was checked above");

    let lines = patch_get_hunklines(&patch)?;

    Ok((patch, lines))
}

//
fn patch_get_hunklines<'a>(
    patch: &Patch<'a>,
) -> Result<Vec<HunkLines<'a>>> {
    let count_hunks = patch.num_hunks();
    let mut res = Vec::with_capacity(count_hunks);
    for hunk_idx in 0..count_hunks {
        let (hunk, _) = patch.hunk(hunk_idx)?;

        let count_lines = patch.num_lines_in_hunk(hunk_idx)?;

        let mut hunk = HunkLines {
            hunk: HunkHeader::from(hunk),
            lines: Vec::with_capacity(count_lines),
        };

        for line_idx in 0..count_lines {
            let line = patch.line_in_hunk(hunk_idx, line_idx)?;
            hunk.lines.push(line);
        }

        res.push(hunk);
    }

    Ok(res)
}

//
fn get_patches<'a>(diff: &Diff<'a>) -> Result<Vec<Patch<'a>>> {
    let count = diff.deltas().len();

    let mut res = Vec::with_capacity(count);
    for idx in 0..count {
        let p = Patch::from_diff(&diff, idx)?;
        if let Some(p) = p {
            res.push(p);
        }
    }

    Ok(res)
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::sync::{
//         diff::get_diff_raw,
//         tests::{repo_init, repo_write_file, write_commit_file},
//     };

//     static FILE_1: &str = r"
// 1   start
// 2
// 3
// 4
// 5
// 6
// 7
// 8
// 9
// 0
// 1   end";

//     static FILE_2: &str = r"
// 1   start2
// 2
// 3
// 4
// 5
// 6
// 7
// 8
// 9
// 0
// 1   end2";

//     #[test]
//     fn test_smoke() {
//         let (_path, repo) = repo_init().unwrap();

//         repo_write_file(&repo, "test.txt", FILE_1);

//         let diff = get_diff_raw(&repo, "", false, false).unwrap();
//         assert_eq!(diff.stats().unwrap().files_changed(), 1);

//         let patches = get_patches(&diff).unwrap();
//         assert_eq!(patches.len(), 1);
//         assert_eq!(get_patch_hunks(&patches[0]).unwrap().len(), 0);
//     }

//     #[test]
//     fn test_multiple_hunks() {
//         let (_path, repo) = repo_init().unwrap();

//         write_commit_file(&repo, "test.txt", FILE_1, "commit1");

//         repo_write_file(&repo, "test.txt", FILE_2);

//         let diff = get_diff_raw(&repo, "", false, false).unwrap();
//         assert_eq!(diff.stats().unwrap().files_changed(), 1);

//         let patches = get_patches(&diff).unwrap();
//         assert_eq!(patches.len(), 1);

//         let hunks = get_patch_hunks(&patches[0]).unwrap();
//         assert_eq!(hunks.len(), 2);
//     }
// }
