use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;

use tempfile::TempDir;

const TRAIT: &str =
    "---\ntype: Trait\ndescription: check the build\nwrites: false\n---\nRun the build.\n";

/// Serve one response on a free local port and return its URL. The thread
/// ends after the one request.
fn serve(status: &str, body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let status = status.to_string();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request);
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
    });
    format!("http://127.0.0.1:{port}/index.md")
}

/// Run `trait fetch <url>` with a temp folder of its own for `TMPDIR`.
fn fetch(tmp: &Path, url: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["trait", "fetch", url])
        .env("TMPDIR", tmp)
        .current_dir(tmp)
        .output()
        .unwrap()
}

fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().canonicalize().unwrap();
    (tmp, path)
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

fn left_in(tmp: &Path) -> Vec<String> {
    fs::read_dir(tmp)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn trait_fetch_downloads_checks_and_prints_the_description_writes_and_path() {
    let (_tmp, root) = root();
    let url = serve("200 OK", TRAIT.as_bytes().to_vec());

    let out = fetch(&root, &url);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    let fields: Vec<&str> = text.trim_end().split('\t').collect();
    assert_eq!(fields[..2], ["check the build", "false"], "{text}");
    let file = Path::new(fields[2]);
    assert!(file.starts_with(&root), "{file:?}");
    assert!(
        file.parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("agent-graph-trait."),
        "{file:?}"
    );
    assert_eq!(file.file_name().unwrap(), "index.md");
    assert_eq!(fs::read_to_string(file).unwrap(), TRAIT);
}

#[test]
fn trait_fetch_output_is_what_trait_check_prints_for_the_same_file() {
    let (_tmp, root) = root();
    let url = serve("200 OK", TRAIT.as_bytes().to_vec());

    let out = fetch(&root, &url);
    let path = stdout(&out)
        .trim_end()
        .split('\t')
        .nth(2)
        .unwrap()
        .to_string();
    let check = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["trait", "check", &path])
        .output()
        .unwrap();

    assert_eq!(stdout(&out), stdout(&check));
}

#[test]
fn trait_fetch_of_a_missing_page_fails_and_leaves_no_folder() {
    let (_tmp, root) = root();
    let url = serve("404 Not Found", b"nope".to_vec());

    let out = fetch(&root, &url);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("404"), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(left_in(&root), Vec::<String>::new());
}

#[test]
fn trait_fetch_of_a_file_that_is_not_a_trait_fails_and_leaves_no_folder() {
    let (_tmp, root) = root();
    let url = serve("200 OK", b"just text\n".to_vec());

    let out = fetch(&root, &url);

    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("is not a valid trait"),
        "{}",
        stderr(&out)
    );
    assert_eq!(left_in(&root), Vec::<String>::new());
}

#[test]
fn trait_fetch_refuses_a_page_over_the_size_limit() {
    let (_tmp, root) = root();
    let mut big = TRAIT.as_bytes().to_vec();
    big.extend(std::iter::repeat_n(b'x', 2 * 1024 * 1024));
    let url = serve("200 OK", big);

    let out = fetch(&root, &url);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("too large"), "{}", stderr(&out));
    assert_eq!(left_in(&root), Vec::<String>::new());
}

#[test]
fn trait_fetch_takes_an_https_url_and_a_loopback_http_url_only() {
    let (_tmp, root) = root();

    for url in [
        "http://example.com/index.md",
        "ftp://example.com/index.md",
        "file:///etc/passwd",
        "example.com/index.md",
        "http://127.0.0.1.example.com/index.md",
        "http://localhost.example.com/index.md",
    ] {
        let out = fetch(&root, url);

        assert_eq!(out.status.code(), Some(1), "{url}: {}", stderr(&out));
        assert!(stderr(&out).contains("https://"), "{url}: {}", stderr(&out));
    }
    assert_eq!(left_in(&root), Vec::<String>::new());
}

/// Serve one response, and tell whether a client ever came.
fn serve_counting(
    status: &str,
    headers: &str,
    body: &[u8],
) -> (String, std::sync::mpsc::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sender, receiver) = std::sync::mpsc::channel();
    let (status, headers, body) = (status.to_string(), headers.to_string(), body.to_vec());
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = sender.send(());
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request);
        let head = format!(
            "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
    });
    (format!("http://127.0.0.1:{port}/index.md"), receiver)
}

#[test]
fn trait_fetch_follows_no_redirect_for_a_loopback_http_url() {
    let (_tmp, root) = root();
    let (target, target_hit) = serve_counting("200 OK", "", TRAIT.as_bytes());
    let (url, _) = serve_counting("302 Found", &format!("Location: {target}\r\n"), b"");

    let out = fetch(&root, &url);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("redirect"), "{}", stderr(&out));
    assert!(
        target_hit
            .recv_timeout(std::time::Duration::from_millis(300))
            .is_err(),
        "the client followed the redirect"
    );
    assert_eq!(left_in(&root), Vec::<String>::new());
}

#[cfg(unix)]
#[test]
fn trait_fetch_makes_a_private_folder_and_file() {
    use std::os::unix::fs::PermissionsExt;

    let (_tmp, root) = root();
    let url = serve("200 OK", TRAIT.as_bytes().to_vec());

    let out = fetch(&root, &url);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    let file = Path::new(text.trim_end().split('\t').nth(2).unwrap());
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(file.parent().unwrap()), 0o700);
    assert_eq!(mode(file), 0o600);
}
