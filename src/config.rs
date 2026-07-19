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
    /// Wire protocol: "iam" (SASL_SSL + MSK IAM, default), "tls" (SSL, no auth),
    /// or "plaintext" (no TLS, no auth - typical for VPC-internal 9092).
    #[serde(default = "default_auth")]
    pub auth: String,
    /// AWS profile to use for creds (optional; falls back to default chain).
    #[serde(default)]
    pub aws_profile: Option<String>,
    /// Marks a production cluster - destructive ops require typed confirmation.
    #[serde(default)]
    pub prod: bool,
}

fn default_auth() -> String {
    "iam".to_string()
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
