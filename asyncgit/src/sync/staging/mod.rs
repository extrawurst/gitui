use super::{
    diff::DiffLinePosition,
    patches::{get_file_diff_patch_and_hunklines, HunkLines},
    utils::{repo, work_dir},
};
use crate::error::{Error, Result};
use git2::{DiffLine, Repository};
use scopetime::scope_time;
use std::{
    convert::TryFrom,
    fs::File,
    io::{Read, Write},
};

/// discards specific lines in an unstaged hunk of a diff
pub fn discard_lines(
    repo_path: &str,
    file_path: &str,
    lines: &[DiffLinePosition],
) -> Result<()> {
    scope_time!("discard_lines");

    if lines.is_empty() {
        return Ok(());
    }

    log::debug!("discard_lines: {}", lines.len());
    lines
        .iter()
        .for_each(|line| log::debug!("line: {:?}", line));

    let repo = repo(repo_path)?;

    //TODO: check that file is not new (status modified)

    let (_patch, hunks) = get_file_diff_patch_and_hunklines(
        &repo, file_path, false, false,
    )?;

    // let mut index = repo.index()?;
    // index.read(true)?;
    // let idx = index
    //     .get_path(Path::new(file_path), 0)
    //     .expect("only non new files supported");
    // let blob = repo.find_blob(idx.id)?;
    // let indexed_content =
    //     String::from_utf8(blob.content().into()).unwrap();

    let working_content = load_file(&repo, file_path)?;
    let old_lines = working_content.lines().collect::<Vec<_>>();

    let new_content =
        apply_selection(lines, &hunks, old_lines, true)?;

    repo_write_file(&repo, file_path, new_content.as_str())?;

    Ok(())
}

#[derive(Default)]
struct NewFromOldContent {
    lines: Vec<String>,
    old_index: usize,
}

impl NewFromOldContent {
    fn add_from_hunk(&mut self, line: &DiffLine) -> Result<()> {
        let line = String::from_utf8(line.content().into())?;

        let line = if line.ends_with("\n") {
            line[0..line.len() - 1].to_string()
        } else {
            line
        };

        self.lines.push(line);

        Ok(())
    }

    fn skip_old_line(&mut self) {
        self.old_index += 1;
    }

    // fn add_old_line(&mut self, old_lines: &[&str]) {
    //     self.lines.push(old_lines[self.old_index].to_string());
    //     self.old_index += 1;
    // }

    fn catchup_to_hunkstart(
        &mut self,
        hunk_start: usize,
        old_lines: &[&str],
    ) {
        while hunk_start > self.old_index + 1 {
            self.lines.push(old_lines[self.old_index].to_string());
            self.old_index += 1;
        }
    }

    fn finish(mut self, old_lines: &[&str]) -> String {
        for i in self.old_index..old_lines.len() {
            self.lines.push(old_lines[i].to_string());
        }
        let lines = self.lines.join("\n");
        if lines.ends_with("\n") {
            lines
        } else {
            let mut lines = lines;
            lines.push('\n');
            lines
        }
    }
}

fn apply_selection(
    lines: &[DiffLinePosition],
    hunks: &[HunkLines],
    old_lines: Vec<&str>,
    reverse: bool,
) -> Result<String> {
    let mut new_content = NewFromOldContent::default();
    let mut line_iter = lines.clone().into_iter();
    let current_line = line_iter.next();

    let char_deleted = if reverse { '+' } else { '-' };
    let char_added = if reverse { '-' } else { '+' };

    for hunk in hunks {
        let hunk_start = if reverse {
            usize::try_from(hunk.hunk.new_start)?
        } else {
            usize::try_from(hunk.hunk.old_start)?
        };
        if let Some(current_line) = current_line.cloned() {
            let in_hunk =
                hunk.lines.iter().any(|line| current_line == line);

            if in_hunk {
                // catchup until this hunk
                new_content
                    .catchup_to_hunkstart(hunk_start, &old_lines);

                for hunk_line in &hunk.lines {
                    let selected_line =
                        lines.iter().any(|line| *line == hunk_line);

                    log::debug!(
                        // print!(
                        "{} line: {} [{:?} old, {:?} new] -> {}",
                        if selected_line { "*" } else { " " },
                        hunk_line.origin(),
                        hunk_line.old_lineno(),
                        hunk_line.new_lineno(),
                        String::from_utf8_lossy(hunk_line.content())
                    );

                    if selected_line {
                        if hunk_line.origin() == char_added {
                            new_content.add_from_hunk(hunk_line)?;
                        } else if hunk_line.origin() == char_deleted {
                            new_content.skip_old_line();
                        } else {
                            todo!()
                        }
                    } else {
                        if hunk_line.origin() != char_added {
                            new_content.add_from_hunk(hunk_line)?;
                            new_content.skip_old_line();
                        }
                    }
                }
            }
        }
    }

    Ok(new_content.finish(&old_lines))
}

