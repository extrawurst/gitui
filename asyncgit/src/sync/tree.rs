use super::utils::bytes2string;
use crate::{error::Result, sync::utils::repo};
use git2::{Repository, Tree};
use scopetime::scope_time;
use std::path::{Path, PathBuf};

///
pub fn tree_files(repo_path: &str) -> Result<Vec<String>> {
    scope_time!("tree_files");

    let repo = repo(repo_path)?;

    let tree = repo.head()?.peel_to_tree()?;

    let mut files: Vec<String> = Vec::new();

    tree_recurse(&repo, &PathBuf::from("./"), &tree, &mut files)?;

    Ok(files)
}

///
fn tree_recurse(
    repo: &Repository,
    path: &Path,
    tree: &Tree,
    out: &mut Vec<String>,
) -> Result<()> {
    out.reserve(tree.len());

    for e in tree {
        let path = path.join(bytes2string(e.name_bytes())?);
        match e.kind() {
            Some(git2::ObjectType::Blob) => {
                // log::info!("file: {:?}", path);
                if let Some(n) = path.to_str() {
                    out.push(n.to_string());
                }
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

        write_commit_file(&repo, "test.txt", "", "c1");

        let files = tree_files(repo_path).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(&files[0], "./test.txt");
    }
}
