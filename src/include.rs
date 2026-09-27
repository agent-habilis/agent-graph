use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::frontmatter;

/// The body of `file` with each `@path` line replaced by the body of that
/// file, recursively. Every included file must be inside `root`.
pub(crate) fn expand(root: &Path, file: &Path) -> Result<String> {
    let root = root.canonicalize()?;
    let mut out = String::new();
    expand_into(&root, &file.canonicalize()?, &mut Vec::new(), &mut out)?;
    Ok(out)
}

fn expand_into(root: &Path, file: &Path, stack: &mut Vec<PathBuf>, out: &mut String) -> Result<()> {
    if stack.iter().any(|seen| seen == file) {
        let chain: Vec<_> = stack
            .iter()
            .chain([&file.to_path_buf()])
            .map(|path| relative(root, path))
            .collect();
        bail!("include cycle: {}", chain.join(" -> "));
    }
    stack.push(file.to_path_buf());
    let content =
        fs::read_to_string(file).with_context(|| format!("cannot read {}", file.display()))?;
    let mut in_fence = false;
    for line in frontmatter::body(&content).lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        }
        match include_target(trimmed) {
            Some(target) if !in_fence => {
                let included = resolve(root, file, target)?;
                expand_into(root, &included, stack, out)?;
            }
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    stack.pop();
    Ok(())
}

fn include_target(line: &str) -> Option<&str> {
    line.strip_prefix('@')
        .filter(|target| !target.is_empty() && !target.contains(char::is_whitespace))
}

fn resolve(root: &Path, from: &Path, target: &str) -> Result<PathBuf> {
    let joined = from.parent().unwrap_or(root).join(target);
    let path = joined
        .canonicalize()
        .with_context(|| format!("include `@{target}` in {} not found", relative(root, from)))?;
    if !path.starts_with(root) {
        bail!(
            "include `@{target}` in {} is outside the template folder",
            relative(root, from)
        );
    }
    Ok(path)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
