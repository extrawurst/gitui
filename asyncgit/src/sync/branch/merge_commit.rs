//! merging from upstream

use super::BranchType;
use crate::{
    error::{Error, Result},
    sync::{utils, CommitId},
};
use git2::MergeOptions;
use scopetime::scope_time;

/// merge upstream using a merge commit without conflicts. fails if not possible without conflicts
pub fn merge_upstream_commit(
    repo_path: &str,
    branch_name: &str,
) -> Result<CommitId> {
    scope_time!("merge_upstream_commit");

    let repo = utils::repo(repo_path)?;

    let branch = repo.find_branch(branch_name, BranchType::Local)?;
    let upstream = branch.upstream()?;

    let upstream_commit = upstream.get().peel_to_commit()?;

    let annotated_upstream =
        repo.find_annotated_commit(upstream_commit.id())?;

    let (analysis, _) =
        repo.merge_analysis(&[&annotated_upstream])?;

    if !analysis.is_normal() {
        return Err(Error::Generic(
            "normal merge not possible".into(),
        ));
    }

    //TODO: support merge on unborn
    if analysis.is_unborn() {
        return Err(Error::Generic("head is unborn".into()));
    }

    let mut opt = MergeOptions::default();
    opt.fail_on_conflict(true);

    repo.merge(&[&annotated_upstream], Some(&mut opt), None)?;

    assert!(!repo.index()?.has_conflicts());

    let signature =
        crate::sync::commit::signature_allow_undefined_name(&repo)?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let head_commit = repo.find_commit(
        crate::sync::utils::get_head_repo(&repo)?.into(),
    )?;
    let parents = vec![&head_commit, &upstream_commit];

    //find remote url for this branch
    let remote_url = {
        let branch_refname =
            branch.get().name().ok_or_else(|| {
                Error::Generic(String::from(
                    "branch refname not found",
                ))
            })?;
        let buf = repo.branch_upstream_remote(branch_refname)?;
        let remote =
            repo.find_remote(buf.as_str().ok_or_else(|| {
                Error::Generic(String::from("remote name not found"))
            })?)?;
        remote.url().unwrap_or_default().to_string()
    };

    let commit_id = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            format!("Merge '{}' of {}", branch_name, remote_url)
                .as_str(),
            &tree,
            parents.as_slice(),
        )?
        .into();

    repo.cleanup_state()?;

    Ok(commit_id)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sync::{
        branch_compare_upstream,
        remotes::{fetch_origin, push::push},
        tests::{
            debug_cmd_print, get_commit_ids, repo_clone,
            repo_init_bare, write_commit_file,
        },
        RepoState,
    };

    #[test]
    fn test_merge_normal() {
        let (r1_dir, _repo) = repo_init_bare().unwrap();

        let (clone1_dir, clone1) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        let (clone2_dir, clone2) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        let clone2_dir = clone2_dir.path().to_str().unwrap();

        // clone1

        let commit1 =
            write_commit_file(&clone1, "test.txt", "test", "commit1");

        push(
            clone1_dir.path().to_str().unwrap(),
            "origin",
            "master",
            false,
            None,
            None,
        )
        .unwrap();

        // clone2

        let commit2 = write_commit_file(
            &clone2,
            "test2.txt",
            "test",
            "commit2",
        );

        //push should fail since origin diverged
        assert!(push(
            clone2_dir, "origin", "master", false, None, None,
        )
        .is_err());

        //lets fetch from origin
        let bytes =
            fetch_origin(clone2_dir, "master", None, None).unwrap();
        assert!(bytes > 0);

        //we should be one commit behind
        assert_eq!(
            branch_compare_upstream(clone2_dir, "master")
                .unwrap()
                .behind,
            1
        );

        let merge_commit =
            merge_upstream_commit(clone2_dir, "master").unwrap();

        let state = crate::sync::repo_state(clone2_dir).unwrap();

        assert_eq!(state, RepoState::Clean);

        let commits = get_commit_ids(&clone2, 10);
        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0], merge_commit);
        assert_eq!(commits[1], commit2);
        assert_eq!(commits[2], commit1);

        //verify commit msg
        let details =
            crate::sync::get_commit_details(clone2_dir, merge_commit)
                .unwrap();
        assert_eq!(
            details.message.unwrap().combine(),
            format!(
                "Merge 'master' of {}",
                r1_dir.path().to_str().unwrap()
            )
        );
    }

    #[test]
    fn test_merge_normal_conflict() {
        let (r1_dir, _repo) = repo_init_bare().unwrap();

        let (clone1_dir, clone1) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        let (clone2_dir, clone2) =
            repo_clone(r1_dir.path().to_str().unwrap()).unwrap();

        // clone1

        write_commit_file(&clone1, "test.bin", "test", "commit1");

        debug_cmd_print(
            clone2_dir.path().to_str().unwrap(),
            "git status",
        );

        push(
            clone1_dir.path().to_str().unwrap(),
            "origin",
            "master",
            false,
            None,
            None,
        )
        .unwrap();

        // clone2

        write_commit_file(&clone2, "test.bin", "foobar", "commit2");

        let bytes = fetch_origin(
            clone2_dir.path().to_str().unwrap(),
            "master",
            None,
            None,
        )
        .unwrap();
        assert!(bytes > 0);

        let res = merge_upstream_commit(
            clone2_dir.path().to_str().unwrap(),
            "master",
        );

        //this should have failed cause it would create a conflict
        assert!(res.is_err());

        let state = crate::sync::repo_state(
            clone2_dir.path().to_str().unwrap(),
        )
        .unwrap();

        //make sure we left the repo not in some merging state
        assert_eq!(state, RepoState::Clean);

        //check that we still only have the first commit
        let commits = get_commit_ids(&clone1, 10);
        assert_eq!(commits.len(), 1);
    }
}
