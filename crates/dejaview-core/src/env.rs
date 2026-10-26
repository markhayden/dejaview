//! The environment allowlist for tool packs (SPEC "MCP over stdio", startup): `PATH`, `HOME`, the
//! locale, and the variables the pack declares, minus every name the harness knows as a credential
//! source and every name ending in `_KEY`, `_TOKEN` or `_SECRET`. Pure: the caller supplies the
//! environment it saw.

use std::collections::BTreeMap;

use crate::config::is_credential_name;

/// Names every pack receives when set.
pub const ALWAYS: &[&str] = &["PATH", "HOME", "LANG", "LANGUAGE", "TZ"];

/// Whether a name is kept for a pack declaring `declared`.
pub fn allowed(name: &str, declared: &[String]) -> bool {
    if is_credential_name(name) {
        return false;
    }
    ALWAYS.contains(&name) || name.starts_with("LC_") || declared.iter().any(|d| d == name)
}

/// Filters an environment down to what a pack may see.
pub fn allowlist<I, K, V>(declared: &[String], environment: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    environment
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .filter(|(k, _)| allowed(k, declared))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_allowlist_table() {
        let declared: Vec<String> = ["DEJAVIEW_TOOLS_THREADS", "OPENAI_API_KEY", "MY_TOKEN"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let cases: &[(&str, bool)] = &[
            ("PATH", true),
            ("HOME", true),
            ("LANG", true),
            ("LC_ALL", true),
            ("LC_CTYPE", true),
            ("TZ", true),
            ("DEJAVIEW_TOOLS_THREADS", true), // declared
            ("OPENAI_API_KEY", false),        // declared, still a credential
            ("MY_TOKEN", false),              // declared, still a credential by suffix
            ("ANTHROPIC_API_KEY", false),
            ("AWS_SESSION_TOKEN", false),
            ("SOME_SECRET", false),
            ("SHELL", false), // not declared
            ("DEJAVIEW_OTHER", false),
            ("PWD", false),
        ];
        for (name, keep) in cases {
            assert_eq!(allowed(name, &declared), *keep, "{name}");
        }
    }

    #[test]
    fn env_allowlist_filters_a_whole_environment() {
        let declared = vec!["PACK_LEVEL".to_string()];
        let env = [
            ("PATH", "/usr/bin"),
            ("HOME", "/home/u"),
            ("ANTHROPIC_API_KEY", "SYNTHETIC"),
            ("PACK_LEVEL", "2"),
            ("EDITOR", "vi"),
        ];
        let kept = allowlist(&declared, env);
        assert_eq!(
            kept.keys().map(String::as_str).collect::<Vec<_>>(),
            ["HOME", "PACK_LEVEL", "PATH"]
        );
    }
}
