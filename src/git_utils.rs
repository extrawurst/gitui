use git2::{
    build::CheckoutBuilder, IndexAddOption, ObjectType, Repository,
    RepositoryOpenFlags,
};
use scopetime::scope_time;
use std::path::Path;

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
