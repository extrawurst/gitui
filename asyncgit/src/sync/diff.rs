//! sync git api for fetching a diff

use super::{commit_files::get_commit_diff, utils, CommitId};
use crate::{error::Error, error::Result, hash};
use git2::{
    Delta, Diff, DiffDelta, DiffFormat, DiffHunk, DiffOptions, Patch,
    Repository,
};
use scopetime::scope_time;
use std::{cell::RefCell, fs, path::Path, rc::Rc};
use utils::{get_head_repo, work_dir};

/// type of diff of a single line
#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum DiffLineType {
    /// just surrounding line, no change
    None,
    /// header of the hunk
    Header,
    /// line added
    Add,
    /// line deleted
    Delete,
}

impl Default for DiffLineType {
    fn default() -> Self {
        DiffLineType::None
    }
}

///
#[derive(Default, Clone, Hash, Debug)]
pub struct DiffLine {
    ///
    pub content: String,
    ///
    pub line_type: DiffLineType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Hash)]
pub(crate) struct HunkHeader {
    old_start: u32,
    old_lines: u32,
    new_start: u32,
    new_lines: u32,
}

impl From<DiffHunk<'_>> for HunkHeader {
    fn from(h: DiffHunk) -> Self {
        Self {
            old_start: h.old_start(),
            old_lines: h.old_lines(),
            new_start: h.new_start(),
            new_lines: h.new_lines(),
        }
    }
}

/// single diff hunk
#[derive(Default, Clone, Hash, Debug)]
pub struct Hunk {
    /// hash of the hunk header
    pub header_hash: u64,
    /// list of `DiffLine`s
    pub lines: Vec<DiffLine>,
}

/// collection of hunks, sum of all diff lines
#[derive(Default, Clone, Hash, Debug)]
pub struct FileDiff {
    /// list of hunks
    pub hunks: Vec<Hunk>,
    /// lines total summed up over hunks
    pub lines: usize,
    ///
    pub untracked: bool,
    /// old and new file size in bytes
    pub sizes: (u64, u64),
    /// size delta in bytes
    pub size_delta: i64,
}

pub(crate) fn get_diff_raw<'a>(
    repo: &'a Repository,
    p: &str,
    stage: bool,
    reverse: bool,
) -> Result<Diff<'a>> {
    // scope_time!("get_diff_raw");

    let mut opt = DiffOptions::new();
    opt.pathspec(p);
    opt.reverse(reverse);

    let diff = if stage {
        // diff against head
        if let Ok(id) = get_head_repo(&repo) {
            let parent = repo.find_commit(id.into())?;

            let tree = parent.tree()?;
            repo.diff_tree_to_index(
                Some(&tree),
                Some(&repo.index()?),
                Some(&mut opt),
            )?
        } else {
            repo.diff_tree_to_index(
                None,
                Some(&repo.index()?),
                Some(&mut opt),
            )?
        }
    } else {
        opt.include_untracked(true);
        opt.recurse_untracked_dirs(true);
        repo.diff_index_to_workdir(None, Some(&mut opt))?
    };

    Ok(diff)
}

/// returns diff of a specific file either in `stage` or workdir
pub fn get_diff(
    repo_path: &str,
    p: String,
    stage: bool,
) -> Result<FileDiff> {
    scope_time!("get_diff");

    let repo = utils::repo(repo_path)?;
    let work_dir = work_dir(&repo);
    let diff = get_diff_raw(&repo, &p, stage, false)?;

    raw_diff_to_file_diff(&diff, work_dir)
}

/// returns diff of a specific file inside a commit
/// see `get_commit_diff`
pub fn get_diff_commit(
    repo_path: &str,
    id: CommitId,
    p: String,
) -> Result<FileDiff> {
    scope_time!("get_diff_commit");

    let repo = utils::repo(repo_path)?;
    let work_dir = work_dir(&repo);
    let diff = get_commit_diff(&repo, id, Some(p))?;

    raw_diff_to_file_diff(&diff, work_dir)
}

