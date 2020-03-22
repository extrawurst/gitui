use git2::{
    build::CheckoutBuilder, DiffFormat, DiffOptions, IndexAddOption,
    ObjectType, Repository, RepositoryOpenFlags,
};
use scopetime::scope_time;
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
pub fn get_diff(p: &Path, stage: bool) -> Diff {
    scope_time!("get_diff");

    let repo = repo();

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

///
pub fn repo() -> Repository {
    let repo = Repository::open_ext(
        "./",
        RepositoryOpenFlags::empty(),
        Vec::<&Path>::new(),
    )
    .unwrap();

    if repo.is_bare() {
        panic!("bare repo")
    }

    repo
}

///
pub fn commit(msg: &String) {
    scope_time!("commit");

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

pub fn stage_add(path: &Path) -> bool {
    scope_time!("stage_add");

    let repo = repo();

    let mut index = repo.index().unwrap();

    let cb = &mut |p: &Path, _matched_spec: &[u8]| -> i32 {
        if p == path {
            0
        } else {
            1
        }
    };
    let cb = Some(cb as &mut git2::IndexMatchedPath);

    let flags = IndexAddOption::DISABLE_PATHSPEC_MATCH
        | IndexAddOption::CHECK_PATHSPEC;

    if let Ok(_) = index.add_all(path, flags, cb) {
        index.write().unwrap();
        return true;
    }

    false
}

pub fn stage_reset(path: &Path) -> bool {
    scope_time!("stage_reset");

    let repo = repo();

    let reference = repo.head().unwrap();
    let obj = repo
        .find_object(
            reference.target().unwrap(),
            Some(ObjectType::Commit),
        )
        .unwrap();

    if let Ok(_) = repo.reset_default(Some(&obj), &[path]) {
        return true;
    }

    false
}

pub fn index_reset(path: &Path) -> bool {
    scope_time!("index_reset");

    let repo = repo();

    let mut checkout_opts = CheckoutBuilder::new();
    checkout_opts.path(&path).force();

    if let Ok(_) = repo.checkout_head(Some(&mut checkout_opts)) {
        return true;
    }

    false
}
