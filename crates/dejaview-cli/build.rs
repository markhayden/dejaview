//! Records the short git commit at build time so `dejaview --version` can print it. A published
//! crate has no repository, but `cargo package` writes the commit to `.cargo_vcs_info.json`, which
//! is read first when present.
use std::process::Command;

fn main() {
    let sha = packaged_sha()
        .or_else(git_sha)
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=DEJAVIEW_GIT_SHA={sha}");
    // Rebuild when the commit changes: HEAD itself (a checkout), the ref it points at (a new
    // commit on the same branch) and packed-refs (after `git gc`). `--git-path` resolves inside
    // worktrees, where `.git` is a file.
    let branch = git(&["rev-parse", "--symbolic-full-name", "HEAD"]).unwrap_or_default();
    for path in ["HEAD", branch.as_str(), "packed-refs"] {
        if path.is_empty() {
            continue;
        }
        if let Some(p) = git(&["rev-parse", "--git-path", path]) {
            println!("cargo:rerun-if-changed={p}");
        }
    }
}

fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn git_sha() -> Option<String> {
    git(&["rev-parse", "--short=9", "HEAD"])
}

fn packaged_sha() -> Option<String> {
    let text = std::fs::read_to_string(".cargo_vcs_info.json").ok()?;
    let key = "\"sha1\":";
    let start = text.find(key)? + key.len();
    let rest = text[start..].trim_start().strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end.min(9)].to_string())
}
