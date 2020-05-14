use super::utils::repo;
use crate::error::Returns;
use scopetime::scope_time;
use std::collections::HashMap;

/// hashmap of tag target commit hash to tag name
pub type Tags = HashMap<String, String>;

/// returns `Tags` type filled with all tags found in repo
pub fn get_tags(repo_path: &str) -> Returns<Tags> {
    scope_time!("get_tags");

    let mut res = Tags::new();

    let repo = repo(repo_path)?;

    for name in repo.tag_names(None)?.iter() {
        if let Some(name) = name {
            let obj = repo.revparse_single(name)?;

            if let Some(tag) = obj.as_tag() {
                let target_hash = tag.target_id().to_string();
                let tag_name = String::from(name);
                res.insert(target_hash, tag_name);
            }
        }
    }

    Ok(res)
}
