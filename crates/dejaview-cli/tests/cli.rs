//! Smoke tests for the binary. These run against the executable Cargo built for this test run,
//! so with the `testkit` feature off they prove a feature-off binary rejects the scripted flag,
//! and with it on they prove the flag exists.

use std::process::Command;

fn dejaview() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dejaview"))
}

#[test]
fn version_prints_crate_version_and_commit() {
    let out = dejaview().arg("--version").output().expect("run dejaview");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains(env!("CARGO_PKG_VERSION")), "got {text}");
    assert!(
        text.contains('('),
        "expected a commit in parentheses, got {text}"
    );
}

#[test]
fn no_command_is_a_usage_error() {
    let out = dejaview().output().expect("run dejaview");
    assert_eq!(out.status.code(), Some(64));
}

#[cfg(not(feature = "testkit"))]
#[test]
fn feature_off_binary_rejects_provider_script() {
    let out = dejaview()
        .args(["check", "--provider-script", "x.json"])
        .output()
        .expect("run dejaview");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unexpected argument"), "got {err}");
}

#[cfg(feature = "testkit")]
#[test]
fn feature_on_binary_accepts_provider_script() {
    let out = dejaview()
        .args(["check", "--provider-script", "x.json"])
        .output()
        .expect("run dejaview");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("unexpected argument"), "got {err}");
}
