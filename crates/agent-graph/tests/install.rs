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

/// A tar file of `folder`, made by the system `tar`, as an older peer made it.
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

/// One entry of a hand-made tar file.
struct Entry<'a> {
    path: &'a str,
    kind: tar::EntryType,
    mode: u32,
    data: &'a [u8],
}

fn file_entry<'a>(path: &'a str, data: &'a [u8]) -> Entry<'a> {
    Entry {
        path,
        kind: tar::EntryType::Regular,
        mode: 0o644,
        data,
    }
}

fn build_tar(file: &Path, entries: &[Entry<'_>]) {
    let mut builder = tar::Builder::new(fs::File::create(file).unwrap());
    for entry in entries {
        let mut header = tar::Header::new_gnu();
        let name = entry.path.as_bytes();
        header.as_gnu_mut().unwrap().name[..name.len()].copy_from_slice(name);
        header.set_entry_type(entry.kind);
        header.set_mode(entry.mode);
        header.set_size(entry.data.len() as u64);
        header.set_cksum();
        builder.append(&header, entry.data).unwrap();
    }
    builder.finish().unwrap();
}

const INDEX: &[u8] = b"---\ntype: Graph\ndescription: g\nscope: branch\n---\n";

fn install_tar(root: &Path, file: &Path, expect: &str) -> Output {
    run(
        root,
        &[
            "graph",
            "install",
            "--tar",
            file.to_str().unwrap(),
            "--expect",
            expect,
        ],
    )
}

/// The hash of a graph folder that holds `files`, made in a scratch folder.
fn hash_of(root: &Path, files: &[(&str, &[u8])]) -> String {
    let dir = root.join("hash-scratch");
    let _ = fs::remove_dir_all(&dir);
    for (path, data) in files {
        fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
        fs::write(dir.join(path), data).unwrap();
    }
    let hash = graph::hash(&dir).unwrap();
    fs::remove_dir_all(&dir).unwrap();
    hash
}

#[test]
fn install_replaces_an_installed_folder_whose_content_does_not_match_its_name() {
    let (_tmp, root) = root();
    let src = root.join("src");
    let hash = graph_folder(&src);
    assert!(
        run(&root, &["graph", "install", src.to_str().unwrap()])
            .status
            .success()
    );
    let copy = installed(&root).join(&hash);
    fs::write(copy.join("index.md"), "tampered\n").unwrap();
    fs::write(copy.join("extra.md"), "extra\n").unwrap();

    let out = run(&root, &["graph", "install", src.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(graph::hash(&copy).unwrap(), hash);
    assert!(!copy.join("extra.md").exists());
    assert_eq!(entries(&root), [hash]);
}

#[test]
fn install_replaces_a_folder_that_does_not_match_its_name_when_it_installs_a_tar_too() {
    let (_tmp, root) = root();
    let files: [(&str, &[u8]); 1] = [("index.md", INDEX)];
    let hash = hash_of(&root, &files);
    let copy = installed(&root).join(&hash);
    fs::create_dir_all(&copy).unwrap();
    fs::write(copy.join("index.md"), "tampered\n").unwrap();
    let file = root.join("g.tar");
    build_tar(&file, &[file_entry("index.md", INDEX)]);

    let out = install_tar(&root, &file, &hash);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(graph::hash(&copy).unwrap(), hash);
}

#[test]
fn install_stages_outside_the_graphs_folder_and_removes_a_stale_stage() {
    let (_tmp, root) = root();
    let src = root.join("src");
    graph_folder(&src);
    let staging = root.join(".agent-graph/.staging");
    let stale = staging.join("old.tmp-1");
    let fresh = staging.join("fresh.tmp-2");
    fs::create_dir_all(&stale).unwrap();
    fs::create_dir_all(&fresh).unwrap();
    let two_hours_ago = std::time::SystemTime::now() - std::time::Duration::from_hours(2);
    fs::File::open(&stale)
        .unwrap()
        .set_modified(two_hours_ago)
        .unwrap();

    let out = run(&root, &["graph", "install", src.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stale.exists(), "a stale stage stays");
    assert!(fresh.exists(), "a recent stage of another process goes");
    assert_eq!(entries(&root).len(), 1, "{:?}", entries(&root));
}

#[test]
fn a_failed_tar_install_leaves_nothing_in_the_staging_folder() {
    let (_tmp, root) = root();
    let file = root.join("g.tar");
    build_tar(&file, &[file_entry("index.md", INDEX)]);

    let out = install_tar(&root, &file, &"e".repeat(64));

    assert_eq!(out.status.code(), Some(1));
    let staging = root.join(".agent-graph/.staging");
    let left: Vec<_> = fs::read_dir(&staging)
        .map(Iterator::count)
        .into_iter()
        .collect();
    assert!(left.iter().all(|count| *count == 0), "{left:?}");
}

#[test]
fn install_refuses_a_tar_with_too_many_entries() {
    let (_tmp, root) = root();
    let file = root.join("many.tar");
    let names: Vec<String> = (0..10_001).map(|number| format!("f{number}.md")).collect();
    let entries_: Vec<Entry<'_>> = names.iter().map(|name| file_entry(name, b"")).collect();
    build_tar(&file, &entries_);

    let out = install_tar(&root, &file, &"f".repeat(64));

    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("too many entries"),
        "{}",
        stderr(&out)
    );
    assert_eq!(entries(&root), Vec::<String>::new());
}

#[test]
fn install_refuses_a_tar_over_the_size_limit_before_it_writes_the_big_file() {
    let (_tmp, root) = root();
    let file = root.join("big.tar");
    let big = vec![0_u8; 16 * 1024 * 1024 + 1];
    build_tar(
        &file,
        &[file_entry("index.md", INDEX), file_entry("big.bin", &big)],
    );

    let out = install_tar(&root, &file, &"1".repeat(64));

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("too large"), "{}", stderr(&out));
    assert_eq!(entries(&root), Vec::<String>::new());
}

#[test]
fn install_refuses_an_absolute_path_in_a_tar() {
    let (_tmp, root) = root();
    let file = root.join("abs.tar");
    build_tar(&file, &[file_entry("/index.md", INDEX)]);
    let files: [(&str, &[u8]); 1] = [("index.md", INDEX)];
    let hash = hash_of(&root, &files);

    let out = install_tar(&root, &file, &hash);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("outside"), "{}", stderr(&out));
    assert_eq!(entries(&root), Vec::<String>::new());
}

#[cfg(unix)]
#[test]
fn install_sets_a_mode_of_0644_for_a_file_and_0755_for_a_folder() {
    use std::os::unix::fs::PermissionsExt;

    let (_tmp, root) = root();
    let files: [(&str, &[u8]); 2] = [("index.md", INDEX), ("nodes/w/index.md", b"n\n")];
    let hash = hash_of(&root, &files);
    let file = root.join("g.tar");
    build_tar(
        &file,
        &[
            Entry {
                path: "nodes",
                kind: tar::EntryType::Directory,
                mode: 0o700,
                data: b"",
            },
            Entry {
                path: "nodes/w",
                kind: tar::EntryType::Directory,
                mode: 0o700,
                data: b"",
            },
            Entry {
                mode: 0o777,
                ..file_entry("index.md", INDEX)
            },
            Entry {
                mode: 0o600,
                ..file_entry("nodes/w/index.md", b"n\n")
            },
        ],
    );

    let out = install_tar(&root, &file, &hash);

    assert!(out.status.success(), "{}", stderr(&out));
    let copy = installed(&root).join(&hash);
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&copy.join("index.md")), 0o644);
    assert_eq!(mode(&copy.join("nodes/w/index.md")), 0o644);
    assert_eq!(mode(&copy.join("nodes")), 0o755);
    assert_eq!(mode(&copy.join("nodes/w")), 0o755);
}

