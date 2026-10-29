//! Task 3.6 at the binary: `report view` serves the embedded viewer, `report.json` and the run
//! artifacts of a decision directory on loopback, and nothing else. The devtools check of the
//! page itself (no console errors, loopback-only requests, the crop-frame overlay at its mapped
//! position) is recorded in docs/viewer.md.
#![cfg(feature = "testkit")]
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Kills the server when the test ends, panic or not.
struct Serving(Child);

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn get(url: &str, path: &str) -> (u16, String, Vec<u8>) {
    let host = url.trim_start_matches("http://").trim_end_matches('/');
    let mut s = TcpStream::connect(host).unwrap();
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status: u16 = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, head, raw[split + 4..].to_vec())
}

#[test]
fn report_view_serves_the_viewer_the_report_and_only_the_run_artifacts() {
    // A decision with a tool call that returned a crop frame.
    let out = std::env::temp_dir().join(format!("dejaview-viewer-serve-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let o = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .current_dir(repo_root())
        .args([
            "check",
            "--config",
            "fixtures/config/fake-tool.toml",
            "--job",
            "fixtures/jobs/pair.toml",
            "--provider-script",
            "fixtures/scripts/one-call-crop-finding.json",
            "--tool-script",
            "fixtures/tool-scripts/crop-image.json",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        matches!(o.status.code(), Some(0 | 1)),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let dir = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let report = std::fs::read(dir.join("report.json")).unwrap();
    let r: serde_json::Value = serde_json::from_slice(&report).unwrap();
    assert!(r["runs"][0]["frames"]["crop:call:1:0"]["mapping"]["bbox"].is_array());
    // The finding in the crop frame is mapped to raw coordinates of the candidate, which is what
    // the viewer draws on the root image.
    let root = &r["runs"][0]["findings"][0]["root"];
    assert_eq!(root["image"], "candidate");
    assert_eq!(root["bbox"], serde_json::json!([0.1, 0.1, 0.3, 0.3]));

    let mut child = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .args(["report", "view", dir.to_str().unwrap(), "--port", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut first)
        .unwrap();
    let _serving = Serving(child);
    let url = first
        .split_whitespace()
        .find(|w| w.starts_with("http://127.0.0.1:"))
        .unwrap_or_else(|| panic!("no URL in {first:?}"))
        .to_string();

    let (status, head, body) = get(&url, "/");
    assert_eq!(status, 200);
    assert!(head.contains("text/html"), "{head}");
    assert!(String::from_utf8_lossy(&body).contains("<title>dejaview report</title>"));
    let (status, head, body) = get(&url, "/viewer.js");
    assert_eq!(status, 200);
    assert!(head.contains("text/javascript"), "{head}");
    let js = String::from_utf8_lossy(&body).to_string();
    assert!(js.contains("SUPPORTED_REPORT_VERSIONS = [1]"));
    assert!(
        js.contains("is not supported by this viewer"),
        "the refusal branch for other versions"
    );
    let (status, head, geometry) = get(&url, "/geometry.js");
    assert_eq!(status, 200);
    assert!(head.contains("text/javascript"), "{head}");
    let geometry = String::from_utf8_lossy(&geometry).to_string();
    assert!(geometry.contains("dejaviewGeometry"));
    let (status, _, css) = get(&url, "/viewer.css");
    assert_eq!(status, 200);
    // Nothing in the page reaches off the server that served it: no absolute URL anywhere but
    // the SVG namespace, no stylesheet import, no preconnect.
    let page = String::from_utf8_lossy(&get(&url, "/").2).to_string();
    let css = String::from_utf8_lossy(&css).to_string();
    for (name, text) in [
        ("index.html", &page),
        ("viewer.js", &js),
        ("geometry.js", &geometry),
        ("viewer.css", &css),
    ] {
        for (i, _) in text.match_indices("http") {
            let hit = &text[i..(i + 30).min(text.len())];
            assert!(
                hit.starts_with("http://www.w3.org/2000/svg"),
                "{name}: {hit}"
            );
        }
        assert!(
            !text.contains("@import") && !text.contains("preconnect"),
            "{name}"
        );
    }
    let (status, head, body) = get(&url, "/report.json");
    assert_eq!(status, 200);
    assert!(head.contains("application/json"), "{head}");
    assert_eq!(body, report, "the report is served byte for byte");

    // Artifacts: an image the model saw and the tool envelope, typed by their bytes.
    let sent = r["runs"][0]["images_sent"].as_object().unwrap();
    let candidate = sent["raw:candidate"]["sha256"].as_str().unwrap();
    let (status, head, body) = get(&url, &format!("/run-1/artifacts/{candidate}"));
    assert_eq!(status, 200);
    assert!(head.contains("image/"), "{head}");
    assert_eq!(
        body,
        std::fs::read(dir.join("run-1/artifacts").join(candidate)).unwrap()
    );
    let envelope = r["runs"][0]["tool_calls"][0]["envelope"]["sha256"]
        .as_str()
        .unwrap();
    let (status, head, _) = get(&url, &format!("/run-1/artifacts/{envelope}"));
    assert_eq!(status, 200);
    assert!(head.contains("application/json"), "{head}");

    // Nothing else: traversal, other files of the directory, unknown artifacts, other methods.
    for path in [
        "/run-1/artifacts/../../report.json",
        "/run-1/journal.jsonl",
        "/run-1/frames/raw_candidate.png",
        "/decision.json",
        "/../Cargo.toml",
        &format!("/run-1/artifacts/{}", "0".repeat(64)),
        "/run-1/artifacts/short",
    ] {
        assert_eq!(get(&url, path).0, 404, "{path}");
    }
    let host = url.trim_start_matches("http://").trim_end_matches('/');
    let mut s = TcpStream::connect(host).unwrap();
    write!(
        s,
        "POST / HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    assert!(raw.starts_with("HTTP/1.1 405"), "{raw}");
    assert!(
        head.contains("Connection: close") && head.contains("nosniff"),
        "{head}"
    );
    assert!(
        head.contains("Content-Security-Policy: default-src 'self'"),
        "{head}"
    );
    // A request for another host name (DNS rebinding) is refused before any route.
    let mut s = TcpStream::connect(host).unwrap();
    write!(
        s,
        "GET /report.json HTTP/1.1\r\nHost: evil.example\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    assert!(raw.starts_with("HTTP/1.1 421"), "{raw}");
    let mut s = TcpStream::connect(host).unwrap();
    write!(s, "GET /report.json HTTP/1.1\r\nConnection: close\r\n\r\n").unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    assert!(raw.starts_with("HTTP/1.1 421"), "no Host at all: {raw}");

    // A directory without a report is a usage error.
    let o = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .args(["report", "view", out.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(64));
}
