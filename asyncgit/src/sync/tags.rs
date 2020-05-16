use super::utils::repo;
use crate::error::Result;
use scopetime::scope_time;
use std::collections::HashMap;

/// hashmap of tag target commit hash to tag names
pub type Tags = HashMap<String, Vec<String>>;

/// returns `Tags` type filled with all tags found in repo
pub fn get_tags(repo_path: &str) -> Result<Tags> {
    scope_time!("get_tags");

    let mut res = Tags::new();
    let mut adder = |key: String, value: String| {
        if let Some(key) = res.get_mut(&key) {
            key.push(value)
        } else {
            res.insert(key, vec![value]);
        }
    };

    let repo = repo(repo_path)?;

    for name in repo.tag_names(None)?.iter() {
        if let Some(name) = name {
            let obj = repo.revparse_single(name)?;

            if let Some(tag) = obj.as_tag() {
                let target_hash = tag.target_id().to_string();
                let tag_name = String::from(name);
                adder(target_hash, tag_name);
            }
        }
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::tests::repo_init;
    use git2::ObjectType;

    #[test]
    fn test_smoke() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        assert_eq!(get_tags(repo_path).unwrap().is_empty(), true);
    }

    #[test]
    fn test_multitags() {
        let (_td, repo) = repo_init().unwrap();
        let root = repo.path().parent().unwrap();
        let repo_path = root.as_os_str().to_str().unwrap();

        let sig = repo.signature().unwrap();
        let head_id = repo.head().unwrap().target().unwrap();
        let target = repo
            .find_object(
                repo.head().unwrap().target().unwrap(),
                Some(ObjectType::Commit),
            )
            .unwrap();

        repo.tag("a", &target, &sig, "", false).unwrap();
        repo.tag("b", &target, &sig, "", false).unwrap();

        assert_eq!(
            get_tags(repo_path).unwrap()[&head_id.to_string()],
            vec!["a", "b"]
        );
    }
}