#[cfg(unix)]
#[test]
fn install_of_a_folder_sets_the_same_modes() {
    use std::os::unix::fs::PermissionsExt;

    let (_tmp, root) = root();
    let src = root.join("src");
    let hash = graph_folder(&src);
    fs::set_permissions(src.join("index.md"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(src.join("nodes"), fs::Permissions::from_mode(0o700)).unwrap();

    let out = run(&root, &["graph", "install", src.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    let copy = installed(&root).join(hash);
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&copy.join("index.md")), 0o644);
    assert_eq!(mode(&copy.join("nodes")), 0o755);
}

#[test]
fn install_refuses_an_empty_home() {
    let (_tmp, root) = root();
    let src = root.join("src");
    graph_folder(&src);

    let out = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["graph", "install", src.to_str().unwrap()])
        .env("HOME", "")
        .current_dir(&root)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("HOME"), "{}", stderr(&out));
    assert!(!root.join(".agent-graph").exists());
}

/// Run with a working folder and a home folder of their own.
fn run_in(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    fs::create_dir_all(cwd).unwrap();
    Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(args)
        .env("HOME", home)
        .current_dir(cwd)
        .output()
        .unwrap()
}

#[test]
fn install_skips_apple_double_entries_so_a_tar_of_an_older_peer_installs() {
    let (_tmp, root) = root();
    let files: [(&str, &[u8]); 2] = [("index.md", INDEX), ("nodes/w/index.md", b"n\n")];
    let hash = hash_of(&root, &files);
    let file = root.join("old.tar");
    build_tar(
        &file,
        &[
            file_entry("index.md", INDEX),
            file_entry("._index.md", b"\0\x05\x16\x07 apple double"),
            file_entry("nodes/w/index.md", b"n\n"),
            file_entry("nodes/w/._index.md", b"\0\x05\x16\x07"),
        ],
    );

    let out = install_tar(&root, &file, &hash);

    assert!(out.status.success(), "{}", stderr(&out));
    let copy = installed(&root).join(&hash);
    assert!(!copy.join("._index.md").exists());
    assert!(!copy.join("nodes/w/._index.md").exists());
}

