//! R09: 注释掉的代码块检测。
//!
//! 启发式：连续多行注释，且内容看起来像代码（包含分号、括号、等号等）。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct CommentedCode;

impl Rule for CommentedCode {
    fn id(&self) -> &'static str {
        "R09"
    }
    fn name(&self) -> &'static str {
        "commented-out-code"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
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
        let min_block = ctx.rule_config.threshold_i64("min_block_lines", 3) as usize;
        let mut findings = Vec::new();
        let lines: Vec<&str> = ctx.source.lines().collect();

        let comment_prefix = comment_prefix(ctx.language);
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim_start();
            if trimmed.starts_with(comment_prefix) {
                let start = i;
                let mut count = 0;
                while i < lines.len() {
                    let t = lines[i].trim_start();
                    if t.starts_with(comment_prefix) {
                        let content = t.strip_prefix(comment_prefix).unwrap_or(t).trim();
                        if looks_like_code(content) {
                            count += 1;
                            i += 1;
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        break;
                    }
                }
                if count >= min_block {
                    findings.push(Finding {
                        rule_id: "R09",
                        rule_name: "commented-out-code",
                        severity: Severity::Warning,
                        location: Location {
                            file: ctx.file_path.to_path_buf(),
                            line: start + 1,
                            column: 1,
                        },
                        message: format!(
                            "注释掉的代码块：{} 行（下限 {}） | commented-out code block: {} lines (min {})",
                            count, min_block, count, min_block
                        ),
                        snippet: None,
                    });
                }
            } else {
                i += 1;
            }
        }
        Ok(findings)
    }
}

fn comment_prefix(lang: Language) -> &'static str {
    match lang {
        Language::Python => "#",
        _ => "//",
    }
}

fn looks_like_code(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }
    // 说明性/文档性注释（注意/说明/警告/TODO 等）不是注释掉的代码
    let doc_markers = [
        "注意", "说明", "警告", "备注", "TODO", "FIXME", "HACK", "XXX", "NOTE", "Note",
    ];
    if doc_markers.iter().any(|m| content.starts_with(m)) {
        return false;
    }
    let code_signals = [';', '{', '}', '=', '(', ')', '<', '>'];
    let signal_count = content.chars().filter(|c| code_signals.contains(c)).count();
    signal_count >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::analyze_source;

    #[test]
    fn doc_marker_lines_not_counted_as_code() {
        // 3 行里 1 行是说明注释 → 代码行不足 3，不报
        let source = "// 注意：不要删除
// var x = 1;
// var y = 2;
";
        assert!(analyze_source(&CommentedCode, source, Language::CSharp).is_empty());
    }

    #[test]
    fn consecutive_code_lines_still_flagged() {
        let source = "// var x = 1;
// var y = 2;
// var z = 3;
";
        let findings = analyze_source(&CommentedCode, source, Language::CSharp);
        assert_eq!(findings.len(), 1);
    }
}
