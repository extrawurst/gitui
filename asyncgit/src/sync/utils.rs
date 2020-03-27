//! sync git api (various methods)

use git2::{IndexAddOption, Repository, RepositoryOpenFlags};
use scopetime::scope_time;
use std::path::Path;

//TODO: get rid of this
///
pub fn repo() -> Repository {
    repo_at("./")
}

///
pub fn repo_at(repo_path: &str) -> Repository {
    let repo = Repository::open_ext(
        repo_path,
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
pub fn commit(msg: &str) {
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
        msg,
        &tree,
        &[&parent],
    )
    .unwrap();
}

///
pub fn stage_add(path: &Path) -> bool {
    stage_add_at("./", path)
}

///
pub fn stage_add_at(repo_path: &str, path: &Path) -> bool {
    scope_time!("stage_add");

    let repo = repo_at(repo_path);

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

    if index.add_all(path, flags, cb).is_ok() {
        index.write().unwrap();
        return true;
    }

    false
}
