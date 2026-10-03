//! Environment profiles. One TOML file lists every MSK cluster you touch
//! (stag / preprod / prod / regression), each with its own bootstrap + region
//! and a `prod` flag that drives the delete guardrail.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct EnvProfile {
    /// Display name, e.g. "stag", "prod".
    pub name: String,
    /// One or more brokers, comma-separated. Port picks the protocol on MSK:
    /// 9092 = plaintext, 9094 = tls, 9098 = IAM.
    pub bootstrap: String,
    /// AWS region of the cluster (only used for IAM auth).
    pub region: String,
    /// Wire protocol; defaults to IAM.
    #[serde(default)]
    pub auth: Auth,
    /// AWS profile to use for creds (optional; falls back to default chain).
    #[serde(default)]
    pub aws_profile: Option<String>,
    /// Marks a production cluster - destructive ops require typed confirmation.
    #[serde(default)]
    pub prod: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    /// SASL_SSL + MSK IAM (OAUTHBEARER), port 9098.
    #[default]
    Iam,
    /// SSL, no auth, port 9094.
    #[serde(alias = "ssl")]
    Tls,
    /// No TLS, no auth - typical for VPC-internal 9092.
    Plaintext,
}

impl Auth {
    pub fn as_str(self) -> &'static str {
        match self {
            Auth::Iam => "iam",
            Auth::Tls => "tls",
            Auth::Plaintext => "plaintext",
        }
    }
}

impl EnvProfile {
    /// Warns when a broker uses a well-known MSK port for a different auth
    /// mode (e.g. 9092 with auth = "iam"), the classic copy-paste mistake.
    /// Non-MSK ports are not second-guessed.
    pub fn port_mismatch(&self) -> Option<String> {
        self.bootstrap.split(',').find_map(|hp| {
            let port = hp.trim().rsplit_once(':')?.1;
            let implied = match port {
                "9092" => Auth::Plaintext,
                "9094" | "9194" => Auth::Tls,
                "9098" | "9198" => Auth::Iam,
                _ => return None,
            };
            (implied != self.auth).then(|| {
                format!(
                    "port {port} is MSK's {} port, but auth = \"{}\"",
                    implied.as_str(),
                    self.auth.as_str()
                )
            })
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(rename = "env")]
    pub envs: Vec<EnvProfile>,
}

impl Config {
    /// Loads config, preferring `./kitz.toml` then `~/.config/kitz/config.toml`.
    pub fn load() -> Result<Self> {
        let path = Self::locate()
            .context("no config found - create ./kitz.toml or ~/.config/kitz/config.toml")?;
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        anyhow::ensure!(!cfg.envs.is_empty(), "config has no [[env]] entries");
        Ok(cfg)
    }

    fn locate() -> Option<PathBuf> {
        let local = PathBuf::from("kitz.toml");
        if local.exists() {
            return Some(local);
        }
        let global = dirs::config_dir()?.join("kitz").join("config.toml");
        global.exists().then_some(global)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth_of(toml_auth: &str) -> Result<Auth, toml::de::Error> {
        let raw = format!("[[env]]\nname='x'\nbootstrap='b:1'\nregion='r'\n{toml_auth}");
        toml::from_str::<Config>(&raw).map(|c| c.envs[0].auth)
    }

    #[test]
    fn auth_parses_all_modes_and_rejects_typos() {
        assert_eq!(auth_of("").unwrap(), Auth::Iam);
        assert_eq!(auth_of("auth='iam'").unwrap(), Auth::Iam);
        assert_eq!(auth_of("auth='tls'").unwrap(), Auth::Tls);
        assert_eq!(auth_of("auth='ssl'").unwrap(), Auth::Tls);
        assert_eq!(auth_of("auth='plaintext'").unwrap(), Auth::Plaintext);
        assert!(auth_of("auth='IAM'").is_err());
    }

    #[test]
    fn port_mismatch_flags_known_msk_ports_only() {
        let env = |bootstrap: &str, auth| EnvProfile {
            name: "x".into(),
            bootstrap: bootstrap.into(),
            region: "r".into(),
            auth,
            aws_profile: None,
            prod: false,
        };
        assert!(env("b:9092,c:9092", Auth::Iam).port_mismatch().is_some());
        assert!(env("b:9098", Auth::Iam).port_mismatch().is_none());
        assert!(env("b:9094", Auth::Tls).port_mismatch().is_none());
        assert!(env("localhost:19092", Auth::Iam).port_mismatch().is_none());
    }
}
