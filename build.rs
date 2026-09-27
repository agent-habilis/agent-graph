//! Embed every file under `defaults/` into the binary for `agent-graph template init`,
//! and render the `skills/` sources into one self-contained `SKILL.md` per skill
//! for `agent-graph plug`.

use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as _;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{env, io};

fn main() -> io::Result<()> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    embed_defaults(&manifest.join("defaults"), &out_dir)?;
    let rendered = out_dir.join("skills");
    render_skills(&manifest.join("skills"), &rendered)?;
    emit_skills_fingerprint(&rendered);
    Ok(())
}

fn embed_defaults(root: &Path, out_dir: &Path) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(root, &mut files)?;
    files.sort();
    let mut out = String::from("pub(crate) static DEFAULTS: &[(&str, &str)] = &[\n");
    for file in &files {
        let relative = file.strip_prefix(root).expect("walked under root");
        writeln!(
            out,
            "    ({:?}, include_str!({:?})),",
            relative.to_str().expect("UTF-8 path"),
            file.to_str().expect("UTF-8 path"),
        )
        .expect("write to String");
    }
    out.push_str("];\n");
    fs::write(out_dir.join("defaults.rs"), out)
}

/// Expand each `skills/{template,role,team}-*/SKILL.md` source into `dest`.
/// The partials in `skills/shared/` are only spliced in, never emitted.
fn render_skills(src: &Path, dest: &Path) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", src.display());
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    let mut skills: Vec<PathBuf> = fs::read_dir(src)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| {
            path.is_dir()
                && path.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    ["template-", "role-", "team-"]
                        .iter()
                        .any(|prefix| name.starts_with(prefix))
                })
        })
        .collect();
    skills.sort();
    let mut loader = |path: &Path| fs::read_to_string(path).map_err(|error| error.to_string());
    for skill_dir in skills {
        let skill = skill_dir.file_name().expect("skill dir has a name");
        let rendered = slot_template::expand(&skill_dir.join("SKILL.md"), &[], &mut loader)
            .unwrap_or_else(|error| panic!("skills/{}/SKILL.md: {error}", skill.to_string_lossy()));
        let out = dest.join(skill);
        fs::create_dir_all(&out)?;
        fs::write(out.join("SKILL.md"), rendered)?;
    }
    Ok(())
}

/// `include_dir!` does not track its folder, so `src/plug.rs` reads this hash
/// with `env!`: a changed skill recompiles that module and its embed.
fn emit_skills_fingerprint(rendered: &Path) {
    let mut files = Vec::new();
    walk(rendered, &mut files).expect("walk rendered skills");
    files.sort();
    let mut hasher = DefaultHasher::new();
    for file in files {
        file.strip_prefix(rendered)
            .expect("walked under root")
            .hash(&mut hasher);
        fs::read(&file)
            .expect("read rendered skill")
            .hash(&mut hasher);
    }
    println!(
        "cargo:rustc-env=AGENT_GRAPH_SKILLS_FINGERPRINT={:016x}",
        hasher.finish()
    );
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}
