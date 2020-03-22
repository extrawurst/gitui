use crate::utils;
use git2::{DiffFormat, DiffOptions};
use scopetime::scope_time;

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
        // diff against stage
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
