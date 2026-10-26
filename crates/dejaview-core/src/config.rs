//! The config file. Task 1.1 reads the three sections the walking slice needs; the full schema,
//! `${VAR}` expansion, CLI overrides and the committed JSON Schema arrive in task 1.3.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub provider: ProviderConfig,
    pub privacy: PrivacyConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub profile: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyConfig {
    pub cloud_calls: bool,
    pub account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    pub dir: PathBuf,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("cannot read {}: {e}", path.display())))?;
        let config: Config =
            toml::from_str(&text).map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
        if !config.privacy.cloud_calls {
            return Err(Error::Config(
                "[privacy] cloud_calls must be true: dejaview sends images and text to the configured \
                 provider, and refuses to run without that explicit opt-in"
                    .into(),
            ));
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_minimal_fixture() {
        let text = include_str!("../../../fixtures/config/minimal.toml");
        let config: Config = toml::from_str(text).expect("parses");
        assert_eq!(config.provider.profile, "fake");
        assert!(config.privacy.cloud_calls);
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let text = "[provider]\nprofile = \"fake\"\nmodel = \"m\"\nbogus = 1\n[privacy]\ncloud_calls = true\naccount = \"a\"\n[output]\ndir = \"o\"\n";
        assert!(toml::from_str::<Config>(text).is_err());
    }
}
