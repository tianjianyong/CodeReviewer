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
    /// 本规则跳过的文件（文件名子串匹配，不区分大小写）
    #[serde(default)]
    pub exclude_files: Vec<String>,
}

impl RuleConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 文件是否被本规则排除。
    pub fn excludes_file(&self, file: &Path) -> bool {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        self.exclude_files
            .iter()
            .any(|p| name.contains(&p.to_lowercase()))
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

    /// 从扫描路径向上查找 .codereviewer.toml；找不到则回退从当前目录向上找；都没有用默认。
    pub fn load_auto_from(start: &Path) -> Result<Self, std::io::Error> {
        if let Some(p) = find_up_from(start) {
            return Self::load_from_file(&p);
        }
        Self::load_auto()
    }

    /// 从当前目录向上查找 .codereviewer.toml 并加载；找不到返回默认配置。
    pub fn load_auto() -> Result<Self, std::io::Error> {
        match std::env::current_dir().ok().and_then(|d| find_up_from(&d)) {
            Some(p) => Self::load_from_file(&p),
            None => Ok(Self::default()),
        }
    }
}

/// 从 start 目录向上查找 .codereviewer.toml。
fn find_up_from(start: &Path) -> Option<std::path::PathBuf> {
    let mut current = if start.is_dir() {
        start
    } else {
        start.parent()?
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_file_matches_file_name_substring_case_insensitive() {
        let cfg = RuleConfig {
            exclude_files: vec!["testautomationclient".to_string()],
            ..Default::default()
        };
        assert!(cfg.excludes_file(Path::new("src/NavisworksTestAutomationClient.cs")));
        assert!(!cfg.excludes_file(Path::new("src/Commands/AutoPathPlanningCommand.cs")));
    }
}