/// `tests/fixtures/apple-double.tar` is the tar that `/usr/bin/tar` on macOS
/// made, with no `COPYFILE_DISABLE`, for a graph whose files have an xattr. It
/// holds a `._` entry for each file and folder. It was made once, so that the
/// check runs on each OS and not only on macOS. The graph is `index.md` and
/// `nodes/worker/index.md`, and its hash is below.
const APPLE_DOUBLE_TAR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/apple-double.tar"
);
const APPLE_DOUBLE_HASH: &str = "0cd6499109c5a696d160babb7b86a186ad69a6ab60d44f0f983c2f2cb4f4eca1";

#[test]
fn install_accepts_the_committed_tar_that_macos_made_with_apple_double_entries() {
    let names: Vec<String> = tar::Archive::new(fs::File::open(APPLE_DOUBLE_TAR).unwrap())
        .entries()
        .unwrap()
        .map(|entry| {
            entry
                .unwrap()
                .path()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(
        names.iter().any(|name| name.contains("._")),
        "the fixture has no AppleDouble entry: {names:?}"
    );
    let (_tmp, root) = root();

    let out = install_tar(&root, Path::new(APPLE_DOUBLE_TAR), APPLE_DOUBLE_HASH);

    assert!(out.status.success(), "{}", stderr(&out));
    let copy = installed(&root).join(APPLE_DOUBLE_HASH);
    assert!(copy.join("index.md").is_file());
    assert!(copy.join("nodes/worker/index.md").is_file());
    assert_eq!(graph::hash(&copy).unwrap(), APPLE_DOUBLE_HASH);
    assert!(
        graph::files(&copy).unwrap().len() == 2,
        "the copy holds more than the graph"
    );
    for entry in fs::read_dir(&copy).unwrap() {
        assert!(
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("._")
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn install_accepts_the_tar_that_the_system_tar_makes_for_a_file_with_an_xattr() {
    let (_tmp, root) = root();
    let src = root.join("src");
    let hash = graph_folder(&src);
    xattr::set(src.join("index.md"), "com.example.test", b"x").unwrap();
    let file = root.join("graph.tar");
    // No COPYFILE_DISABLE: this is the tar of a peer that does not set it.
    let status = Command::new("/usr/bin/tar")
        .arg("-cf")
        .arg(&file)
        .arg("-C")
        .arg(&src)
        .arg(".")
        .status()
        .unwrap();
    assert!(status.success());
    // `tar -tf` hides the AppleDouble entries, so read the file the way the
    // installer does.
    let names: Vec<String> = tar::Archive::new(fs::File::open(&file).unwrap())
        .entries()
        .unwrap()
        .map(|entry| entry.unwrap().path().unwrap().display().to_string())
        .collect();
    assert!(
        names.iter().any(|name| name.contains("._")),
        "the system tar made no AppleDouble entry: {names:?}"
    );

    let out = install_tar(&root, &file, &hash);

    assert!(out.status.success(), "{}", stderr(&out));
    let copy = installed(&root).join(&hash);
    assert!(!copy.join("._index.md").exists());
}

/// A graph folder `<root>/.agent-graph/graphs/g` with a nested node and an
/// extended attribute on one file. Returns the hash.
fn graph_with_an_xattr(root: &Path) -> String {
    let dir = root.join(".agent-graph/graphs/g");
    let hash = graph_folder(&dir);
    #[cfg(unix)]
    xattr::set(dir.join("index.md"), "user.test", b"x").ok();
    hash
}

#[test]
fn graph_pack_writes_a_tar_that_install_accepts_with_the_hash_of_the_graph() {
    let (_tmp, root) = root();
    let hash = graph_with_an_xattr(&root);
    let file = root.join("g.tar");
    let home = root.join("home");

    let packed = run_in(
        &root,
        &home,
        &["graph", "pack", "g", "--output", file.to_str().unwrap()],
    );
    let out = run_in(
        &root,
        &home,
        &[
            "graph",
            "install",
            "--tar",
            file.to_str().unwrap(),
            "--expect",
            &hash,
        ],
    );

    assert!(packed.status.success(), "{}", stderr(&packed));
    assert_eq!(stdout(&packed), "");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        graph::hash(&home.join(".agent-graph/graphs").join(&hash)).unwrap(),
        hash
    );
}

#[test]
fn a_graph_with_a_real_apple_double_name_has_the_hash_of_the_graph_without_it_and_installs() {
    let (_tmp, root) = root();
    let dir = root.join(".agent-graph/graphs/g");
    let plain = graph_folder(&dir);
    write(&dir.join("._index.md"), "a file that macOS wrote on exFAT");
    write(&dir.join("nodes/worker/._index.md"), "another");
    write(
        &dir.join("nodes/._hidden/index.md"),
        "a folder with that name",
    );
    let hash = graph::hash(&dir).unwrap();
    let file = root.join("g.tar");
    let home = root.join("home");

    let packed = run_in(
        &root,
        &home,
        &["graph", "pack", "g", "--output", file.to_str().unwrap()],
    );
    let out = run_in(
        &root,
        &home,
        &[
            "graph",
            "install",
            "--tar",
            file.to_str().unwrap(),
            "--expect",
            &hash,
        ],
    );

    assert_eq!(hash, plain, "the hash covers a ._ name");
    assert!(packed.status.success(), "{}", stderr(&packed));
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(graph::hash(&installed(&home).join(&hash)).unwrap(), hash);
}

#[test]
fn graph_pack_holds_only_regular_files_in_path_order_with_fixed_headers() {
    let (_tmp, root) = root();
    graph_with_an_xattr(&root);
    let (first, second) = (root.join("a.tar"), root.join("b.tar"));
    let home = root.join("home");

    for file in [&first, &second] {
        let out = run_in(
            &root,
            &home,
            &["graph", "pack", "g", "--output", file.to_str().unwrap()],
        );
        assert!(out.status.success(), "{}", stderr(&out));
    }

    assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
    let mut archive = tar::Archive::new(fs::File::open(&first).unwrap());
    let mut seen = Vec::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let header = entry.header();
        assert_eq!(header.entry_type(), tar::EntryType::Regular);
        assert_eq!(header.mode().unwrap(), 0o644);
        assert_eq!(
            (
                header.uid().unwrap(),
                header.gid().unwrap(),
                header.mtime().unwrap()
            ),
            (0, 0, 0)
        );
        seen.push(entry.path().unwrap().display().to_string());
    }
    assert_eq!(seen, ["index.md", "nodes/worker/index.md"]);
}

#[test]
fn graph_pack_of_an_unknown_graph_fails_and_writes_no_file() {
    let (_tmp, root) = root();
    let file = root.join("g.tar");

    let out = run_in(
        &root,
        &root,
        &["graph", "pack", "nope", "--output", file.to_str().unwrap()],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("not found"), "{}", stderr(&out));
    assert!(!file.exists());
}

#[test]
fn graph_pack_refuses_a_graph_over_the_size_limit() {
    let (_tmp, root) = root();
    graph_with_an_xattr(&root);
    fs::write(
        root.join(".agent-graph/graphs/g/big.bin"),
        vec![0_u8; 16 * 1024 * 1024 + 1],
    )
    .unwrap();
    let file = root.join("g.tar");

    let out = run_in(
        &root,
        &root,
        &["graph", "pack", "g", "--output", file.to_str().unwrap()],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("too large"), "{}", stderr(&out));
    assert!(!file.exists());
}

#[cfg(unix)]
#[test]
fn graph_pack_does_not_write_through_a_link() {
    let (_tmp, root) = root();
    graph_with_an_xattr(&root);
    let target = root.join("precious.txt");
    fs::write(&target, "keep\n").unwrap();
    let link = root.join("g.tar");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let out = run_in(
        &root,
        &root,
        &["graph", "pack", "g", "--output", link.to_str().unwrap()],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("symbolic link"), "{}", stderr(&out));
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep\n");
}
