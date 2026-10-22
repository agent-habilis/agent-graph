use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::node::{TRAITS, subfolders};
use crate::{frontmatter, markdown};

#[derive(Debug)]
pub struct Trait {
    pub description: String,
    /// Whether the trait can change files that git tracks and commit.
    pub writes: bool,
}

/// Trait folders in `.agent-graph/traits/` from `start` up to `/`, by name.
/// A nearer folder shadows a farther one with the same name, valid or not.
#[must_use]
pub fn discover(start: &Path) -> BTreeMap<String, PathBuf> {
    let mut traits = BTreeMap::new();
    for level in start.ancestors() {
        let Ok(entries) = fs::read_dir(level.join(".agent-graph").join(TRAITS)) else {
            continue;
        };
        for (name, path) in subfolders(entries) {
            traits.entry(name).or_insert(path);
        }
    }
    traits
}

/// Read and validate a trait file: the `index.md` of a trait folder, or a
/// downloaded copy of one. The error is a reason for the user.
///
/// # Errors
///
/// Fails when `file` cannot be read or is not a valid trait.
pub fn load(file: &Path) -> Result<Trait, String> {
    let content = fs::read_to_string(file).map_err(|err| err.to_string())?;
    let frontmatter = frontmatter::parse(&content).ok_or("no frontmatter with a `type:` key")?;
    match frontmatter.get("type") {
        Some("Trait") => {}
        other => {
            return Err(format!(
                "type is `{}`, must be `Trait`",
                other.unwrap_or("")
            ));
        }
    }
    let description = frontmatter
        .get("description")
        .filter(|description| !description.is_empty())
        .ok_or("no description")?
        .to_string();
    let writes = match frontmatter.get("writes") {
        None | Some("false") => false,
        Some("true") => true,
        Some(value) => return Err(format!("writes is `{value}`, must be `true` or `false`")),
    };
    markdown::check_boundaries(&content)
        .map_err(|(line, reason)| format!("line {line}: {reason}"))?;
    Ok(Trait {
        description,
        writes,
    })
}
