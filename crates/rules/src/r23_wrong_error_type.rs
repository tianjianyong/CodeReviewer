//! R23: 错误类型传播丢信息检测。
//!
//! 启发式：宽泛 catch/except（Exception/base）体内返回固定状态码或泛型错误，
//! 且不引用异常变量——丢失了本应区分的错误信息。
//! 与 R01 互补：R01 是吞错误，R23 是错误有传播但类型丢信息。

use codereviewer_core::ast::{node_text, walk};
use codereviewer_core::finding::{Finding, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

use crate::common::{body_uses_word, exception_var, is_broad_catch, returns_generic};

pub struct WrongErrorTypePropagation;

impl Rule for WrongErrorTypePropagation {
    fn id(&self) -> &'static str {
        "R23"
    }
    fn name(&self) -> &'static str {
        "wrong-error-type-propagation"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn languages(&self) -> &'static [Language] {
        // Rust 无 catch/except 语法
        &[
            Language::Python,
            Language::TypeScript,
            Language::TypeScriptTsx,
            Language::CSharp,
            Language::Java,
        ]
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Result<Vec<Finding>, RuleError> {
        let catch_kind = catch_kind(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if node.kind() != catch_kind {
                return;
            }
            if !is_broad_catch(&node, ctx) {
                return;
            }
            let body_text = node_text(&node, ctx.source);
            let exc_var = exception_var(&node, ctx);
            // 头部之后的体内按词边界引用了异常变量 → 在用信息，不报
            if !exc_var.is_empty() && body_uses_word(body_text, &exc_var) {
                return;
            }
            if returns_generic(&node, ctx) {
                findings.push(Finding::new(
                    "R23",
                    "wrong-error-type-propagation",
                    Severity::Warning,
                    ctx.file_path,
                    &node,
                    ctx.source,
                    format!(
                        "宽泛 {} 返回固定值且不引用异常变量，丢失错误类型信息 | broad catch returns fixed value without inspecting exception, loses error type info",
                        catch_kind
                    ),
                ));
            }
        });

        Ok(findings)
    }
}

fn catch_kind(lang: Language) -> &'static str {
    match lang {
        Language::Python => "except_clause",
        _ => "catch_clause",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::analyze_source;

    #[test]
    fn broad_except_unused_var_flagged() {
        let source =
            "def f():\n    try:\n        pass\n    except Exception as e:\n        return None\n";
        let findings = analyze_source(&WrongErrorTypePropagation, source, Language::Python);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "R23");
    }

    #[test]
    fn broad_except_var_used_not_flagged() {
        let source = "def f():\n    try:\n        pass\n    except Exception as e:\n        print(e)\n        return None\n";
        let findings = analyze_source(&WrongErrorTypePropagation, source, Language::Python);
        assert!(findings.is_empty());
    }

    #[test]
    fn contains_word_respects_boundaries() {
        let cw = codereviewer_core::ast::contains_word;
        assert!(cw("print(e)", "e"));
        assert!(!cw("return", "e"));
        assert!(!cw("error", "e"));
        assert!(cw("log_error(e)", "e"));
    }
}
