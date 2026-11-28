use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use agent_graph_core::graph;
use tempfile::TempDir;

fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().canonicalize().unwrap();
    (tmp, path)
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A graph folder with a nested node, and the hash of it.
fn graph_folder(dir: &Path) -> String {
    write(
        &dir.join("index.md"),
        "---\ntype: Graph\ndescription: g\nscope: branch\n---\n",
    );
    write(
        &dir.join("nodes/worker/index.md"),
        "---\ntype: Node\ndescription: w\n---\nbody\n",
    );
    graph::hash(dir).unwrap()
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(args)
        .env("HOME", home)
        .current_dir(home)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

fn installed(home: &Path) -> PathBuf {
    home.join(".agent-graph/graphs")
}

/// The entries of the graphs folder, or none when it does not exist.
fn entries(home: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(installed(home)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A tar file of `folder`, made by the system `tar`, as the skills make it.
fn system_tar(folder: &Path, file: &Path) {
    let status = Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .arg("-cf")
        .arg(file)
        .arg("-C")
        .arg(folder)
        .arg(".")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn install_copies_a_graph_folder_under_its_hash_and_prints_the_hash() {
    let (_tmp, root) = root();
    let src = root.join("src");
    let hash = graph_folder(&src);

    let out = run(&root, &["graph", "install", src.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), format!("{hash}\n"));
    let copy = installed(&root).join(&hash);
    assert_eq!(graph::hash(&copy).unwrap(), hash);
    assert_eq!(entries(&root), [hash]);
}

#[test]
fn install_twice_succeeds_and_leaves_one_folder() {
    let (_tmp, root) = root();
    let src = root.join("src");
    let hash = graph_folder(&src);

    for _ in 0..2 {
        let out = run(&root, &["graph", "install", src.to_str().unwrap()]);
        assert!(out.status.success(), "{}", stderr(&out));
        assert_eq!(stdout(&out), format!("{hash}\n"));
    }

    assert_eq!(entries(&root), [hash]);
}

#[test]
fn install_tar_extracts_a_graph_that_matches_the_expected_hash() {
    let (_tmp, root) = root();
    let src = root.join("src");
    let hash = graph_folder(&src);
    let file = root.join("graph.tar");
    system_tar(&src, &file);

    let out = run(
        &root,
        &[
            "graph",
            "install",
            "--tar",
            file.to_str().unwrap(),
            "--expect",
            &hash,
        ],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), format!("{hash}\n"));
    assert_eq!(graph::hash(&installed(&root).join(&hash)).unwrap(), hash);
    assert_eq!(entries(&root), [hash]);
}

#[test]
fn install_tar_with_another_hash_removes_what_it_extracted_and_exits_1() {
    let (_tmp, root) = root();
    let src = root.join("src");
    let hash = graph_folder(&src);
    let file = root.join("graph.tar");
    system_tar(&src, &file);
    let wrong = "0".repeat(64);

    let out = run(
        &root,
        &[
            "graph",
            "install",
            "--tar",
            file.to_str().unwrap(),
            "--expect",
            &wrong,
        ],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("mismatch"), "{}", stderr(&out));
    assert!(stderr(&out).contains(&wrong), "{}", stderr(&out));
    assert!(stderr(&out).contains(&hash), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(entries(&root), Vec::<String>::new());
}

/// A tar file with one entry, written by hand so that the entry can be one
/// that the system `tar` does not make.
fn tar_with_entry(file: &Path, path: &[u8], kind: tar::EntryType, link: Option<&str>) {
    let mut header = tar::Header::new_gnu();
    header.as_gnu_mut().unwrap().name[..path.len()].copy_from_slice(path);
    header.set_entry_type(kind);
    header.set_mode(0o644);
    if let Some(target) = link {
        header.set_link_name(target).unwrap();
    }
    header.set_size(0);
    header.set_cksum();
    let mut builder = tar::Builder::new(fs::File::create(file).unwrap());
    builder.append(&header, std::io::empty()).unwrap();
    builder.finish().unwrap();
}

#[test]
fn install_tar_refuses_a_link_and_leaves_nothing() {
    let (_tmp, root) = root();
    let file = root.join("bad.tar");
    tar_with_entry(
        &file,
        b"index.md",
        tar::EntryType::Symlink,
        Some("/etc/passwd"),
    );

    let out = run(
        &root,
        &[
            "graph",
            "install",
            "--tar",
            file.to_str().unwrap(),
            "--expect",
            &"a".repeat(64),
        ],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("link"), "{}", stderr(&out));
    assert_eq!(entries(&root), Vec::<String>::new());
}

#[test]
fn install_tar_refuses_a_path_that_leaves_the_folder_and_leaves_nothing() {
    let (_tmp, root) = root();
    let file = root.join("bad.tar");
    tar_with_entry(&file, b"../escape.md", tar::EntryType::Regular, None);

    let out = run(
        &root,
        &[
            "graph",
            "install",
            "--tar",
            file.to_str().unwrap(),
            "--expect",
            &"b".repeat(64),
        ],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("outside"), "{}", stderr(&out));
    assert!(!installed(&root).join("escape.md").exists());
    assert!(!root.join(".agent-graph/escape.md").exists());
    assert_eq!(entries(&root), Vec::<String>::new());
}

#[test]
fn install_takes_a_folder_or_a_tar_with_its_hash_and_nothing_else() {
    let (_tmp, root) = root();
    let src = root.join("src");
    graph_folder(&src);
    let hash = "c".repeat(64);
    let src_text = src.to_str().unwrap();

    for args in [
        vec!["graph", "install"],
        vec!["graph", "install", "--tar", "x.tar"],
        vec!["graph", "install", "--expect", hash.as_str()],
        vec![
            "graph",
            "install",
            src_text,
            "--tar",
            "x.tar",
            "--expect",
            hash.as_str(),
        ],
        vec![
            "graph",
            "install",
            "--tar",
            "x.tar",
            "--expect",
            "../../etc",
        ],
        vec!["graph", "install", "--tar", "x.tar", "--expect", "ABCDEF"],
    ] {
        let out = run(&root, &args);

        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
    }
    assert_eq!(entries(&root), Vec::<String>::new());
}

#[test]
fn install_of_a_missing_folder_or_tar_fails_and_leaves_nothing() {
    let (_tmp, root) = root();

    let folder = run(&root, &["graph", "install", "nope"]);
    let file = run(
        &root,
        &[
            "graph",
            "install",
            "--tar",
            "nope.tar",
            "--expect",
            &"d".repeat(64),
        ],
    );

    assert_eq!(folder.status.code(), Some(1), "{}", stderr(&folder));
    assert_eq!(file.status.code(), Some(1), "{}", stderr(&file));
    assert_eq!(entries(&root), Vec::<String>::new());
}
