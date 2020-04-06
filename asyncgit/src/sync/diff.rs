//! sync git api for fetching a diff

use super::utils;
use crate::hash;
use git2::{
    Delta, Diff, DiffDelta, DiffFormat, DiffHunk, DiffOptions, Patch,
    Repository,
};
use scopetime::scope_time;
use std::{fs, path::Path};

///
#[derive(Copy, Clone, PartialEq, Hash)]
pub enum DiffLineType {
    ///
    None,
    ///
    Header,
    ///
    Add,
    ///
    Delete,
}

impl Default for DiffLineType {
    fn default() -> Self {
        DiffLineType::None
    }
}

///
#[derive(Default, Clone, Hash)]
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

///
#[derive(Default, Clone, Hash)]
pub struct Hunk {
    ///
    pub header_hash: u64,
    ///
    pub lines: Vec<DiffLine>,
}

///
#[derive(Default, Clone, Hash)]
pub struct FileDiff {
    /// list of hunks
    pub hunks: Vec<Hunk>,
    /// lines total summed up over hunks
    pub lines: u16,
}

pub(crate) fn get_diff_raw<'a>(
    repo: &'a Repository,
    p: &str,
    stage: bool,
    reverse: bool,
) -> (Diff<'a>, DiffOptions) {
    let mut opt = DiffOptions::new();
    opt.pathspec(p);
    opt.reverse(reverse);

    let diff = if stage {
        // diff against head
        if let Ok(ref_head) = repo.head() {
            let parent =
                repo.find_commit(ref_head.target().unwrap()).unwrap();
            let tree = parent.tree().unwrap();
            repo.diff_tree_to_index(
                Some(&tree),
                Some(&repo.index().unwrap()),
                Some(&mut opt),
            )
            .unwrap()
        } else {
            repo.diff_tree_to_index(
                None,
                Some(&repo.index().unwrap()),
                Some(&mut opt),
            )
            .unwrap()
        }
    } else {
        opt.include_untracked(true);
        opt.recurse_untracked_dirs(true);
        repo.diff_index_to_workdir(None, Some(&mut opt)).unwrap()
    };

    (diff, opt)
}

///
pub fn get_diff(repo_path: &str, p: String, stage: bool) -> FileDiff {
    scope_time!("get_diff");

    let repo = utils::repo(repo_path);

    let (diff, mut opt) = get_diff_raw(&repo, &p, stage, false);

    let mut res: FileDiff = FileDiff::default();
    let mut current_lines = Vec::new();
    let mut current_hunk: Option<HunkHeader> = None;

    let mut adder = |header: &HunkHeader, lines: &Vec<DiffLine>| {
        res.hunks.push(Hunk {
            header_hash: hash(header),
            lines: lines.clone(),
        });
        res.lines += lines.len() as u16;
    };

    let mut put = |hunk: Option<DiffHunk>, line: git2::DiffLine| {
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
        let delta: DiffDelta = diff.deltas().next().unwrap();

        if delta.status() == Delta::Untracked {
            let repo_path = Path::new(repo_path);
            let newfile_path =
                repo_path.join(delta.new_file().path().unwrap());

            let newfile_content = new_file_content(&newfile_path);

            let mut patch = Patch::from_buffers(
                &[],
                None,
                newfile_content.as_bytes(),
                Some(&newfile_path),
                Some(&mut opt),
            )
            .unwrap();

            patch
                .print(&mut |_delta, hunk:Option<DiffHunk>, line: git2::DiffLine| {
                    put(hunk,line);
                    true
                })
                .unwrap();

            true
        } else {
            false
        }
    } else {
        false
    };

    if !new_file_diff {
        diff.print(
            DiffFormat::Patch,
            |_, hunk, line: git2::DiffLine| {
                put(hunk, line);
                true
            },
        )
        .unwrap();
    }

    if !current_lines.is_empty() {
        adder(&current_hunk.unwrap(), &current_lines);
    }

    res
}

fn new_file_content(path: &Path) -> String {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return fs::read_link(path)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
        } else if meta.file_type().is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                return content;
            }
        }
    }

    "file not found".to_string()
}

#[cfg(test)]
mod tests {
    use super::get_diff;
    use crate::sync::{
        stage_add,
        status::{get_status, StatusType},
        tests::{repo_init, repo_init_empty},
    };
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    #[test]
    fn test_untracked_subfolder() {
        let (_td, repo) = repo_init();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let res = get_status(repo_path, StatusType::WorkingDir);
        assert_eq!(res.len(), 0);

        fs::create_dir(&root.join("foo")).unwrap();
        File::create(&root.join("foo/bar.txt"))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        let res = get_status(repo_path, StatusType::WorkingDir);
        assert_eq!(res.len(), 1);

        let diff =
            get_diff(repo_path, "foo/bar.txt".to_string(), false);

        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.hunks[0].lines[1].content, "test\n");
    }

    #[test]
    fn test_empty_repo() {
        let file_path = Path::new("foo.txt");
        let (_td, repo) = repo_init_empty();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let res = get_status(repo_path, StatusType::WorkingDir);
        assert_eq!(res.len(), 0);

        File::create(&root.join(file_path))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        assert_eq!(stage_add(repo_path, file_path), true);

        let diff = get_diff(
            repo_path,
            String::from(file_path.to_str().unwrap()),
            true,
        );

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
        let (_td, repo) = repo_init();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let res = get_status(repo_path, StatusType::WorkingDir);
        assert_eq!(res.len(), 0);

        let file_path = root.join("bar.txt");

        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_A.as_bytes())
                .unwrap();
        }

        let res = get_status(repo_path, StatusType::WorkingDir);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].path, "bar.txt");

        let res = stage_add(repo_path, Path::new("bar.txt"));
        assert_eq!(res, true);
        assert_eq!(get_status(repo_path, StatusType::Stage).len(), 1);
        assert_eq!(
            get_status(repo_path, StatusType::WorkingDir).len(),
            0
        );

        // overwrite with next content
        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_B.as_bytes())
                .unwrap();
        }

        assert_eq!(get_status(repo_path, StatusType::Stage).len(), 1);
        assert_eq!(
            get_status(repo_path, StatusType::WorkingDir).len(),
            1
        );

        let res = get_diff(repo_path, "bar.txt".to_string(), false);

        assert_eq!(res.hunks.len(), 2)
    }
}
