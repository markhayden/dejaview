//! Task 1.5a CLI scenario: `tools list` prints the fake tool's manifest as the model will see it.
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
fn tools_list_prints_the_enabled_manifests() {
    let o = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .current_dir(repo_root())
        .args([
            "tools",
            "list",
            "--config",
            "fixtures/config/fake-tool.toml",
        ])
        .output()
        .unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("fake_regions v0.0.1"), "{out}");
    assert!(out.contains("evidence kind: change_region"), "{out}");
    assert!(
        out.contains("model schema:") && out.contains("\"image\""),
        "{out}"
    );
    assert!(
        !out.contains("_dejaview"),
        "the model schema never mentions the harness key: {out}"
    );
}

#[test]
fn tools_list_refuses_an_unknown_enabled_tool() {
    let o = Command::new(env!("CARGO_BIN_EXE_dejaview"))
        .current_dir(repo_root())
        .args(["tools", "list", "--config", "fixtures/config/full.toml"])
        .env("DEJAVIEW_TEST_ACCOUNT", "acct")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&o.stderr).contains("not known to this build"));
}
