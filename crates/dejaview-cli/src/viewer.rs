//! `dejaview report view`: serves a decision directory and the static viewer on loopback. The
//! viewer assets are embedded, so the installed binary needs nothing else; the server answers
//! only `GET` and `HEAD` for the page, its script and stylesheet, `report.json`, and the run
//! artifacts, named strictly (`run-<n>/artifacts/<sha256>`), so nothing outside the directory can
//! be read; a request whose `Host` is not this server's loopback address is refused, and the
//! page's policy keeps it from loading anything from elsewhere.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

const INDEX_HTML: &str = include_str!("../viewer/index.html");
const VIEWER_JS: &str = include_str!("../viewer/viewer.js");
const GEOMETRY_JS: &str = include_str!("../viewer/geometry.js");
const VIEWER_CSS: &str = include_str!("../viewer/viewer.css");

/// The largest request head the server reads.
const MAX_HEAD_BYTES: usize = 8 * 1024;

pub struct Server {
    listener: TcpListener,
    dir: PathBuf,
}

struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn text(status: u16, content_type: &'static str, body: &str) -> Response {
        Response {
            status,
            content_type,
            body: body.as_bytes().to_vec(),
        }
    }

    fn not_found() -> Response {
        Response::text(404, "text/plain; charset=utf-8", "not found\n")
    }
}

impl Server {
    /// Binds `127.0.0.1:port` (0 picks a free port).
    pub fn bind(dir: &Path, port: u16) -> std::io::Result<Server> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))?;
        Ok(Server {
            listener,
            dir: dir.to_path_buf(),
        })
    }

    pub fn addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Serves until the process ends; one thread per connection, each closed after its response.
    pub fn serve_forever(&self) -> std::io::Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream?;
            let dir = self.dir.clone();
            std::thread::spawn(move || {
                let _ = handle(stream, &dir);
            });
        }
        Ok(())
    }
}

fn handle(mut stream: TcpStream, dir: &Path) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader
        .by_ref()
        .take(MAX_HEAD_BYTES as u64)
        .read_line(&mut request_line)?;
    // The headers (bounded): only `Host` matters, and it must name this loopback server, so a
    // page from elsewhere cannot reach the report through a rebound name.
    let mut read = request_line.len();
    let mut host = None;
    loop {
        let mut line = String::new();
        let n = reader
            .by_ref()
            .take((MAX_HEAD_BYTES.saturating_sub(read)) as u64)
            .read_line(&mut line)?;
        read += n;
        if n == 0 || line == "\r\n" || line == "\n" || read >= MAX_HEAD_BYTES {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("host")
        {
            host = Some(value.trim().to_ascii_lowercase());
        }
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let port = stream.local_addr().map(|a| a.port()).unwrap_or(0);
    let host_ok = host
        .as_deref()
        .is_some_and(|h| h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}"));
    let response = if !host_ok {
        Response::text(421, "text/plain; charset=utf-8", "wrong host\n")
    } else {
        match method {
            "GET" | "HEAD" => route(dir, target.split('?').next().unwrap_or("")),
            _ => Response::text(405, "text/plain; charset=utf-8", "method not allowed\n"),
        }
    };
    let reason = match response.status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        421 => "Misdirected Request",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; img-src 'self' data:\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream.write_all(head.as_bytes())?;
    if method != "HEAD" {
        stream.write_all(&response.body)?;
    }
    stream.flush()
}

/// The routes: the embedded page, `report.json`, and `run-<n>/artifacts/<sha256>`.
fn route(dir: &Path, path: &str) -> Response {
    match path {
        "/" | "/index.html" => Response::text(200, "text/html; charset=utf-8", INDEX_HTML),
        "/viewer.js" => Response::text(200, "text/javascript; charset=utf-8", VIEWER_JS),
        "/geometry.js" => Response::text(200, "text/javascript; charset=utf-8", GEOMETRY_JS),
        "/viewer.css" => Response::text(200, "text/css; charset=utf-8", VIEWER_CSS),
        "/report.json" => match std::fs::read(dir.join("report.json")) {
            Ok(body) => Response {
                status: 200,
                content_type: "application/json",
                body,
            },
            Err(_) => Response::not_found(),
        },
        other => match artifact_path(other) {
            Some(rel) => match std::fs::read(dir.join(rel)) {
                Ok(body) => Response {
                    status: 200,
                    content_type: sniff(&body),
                    body,
                },
                Err(_) => Response::not_found(),
            },
            None => Response::not_found(),
        },
    }
}

/// `/run-<n>/artifacts/<64 lowercase hex>` and nothing else, as a relative path.
fn artifact_path(path: &str) -> Option<PathBuf> {
    let rest = path.strip_prefix("/run-")?;
    let (n, rest) = rest.split_once('/')?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let sha = rest.strip_prefix("artifacts/")?;
    if sha.len() != 64 || !sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    Some(
        PathBuf::from(format!("run-{n}"))
            .join("artifacts")
            .join(sha),
    )
}

/// Artifacts carry no type of their own; images and JSON are told apart by their first bytes.
fn sniff(body: &[u8]) -> &'static str {
    if body.starts_with(b"\x89PNG") {
        "image/png"
    } else if body.starts_with(b"\xFF\xD8") {
        "image/jpeg"
    } else if body.starts_with(b"RIFF") && body.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        match body.iter().find(|b| !b.is_ascii_whitespace()) {
            Some(b'{') | Some(b'[') => "application/json",
            _ => "application/octet-stream",
        }
    }
}

/// Opens the URL in the system browser; failures only warn.
pub fn open_in_browser(url: &str) {
    let (program, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else if cfg!(target_os = "windows") {
        ("cmd", vec!["/c", "start", "", url])
    } else {
        ("xdg-open", vec![url])
    };
    if let Err(e) = std::process::Command::new(program).args(args).spawn() {
        eprintln!("dejaview: cannot open a browser ({program}: {e}); open {url} yourself");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_strictly_named_artifacts_resolve() {
        let sha = "a".repeat(64);
        assert_eq!(
            artifact_path(&format!("/run-1/artifacts/{sha}")),
            Some(PathBuf::from("run-1").join("artifacts").join(&sha))
        );
        for bad in [
            "/run-1/artifacts/../report.json",
            "/run-x/artifacts/",
            "/run-/artifacts/",
            "/run-1/frames/raw_candidate.png",
            "/../etc/passwd",
            &format!("/run-1/artifacts/{}", "A".repeat(64)),
            &format!("/run-1/artifacts/{}", "a".repeat(63)),
        ] {
            assert_eq!(artifact_path(bad), None, "{bad}");
        }
    }

    #[test]
    fn content_types_follow_the_bytes() {
        assert_eq!(sniff(b"\x89PNG\r\n"), "image/png");
        assert_eq!(sniff(b"\xFF\xD8\xFF"), "image/jpeg");
        assert_eq!(sniff(b"  {\"a\":1}"), "application/json");
        assert_eq!(sniff(b"[1]"), "application/json");
        assert_eq!(sniff(b"hello"), "application/octet-stream");
    }
}
