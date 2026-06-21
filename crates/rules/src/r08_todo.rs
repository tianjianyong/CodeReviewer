//! R08: TODO/FIXME 堆积检测。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct TodoFixme;

impl Rule for TodoFixme {
    fn id(&self) -> &'static str {
        "R08"
    }
    fn name(&self) -> &'static str {
        "todo-fixme-accumulation"
    }
    fn severity(&self) -> Severity {
        Severity::Info
    }
    fn languages(&self) -> &'static [Language] {
        &[
            Language::Rust,
            Language::Python,
            Language::TypeScript,
            Language::TypeScriptTsx,
            Language::CSharp,
            Language::Java,
        ]
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Result<Vec<Finding>, RuleError> {
        let mut findings = Vec::new();
        for (i, line) in ctx.source.lines().enumerate() {
            let lower = line.to_lowercase();
            if lower.contains("todo") || lower.contains("fixme") || lower.contains("xxx") {
                findings.push(Finding {
                    rule_id: "R08",
                    rule_name: "todo-fixme-accumulation",
                    severity: Severity::Info,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: i + 1,
                        column: 1,
                    },
                    message: "TODO/FIXME marker found".to_string(),
                    snippet: Some(line.trim().to_string()),
                });
            }
        }
        Ok(findings)
    }
}
