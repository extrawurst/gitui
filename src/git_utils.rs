use git2::{
    DiffFormat, DiffOptions, Repository, StatusOptions, StatusShow,
};
use std::path::Path;

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
#[derive(Default, PartialEq)]
pub struct DiffLine {
    pub content: String,
    pub line_type: DiffLineType,
}

///
#[derive(Default, PartialEq)]
pub struct Diff(pub Vec<DiffLine>);

///
pub fn get_diff(p: &Path) -> Diff {
    let repo = repo();

    let mut opt = DiffOptions::new();
    opt.pathspec(p);

    let diff =
        repo.diff_index_to_workdir(None, Some(&mut opt)).unwrap();

    let mut res = Vec::new();

    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
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
        true
    })
    .unwrap();

    Diff(res)
}

///
pub fn repo() -> Repository {
    let repo = Repository::init("./").unwrap();

    if repo.is_bare() {
        panic!("bare repo")
    }

    repo
}

///
pub fn commit(msg: &String) {
    let repo = repo();

    let signature = repo.signature().unwrap();
    let reference = repo.head().unwrap();
    let mut index = repo.index().unwrap();
    let tree_id = index.write_tree().unwrap();

    let tree = repo.find_tree(tree_id).unwrap();
    let parent =
        repo.find_commit(reference.target().unwrap()).unwrap();

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        msg.as_str(),
        &tree,
        &[&parent],
    )
    .unwrap();
}

///
pub fn index_empty() -> bool {
    let repo = repo();

    let statuses = repo
        .statuses(Some(
            StatusOptions::default().show(StatusShow::Index),
        ))
        .unwrap();

    statuses.is_empty()
}
