use super::utils::bytes2string;
use crate::{error::Result, sync::utils::repo};
use git2::{Oid, Repository, Tree};
use scopetime::scope_time;
use std::path::{Path, PathBuf};

pub struct TreeFile {
    pub path: PathBuf,
    pub filemode: i32,
    id: Oid,
}

///
//TODO: allow any commit
pub fn tree_files(repo_path: &str) -> Result<Vec<TreeFile>> {
    scope_time!("tree_files");

    let repo = repo(repo_path)?;

    let tree = repo.head()?.peel_to_tree()?;

    let mut files: Vec<TreeFile> = Vec::new();

    tree_recurse(&repo, &PathBuf::from("./"), &tree, &mut files)?;

    Ok(files)
}

///
pub fn tree_file_content(
    repo_path: &str,
    file: &TreeFile,
) -> Result<String> {
    scope_time!("tree_file_content");

    let repo = repo(repo_path)?;

    let blob = repo.find_blob(file.id)?;
    let content = String::from_utf8(blob.content().into())?;

    Ok(content)
}

///
fn tree_recurse(
    repo: &Repository,
    path: &Path,
    tree: &Tree,
    out: &mut Vec<TreeFile>,
) -> Result<()> {
    out.reserve(tree.len());

    for e in tree {
        let path = path.join(bytes2string(e.name_bytes())?);
        match e.kind() {
            Some(git2::ObjectType::Blob) => {
                let id = e.id();
                let filemode = e.filemode();
                out.push(TreeFile { path, filemode, id });
            }
            Some(git2::ObjectType::Tree) => {
                let obj = e.to_object(repo)?;
                let tree = obj.peel_to_tree()?;
                tree_recurse(repo, &path, &tree, out)?;
            }
            Some(_) | None => (),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::tests::{repo_init, write_commit_file};

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        write_commit_file(&repo, "test.txt", "content", "c1");

        let files = tree_files(repo_path).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("./test.txt"));

        write_commit_file(&repo, "test.txt", "content2", "c2");

        let content =
            tree_file_content(repo_path, &files[0]).unwrap();
        assert_eq!(&content, "content");
    }
}