fn load_file(repo: &Repository, file_path: &str) -> Result<String> {
    let repo_path = repo.workdir().expect("bare repo not supported");
    let mut file = File::open(repo_path.join(file_path).as_path())?;
    let mut res = String::new();
    file.read_to_string(&mut res)?;

    Ok(res)
}

//TODO: use this in unittests instead of the test specific one
/// write a file in repo
pub(crate) fn repo_write_file(
    repo: &Repository,
    file: &str,
    content: &str,
) -> Result<()> {
    let dir = work_dir(repo)?.join(file);
    let file_path = dir.to_str().ok_or_else(|| {
        Error::Generic(String::from("invalid file path"))
    })?;
    File::create(file_path)?.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sync::tests::{
        repo_init, repo_write_file, write_commit_file,
    };

    #[test]
    fn test_discard() {
        static FILE_1: &str = r"0
1
2
3
4
";

        static FILE_2: &str = r"0


3
4
";

        static FILE_3: &str = r"0
2

3
4
";

        let (path, repo) = repo_init().unwrap();
        let path = path.path().to_str().unwrap();

        write_commit_file(&repo, "test.txt", FILE_1, "c1");

        repo_write_file(&repo, "test.txt", FILE_2);

        discard_lines(
            path,
            "test.txt",
            &[
                DiffLinePosition {
                    old_lineno: Some(3),
                    new_lineno: None,
                },
                DiffLinePosition {
                    old_lineno: None,
                    new_lineno: Some(2),
                },
            ],
        )
        .unwrap();

        let result_file = load_file(&repo, "test.txt").unwrap();

        assert_eq!(result_file.as_str(), FILE_3);
    }

    #[test]
    fn test_discard2() {
        static FILE_1: &str = r"start
end
";

        static FILE_2: &str = r"start
1
2
end
";

        static FILE_3: &str = r"start
1
end
";

        let (path, repo) = repo_init().unwrap();
        let path = path.path().to_str().unwrap();

        write_commit_file(&repo, "test.txt", FILE_1, "c1");

        repo_write_file(&repo, "test.txt", FILE_2);

        discard_lines(
            path,
            "test.txt",
            &[DiffLinePosition {
                old_lineno: None,
                new_lineno: Some(3),
            }],
        )
        .unwrap();

        let result_file = load_file(&repo, "test.txt").unwrap();

        assert_eq!(result_file.as_str(), FILE_3);
    }

    #[test]
    fn test_discard3() {
        static FILE_1: &str = r"start
1
end
";

        static FILE_2: &str = r"start
2
end
";

        static FILE_3: &str = r"start
1
end
";

        let (path, repo) = repo_init().unwrap();
        let path = path.path().to_str().unwrap();

        write_commit_file(&repo, "test.txt", FILE_1, "c1");

        repo_write_file(&repo, "test.txt", FILE_2);

        discard_lines(
            path,
            "test.txt",
            &[
                DiffLinePosition {
                    old_lineno: Some(2),
                    new_lineno: None,
                },
                DiffLinePosition {
                    old_lineno: None,
                    new_lineno: Some(2),
                },
            ],
        )
        .unwrap();

        let result_file = load_file(&repo, "test.txt").unwrap();

        assert_eq!(result_file.as_str(), FILE_3);
    }
}
