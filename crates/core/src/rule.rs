//! Rule trait: 检测规则的统一接口。

use std::path::Path;

use crate::config::RuleConfig;
use crate::finding::Finding;
use crate::parser::Language;
use thiserror::Error;
use tree_sitter::Tree;

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("rule failed: {0}")]
    Failed(String),
    #[error("rule panicked")]
    Panic,
}

pub struct AnalysisContext<'a> {
    pub source: &'a str,
    pub tree: &'a Tree,
    pub language: Language,
    pub file_path: &'a Path,
    pub rule_config: &'a RuleConfig,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn severity(&self) -> crate::finding::Severity;
    fn languages(&self) -> &'static [Language];

    fn analyze(&self, ctx: &AnalysisContext) -> Result<Vec<Finding>, RuleError>;
}
