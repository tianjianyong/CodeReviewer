//! Finding: 一次检测发现的数据结构。

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    /// 机器可读的英文 key（用于 JSON 序列化、CLI --severity 解析、MCP 输出）。
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// 面向用户的中文+英文展示标签（用于终端文本输出）。
    pub fn display_label(self) -> &'static str {
        match self {
            Severity::Error => "错误(error)",
            Severity::Warning => "警告(warning)",
            Severity::Info => "信息(info)",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule_id: &'static str,
    pub rule_name: &'static str,
    pub severity: Severity,
    pub location: Location,
    pub message: String,
    pub snippet: Option<String>,
}
