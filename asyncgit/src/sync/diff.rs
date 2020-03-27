//! sync git api for fetching a diff

use super::utils;
use git2::{
    Delta, DiffDelta, DiffFormat, DiffHunk, DiffOptions, Patch,
};
use scopetime::scope_time;
use std::fs;

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

///
#[derive(Default, Clone, Copy, PartialEq)]
struct HunkHeader {
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
pub struct Hunk(pub Vec<DiffLine>);

///
#[derive(Default, Clone, Hash)]
pub struct Diff(pub Vec<Hunk>);

///
pub fn get_diff(p: String, stage: bool) -> Diff {
    scope_time!("get_diff");

    let repo = utils::repo();

    let mut opt = DiffOptions::new();
    opt.pathspec(p);

    let diff = if stage {
        // diff against head
        let ref_head = repo.head().unwrap();
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
        opt.include_untracked(true);
        opt.recurse_untracked_dirs(true);
        repo.diff_index_to_workdir(None, Some(&mut opt)).unwrap()
    };

    let mut res: Diff = Diff::default();
    let mut current_lines = Vec::new();
    let mut current_hunk: Option<HunkHeader> = None;

    let mut put = |hunk: Option<DiffHunk>, line: git2::DiffLine| {
        if let Some(hunk) = hunk {
            let hunk_header = HunkHeader::from(hunk);

            match current_hunk {
                None => current_hunk = Some(hunk_header),
                Some(h) if h != hunk_header => {
                    res.0.push(Hunk(current_lines.clone()));
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
            let newfile_path = delta.new_file().path().unwrap();

            let newfile_content =
                fs::read_to_string(newfile_path).unwrap();

            let mut patch = Patch::from_buffers(
                &[],
                None,
                newfile_content.as_bytes(),
                Some(newfile_path),
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
        res.0.push(Hunk(current_lines))
    }

    res
}

#[cfg(test)]
mod tests {
    use super::get_diff;
    use crate::sync::{
        stage_add,
        status::{get_index, StatusType},
    };
    use git2::Repository;
    use std::env;
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };
    use tempfile::TempDir;

    pub fn repo_init() -> (TempDir, Repository) {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "name").unwrap();
            config.set_str("user.email", "email").unwrap();

            let mut index = repo.index().unwrap();
            let id = index.write_tree().unwrap();

            let tree = repo.find_tree(id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "initial",
                &tree,
                &[],
            )
            .unwrap();
        }
        (td, repo)
    }

    #[test]
    fn test_untracked_subfolder() {
        let (_td, repo) = repo_init();
        let root = repo.path().parent().unwrap();

        assert!(env::set_current_dir(&root).is_ok());

        let res = get_index(StatusType::WorkingDir);
        assert_eq!(res.len(), 0);

        fs::create_dir(&root.join("foo")).unwrap();
        File::create(&root.join("foo/bar.txt"))
            .unwrap()
            .write_all(b"test\nfoo")
            .unwrap();

        let res = get_index(StatusType::WorkingDir);
        assert_eq!(res.len(), 1);

        let diff = get_diff("foo/bar.txt".to_string(), false);

        assert_eq!(diff.0.len(), 1);
        assert_eq!(diff.0[0].0[1].content, "test\n");
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

        //TODO: this makes the test not threading safe
        assert!(env::set_current_dir(&root).is_ok());

        let res = get_index(StatusType::WorkingDir);
        assert_eq!(res.len(), 0);

        let file_path = root.join("bar.txt");

        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_A.as_bytes())
                .unwrap();
        }

        let res = get_index(StatusType::WorkingDir);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].path, "bar.txt");

        let res = stage_add(Path::new("bar.txt"));
        assert_eq!(res, true);
        assert_eq!(get_index(StatusType::Stage).len(), 1);
        assert_eq!(get_index(StatusType::WorkingDir).len(), 0);

        // overwrite with next content
        {
            File::create(&file_path)
                .unwrap()
                .write_all(HUNK_B.as_bytes())
                .unwrap();
        }

        assert_eq!(get_index(StatusType::Stage).len(), 1);
        assert_eq!(get_index(StatusType::WorkingDir).len(), 1);

        let res = get_diff("bar.txt".to_string(), false);

        assert_eq!(res.0.len(), 2)
    }
}
