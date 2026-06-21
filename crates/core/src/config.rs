//! 配置: TOML 加载，可覆盖规则阈值与启用状态。

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RuleConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub thresholds: HashMap<String, toml::Value>,
}

impl RuleConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn threshold_i64(&self, key: &str, default: i64) -> i64 {
        self.thresholds
            .get(key)
            .and_then(|v| v.as_integer())
            .unwrap_or(default)
    }
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load_from_file(path: &Path) -> Result<Self, std::io::Error> {
        let text = std::fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
