//! Records the short git commit at build time so `dejaview --version` can print it.
use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short=9", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=DEJAVIEW_GIT_SHA={sha}");
    // `--git-path` resolves correctly inside worktrees, where `.git` is a file.
    if let Some(head) = Command::new("git")
        .args(["rev-parse", "--git-path", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    {
        println!("cargo:rerun-if-changed={head}");
    }
}
