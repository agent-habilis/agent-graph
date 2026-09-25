//! Embed every file under `defaults/` into the binary for `agent-graph graph init`.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::{env, io};

fn main() -> io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("defaults");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(&root, &mut files)?;
    files.sort();
    let mut out = String::from("pub(crate) static DEFAULTS: &[(&str, &str)] = &[\n");
    for file in &files {
        let relative = file.strip_prefix(&root).expect("walked under root");
        writeln!(
            out,
            "    ({:?}, include_str!({:?})),",
            relative.to_str().expect("UTF-8 path"),
            file.to_str().expect("UTF-8 path"),
        )
        .expect("write to String");
    }
    out.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("defaults.rs"),
        out,
    )
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