///
fn raw_diff_to_file_diff<'a>(
    diff: &'a Diff,
    work_dir: &Path,
) -> Result<FileDiff> {
    let res = Rc::new(RefCell::new(FileDiff::default()));
    {
        let mut current_lines = Vec::new();
        let mut current_hunk: Option<HunkHeader> = None;

        let res_cell = Rc::clone(&res);
        let adder = move |header: &HunkHeader,
                          lines: &Vec<DiffLine>| {
            let mut res = res_cell.borrow_mut();
            res.hunks.push(Hunk {
                header_hash: hash(header),
                lines: lines.clone(),
            });
            res.lines += lines.len();
        };

        let res_cell = Rc::clone(&res);
        let mut put = |delta: DiffDelta,
                       hunk: Option<DiffHunk>,
                       line: git2::DiffLine| {
            {
                let mut res = res_cell.borrow_mut();
                res.sizes = (
                    delta.old_file().size(),
                    delta.new_file().size(),
                );
                res.size_delta = (res.sizes.1 as i64)
                    .saturating_sub(res.sizes.0 as i64);
            }
            if let Some(hunk) = hunk {
                let hunk_header = HunkHeader::from(hunk);

                match current_hunk {
                    None => current_hunk = Some(hunk_header),
                    Some(h) if h != hunk_header => {
                        adder(&h, &current_lines);
                        current_lines.clear();
                        current_hunk = Some(hunk_header)
                    }
                    _ => (),
                }

                let line_type = match line.origin() {
                    'H' => DiffLineType::Header,
                    '<' | '-' => DiffLineType::Delete,
                    '>' | '+' => DiffLineType::Add,
                    _ => DiffLineType::None,
                };

                let diff_line = DiffLine {
                    content: String::from_utf8_lossy(line.content())
                        .to_string(),
                    line_type,
                };

                current_lines.push(diff_line);
            }
        };

        let new_file_diff = if diff.deltas().len() == 1 {
            // it's safe to unwrap here because we check first that diff.deltas has a single element.
            let delta: DiffDelta = diff.deltas().next().unwrap();

            if delta.status() == Delta::Untracked {
                let relative_path =
                    delta.new_file().path().ok_or_else(|| {
                        Error::Generic(
                            "new file path is unspecified."
                                .to_string(),
                        )
                    })?;

                let newfile_path = work_dir.join(relative_path);

                if let Some(newfile_content) =
                    new_file_content(&newfile_path)
                {
                    let mut patch = Patch::from_buffers(
                        &[],
                        None,
                        newfile_content.as_bytes(),
                        Some(&newfile_path),
                        None,
                    )?;

                    patch
                    .print(&mut |delta, hunk:Option<DiffHunk>, line: git2::DiffLine| {
                        put(delta,hunk,line);
                        true
                    })?;

                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !new_file_diff {
            diff.print(
                DiffFormat::Patch,
                move |delta, hunk, line: git2::DiffLine| {
                    put(delta, hunk, line);
                    true
                },
            )?;
        }

        if !current_lines.is_empty() {
            adder(&current_hunk.unwrap(), &current_lines);
        }

        if new_file_diff {
            res.borrow_mut().untracked = true;
        }
    }
    let res = Rc::try_unwrap(res).expect("rc error");
    Ok(res.into_inner())
}

fn new_file_content(path: &Path) -> Option<String> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            if let Ok(path) = fs::read_link(path) {
                return Some(path.to_str()?.to_string());
            }
        } else if meta.file_type().is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                return Some(content);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{get_diff, get_diff_commit};
    use crate::error::Result;
    use crate::sync::{
        commit, stage_add_file,
        status::{get_status, StatusType},
        tests::{get_statuses, repo_init, repo_init_empty},
        CommitId,
    };
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    #[test]
    fn test_untracked_subfolder() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));

        fs::create_dir(&root.join("foo")).unwrap();
        File::create(&root.join("foo/bar.txt"))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));

        let diff =
            get_diff(repo_path, "foo/bar.txt".to_string(), false)
                .unwrap();

        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].lines[1].content, "test\n");
    }

    #[test]
    fn test_empty_repo() {
        let file_path = Path::new("foo.txt");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(get_statuses(repo_path), (1, 0));

        stage_add_file(repo_path, file_path).unwrap();

        assert_eq!(get_statuses(repo_path), (0, 1));

        let diff = get_diff(
            repo_path,
            String::from(file_path.to_str().unwrap()),
            true,
        )
        .unwrap();

        assert_eq!(diff.hunks.len(), 1);
    }

    static HUNK_A: &str = r"
