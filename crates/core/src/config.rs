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

    /// 从当前目录向上查找 .codereviewer.toml 并加载；找不到返回默认配置。
    pub fn load_auto() -> Result<Self, std::io::Error> {
        match Self::find_project_config() {
            Some(p) => Self::load_from_file(&p),
            None => Ok(Self::default()),
        }
    }

    /// 从当前目录向上查找 .codereviewer.toml。
    pub fn find_project_config() -> Option<std::path::PathBuf> {
        let dir = std::env::current_dir().ok()?;
        let mut current = dir.as_path();
        loop {
            let candidate = current.join(".codereviewer.toml");
            if candidate.is_file() {
                return Some(candidate);
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => return None,
            }
        }
    }
}
