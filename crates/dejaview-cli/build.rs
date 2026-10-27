//! Records the short git commit at build time so `dejaview --version` can print it. A published
//! crate has no repository, but `cargo package` writes the commit to `.cargo_vcs_info.json`, which
//! is read first when present.
use std::process::Command;

fn main() {
    let sha = packaged_sha()
        .or_else(git_sha)
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

fn git_sha() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--short=9", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn packaged_sha() -> Option<String> {
    let text = std::fs::read_to_string(".cargo_vcs_info.json").ok()?;
    let key = "\"sha1\":";
    let start = text.find(key)? + key.len();
    let rest = text[start..].trim_start().strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end.min(9)].to_string())
}
