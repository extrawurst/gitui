use super::utils;
use git2::{Delta, DiffDelta, DiffFormat, DiffOptions, Patch};
use scopetime::scope_time;
use std::fs;

///
#[derive(Copy, Clone, PartialEq)]
pub enum DiffLineType {
    None,
    Header,
    Add,
    Delete,
}

impl Default for DiffLineType {
    fn default() -> Self {
        DiffLineType::None
    }
}

///
#[derive(Default, PartialEq, Clone)]
pub struct DiffLine {
    pub content: String,
    pub line_type: DiffLineType,
}

///
#[derive(Default, PartialEq, Clone)]
pub struct Diff(pub Vec<DiffLine>);

///
pub fn get_diff(p: String, stage: bool) -> Diff {
    scope_time!("get_diff");

    let repo = utils::repo();

    let mut opt = DiffOptions::new();
    opt.pathspec(p);

    let diff = if !stage {
        opt.include_untracked(true);
        opt.recurse_untracked_dirs(true);
        repo.diff_index_to_workdir(None, Some(&mut opt)).unwrap()
    } else {
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
    };

    let mut res = Vec::new();

    let mut put = |line: git2::DiffLine| {
        let origin = line.origin();

        if origin != 'F' {
            let line_type = match origin {
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

            if line_type == DiffLineType::Header && res.len() > 0 {
                res.push(DiffLine {
                    content: "\n".to_string(),
                    line_type: DiffLineType::None,
                });
            }

            res.push(diff_line);
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
                .print(&mut |_delta, _hunk, line: git2::DiffLine| {
                    put(line);
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
            |_, _, line: git2::DiffLine| {
                put(line);
                true
            },
        )
        .unwrap();
    }

    Diff(res)
}

#[cfg(test)]
mod tests {
    use crate::sync::status::{get_index, StatusType};
    use git2::Repository;
    use std::env;
    use std::{
        fs::{self, File},
        io::Write,
    };
    // use std::path::Path;
    use super::get_diff;
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

        assert_eq!(diff.0.len(), 4);
        assert_eq!(diff.0[1].content, "test\n");
    }
}
