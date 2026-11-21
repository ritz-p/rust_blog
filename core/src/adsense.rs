use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::LazyLock};

pub const SCRIPT: &str = include_str!("../assets/adsense.js");
pub const DEFAULT_FETCH_CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https: data:; object-src 'none'; base-uri 'self'; form-action 'self'";
pub static DEFAULT_CSP: LazyLock<String> =
    LazyLock::new(|| format!("{DEFAULT_FETCH_CSP}; frame-ancestors 'none'"));

fn integrity(bytes: &[u8]) -> String {
    format!("sha256-{}", STANDARD.encode(Sha256::digest(bytes)))
}

pub static NAV_INTEGRITY: LazyLock<String> =
    LazyLock::new(|| integrity(include_bytes!("../assets/nav.js")));
pub static AD_INTEGRITY: LazyLock<String> = LazyLock::new(|| integrity(SCRIPT.as_bytes()));
pub static AD_CSP: LazyLock<String> = LazyLock::new(|| {
    format!(
        "default-src 'self'; script-src '{}' '{}' 'strict-dynamic' 'unsafe-eval'; style-src 'self' 'unsafe-inline' https:; img-src 'self' https: data:; connect-src 'self' https:; frame-src https:; font-src 'self' https: data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'",
        *NAV_INTEGRITY, *AD_INTEGRITY
    )
});

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdSense {
    enabled: bool,
    ads_txt: String,
    publisher_id: String,
    sidebar_slot: String,
}

#[derive(Serialize)]
pub struct Sidebar<'a> {
    publisher_id: &'a str,
    slot: &'a str,
    script_integrity: &'static str,
}

impl AdSense {
    pub fn from_config(map: &HashMap<String, String>) -> Result<Self> {
        let Some(section) = map.get("adsense") else {
            return Ok(Self::default());
        };
        let config: Self = toml::from_str(section).context("invalid [adsense] configuration")?;
        ensure!(
            !config.enabled || !config.ads_txt.trim().is_empty(),
            "adsense.ads_txt is required when enabled"
        );
        Ok(config)
    }

    pub fn ads_txt(&self) -> Option<&str> {
        self.enabled.then_some(self.ads_txt.as_str())
    }

    pub fn sidebar(&self, requested: bool) -> Result<Option<Sidebar<'_>>> {
        if !(requested && self.enabled) {
            return Ok(None);
        }
        let publisher = self
            .publisher_id
            .strip_prefix("ca-pub-")
            .unwrap_or_default();
        ensure!(
            publisher.len() == 16 && publisher.bytes().all(|b| b.is_ascii_digit()),
            "adsense.publisher_id must be ca-pub- followed by 16 digits"
        );
        ensure!(
            !self.sidebar_slot.is_empty() && self.sidebar_slot.bytes().all(|b| b.is_ascii_digit()),
            "adsense.sidebar_slot must contain the numeric ad unit ID"
        );
        Ok(Some(Sidebar {
            publisher_id: &self.publisher_id,
            slot: &self.sidebar_slot,
            script_integrity: &AD_INTEGRITY,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seller_file_is_independent_of_sidebar_credentials() {
        let mut cfg = AdSense::default();
        assert!(cfg.ads_txt().is_none());
        assert!(cfg.sidebar(true).unwrap().is_none());
        cfg.enabled = true;
        cfg.ads_txt = "first\nsecond\n".into();
        assert_eq!(cfg.ads_txt(), Some("first\nsecond\n"));
        assert!(cfg.sidebar(false).unwrap().is_none());
        assert!(
            cfg.sidebar(true)
                .err()
                .unwrap()
                .to_string()
                .contains("publisher_id")
        );
        cfg.publisher_id = "ca-pub-0123456789012345".into();
        assert!(
            cfg.sidebar(true)
                .err()
                .unwrap()
                .to_string()
                .contains("sidebar_slot")
        );
        cfg.sidebar_slot = "0123456789".into();
        assert_eq!(cfg.sidebar(true).unwrap().unwrap().slot, "0123456789");
    }
}
