//! Task 3.4b at the binary: `eval run` over the fixture set with the scripted provider, then the
//! same command again resuming with nothing redone.
#![cfg(feature = "testkit")]
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn eval_run_completes_a_fake_matrix_and_resumes_without_redoing_it() {
    let out = std::env::temp_dir().join(format!("dejaview-eval-run-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_dejaview"))
            .current_dir(repo_root())
            .args([
                "eval",
                "run",
                "--matrix",
                "fixtures/matrix/fake-a.toml",
                "--out",
                out.to_str().unwrap(),
                "--evalsets-root",
                "fixtures/evalsets",
                "--provider-script",
                "fixtures/scripts/held.json",
            ])
            .output()
            .unwrap()
    };
    let o = run();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let text = String::from_utf8_lossy(&o.stdout);
    assert!(
        text.contains("4 decisions (4 completed now, 0 already complete"),
        "{text}"
    );
    assert!(out.join("summary.json").exists() && out.join("descriptor.json").exists());
    let o = run();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let text = String::from_utf8_lossy(&o.stdout);
    assert!(
        text.contains("(0 completed now, 4 already complete"),
        "{text}"
    );
    // A matrix whose config is missing is a usage error.
    let o = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .current_dir(repo_root())
        .args([
            "eval",
            "run",
            "--matrix",
            "fixtures/matrix/no-such.toml",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(64));
}

#[test]
fn eval_estimate_prints_every_envelope_and_warns_over_the_cap() {
    let o = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .current_dir(repo_root())
        .args([
            "eval",
            "estimate",
            "--matrix",
            "fixtures/matrix/fake-a.toml",
            "--evalsets-root",
            "fixtures/evalsets",
            "--provider-script",
            "fixtures/scripts/held.json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let text = String::from_utf8_lossy(&o.stdout);
    assert!(
        text.contains("4 decisions")
            && text.contains("envelope sum $")
            && !text.contains("warning:"),
        "{text}"
    );
    // A matrix whose envelopes sum past its cap: the warning, and exit 0 all the same.
    let tight =
        std::env::temp_dir().join(format!("dejaview-estimate-cli-{}.toml", std::process::id()));
    let matrix = std::fs::read_to_string(repo_root().join("fixtures/matrix/fake-a.toml"))
        .unwrap()
        .replace("spend_cap_usd = 5.0", "spend_cap_usd = 0.1");
    std::fs::write(&tight, matrix).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .current_dir(repo_root())
        .args([
            "eval",
            "estimate",
            "--matrix",
            tight.to_str().unwrap(),
            "--evalsets-root",
            "fixtures/evalsets",
            "--provider-script",
            "fixtures/scripts/held.json",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let e: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(e["over_cap"], true);
    assert_eq!(e["decisions"].as_array().unwrap().len(), 4);
}
