//! `trait fetch`: download a trait file into a new folder and check it.

use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io::Read as _};

use agent_graph_core::traits::{self, Trait};
use anyhow::{Context, Result, anyhow, bail};

/// A trait is a page of text. A page over this size is not one.
const MAX_BYTES: u64 = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

/// Download `url` into a new folder under the temp folder, and check the
/// file. A page that does not download, is too large, or is not a trait
/// leaves no folder. Returns the trait and the path of the file.
pub(crate) fn fetch(url: &str) -> Result<(Trait, PathBuf)> {
    let loopback = is_loopback_http(url);
    if !loopback && !url.starts_with("https://") {
        bail!("`{url}` must start with https://");
    }
    let body = download(url, loopback)?;
    let dir = private_dir()?;
    let file = dir.path().join("index.md");
    write_private(&file, &body).with_context(|| format!("cannot write {}", file.display()))?;
    let item =
        traits::load(&file).map_err(|reason| anyhow!("{url} is not a valid trait: {reason}"))?;
    // The fork reads the file after this command ends, so the folder stays.
    let kept = dir.keep();
    Ok((item, kept.join("index.md")))
}

/// A new folder that only this user can enter: the file in it is text from
/// outside the project, and the fork obeys it.
fn private_dir() -> Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("agent-graph-trait.");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o700));
    }
    builder.tempdir().context("cannot create a temp folder")
}

fn write_private(file: &std::path::Path, body: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(file)?.write_all(body)
}

/// The client. It trusts the certificate store of the operating system, as
/// `curl` and the browsers do, so a private CA that a company or a proxy
/// installs there works. The bundled list of Mozilla roots would not.
fn agent(loopback: bool) -> ureq::Agent {
    // A redirect from a local server could lead to any `http://` host, so a
    // loopback URL follows none. An `https://` URL follows only `https://`.
    ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .https_only(!loopback)
        .max_redirects(if loopback { 0 } else { 10 })
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into()
}

fn download(url: &str, loopback: bool) -> Result<Vec<u8>> {
    let agent = agent(loopback);
    let mut response = agent.get(url).call().map_err(|error| {
        if let ureq::Error::StatusCode(code) = error {
            anyhow!("{url}: the server answered HTTP {code}")
        } else {
            anyhow!("{url}: {error}")
        }
    })?;
    if response.status().is_redirection() {
        bail!("{url}: the server answered with a redirect, and a local URL follows none");
    }
    let mut body = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut body)
        .with_context(|| format!("{url}: cannot read the page"))?;
    if body.len() as u64 > MAX_BYTES {
        bail!("{url}: the page is too large (over {MAX_BYTES} bytes)");
    }
    Ok(body)
}

/// `http://` is for a server on this machine only, which is how the tests
/// serve a trait. The host must be exactly a loopback name.
fn is_loopback_http(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority
        .rsplit_once(':')
        .filter(|(_, port)| port.bytes().all(|byte| byte.is_ascii_digit()))
        .map_or(authority, |(host, _)| host);
    matches!(host, "127.0.0.1" | "localhost" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::{agent, is_loopback_http};

    #[test]
    fn the_client_trusts_the_certificate_store_of_the_operating_system() {
        for loopback in [false, true] {
            let client = agent(loopback);
            assert!(
                matches!(
                    client.config().tls_config().root_certs(),
                    ureq::tls::RootCerts::PlatformVerifier
                ),
                "loopback {loopback}"
            );
        }
    }

    #[test]
    fn only_a_loopback_host_may_use_http() {
        for url in [
            "http://127.0.0.1/x",
            "http://127.0.0.1:8080/x",
            "http://localhost:1/x",
            "http://localhost",
            "http://[::1]:9/x",
        ] {
            assert!(is_loopback_http(url), "{url}");
        }
        for url in [
            "https://127.0.0.1/x",
            "http://example.com/x",
            "http://127.0.0.1.example.com/x",
            "http://localhost.example.com/x",
            "http://127.0.0.1@example.com/x",
            "http://user:pw@127.0.0.1/x",
            "http://127.0.0.2/x",
            "http:///x",
        ] {
            assert!(!is_loopback_http(url), "{url}");
        }
    }
}