1   start
2
3
4
5
6   middle
7
8
9
0
1   end";

    static HUNK_B: &str = r"
1   start
2   newa
3
4
5
6   middle
7
8
9
0   newb
1   end";

    #[test]
    fn test_hunks() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_statuses(repo_path), (0, 0));

        let file_path = root.join("bar.txt");

        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_A.as_bytes())
                .unwrap();
        }

        let res = get_status(repo_path, StatusType::WorkingDir, true)
            .unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].path, "bar.txt");

        stage_add_file(repo_path, Path::new("bar.txt")).unwrap();
        assert_eq!(get_statuses(repo_path), (0, 1));

        // overwrite with next content
        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_B.as_bytes())
                .unwrap();
        }

        assert_eq!(get_statuses(repo_path), (1, 1));

        let res = get_diff(repo_path, "bar.txt".to_string(), false)
            .unwrap();

        assert_eq!(res.hunks.len(), 2)
    }

    #[test]
    fn test_diff_newfile_in_sub_dir_current_dir() {
        let file_path = Path::new("foo/foo.txt");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();

        let sub_path = root.join("foo/");

        fs::create_dir_all(&sub_path).unwrap();
        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test")
            .unwrap();

        let diff = get_diff(
            sub_path.to_str().unwrap(),
            String::from(file_path.to_str().unwrap()),
            false,
        )
        .unwrap();

        assert_eq!(diff.hunks[0].lines[1].content, "test");
    }

    #[test]
    fn test_diff_new_binary_file_using_invalid_utf8() -> Result<()> {
        let file_path = Path::new("bar");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"\xc3\x28")?;

        let diff = get_diff(
            repo_path,
            String::from(file_path.to_str().unwrap()),
            false,
        )
        .unwrap();

        assert_eq!(diff.hunks.len(), 0);

        Ok(())
    }

    #[test]
    fn test_diff_delta_size() -> Result<()> {
        let file_path = Path::new("bar");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"\x00")?;

        stage_add_file(repo_path, file_path).unwrap();

        commit(repo_path, "commit").unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"\x00\x02")?;

        let diff = get_diff(
            repo_path,
            String::from(file_path.to_str().unwrap()),
            false,
        )
        .unwrap();

        dbg!(&diff);
        assert_eq!(diff.sizes, (1, 2));
        assert_eq!(diff.size_delta, 1);

        Ok(())
    }

    #[test]
    fn test_diff_delta_size_commit() -> Result<()> {
        let file_path = Path::new("bar");
        let (_td, repo) = repo_init_empty().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        File::create(&root.join(file_path))?.write_all(b"\x00")?;

        stage_add_file(repo_path, file_path).unwrap();

        commit(repo_path, "").unwrap();

        File::create(&root.join(file_path))?
            .write_all(b"\x00\x02")?;

        stage_add_file(repo_path, file_path).unwrap();

        let id = commit(repo_path, "").unwrap();

        let diff = get_diff_commit(
            repo_path,
            CommitId::new(id),
            String::new(),
        )
        .unwrap();

        dbg!(&diff);
        assert_eq!(diff.sizes, (1, 2));
        assert_eq!(diff.size_delta, 1);

        Ok(())
    }
}
