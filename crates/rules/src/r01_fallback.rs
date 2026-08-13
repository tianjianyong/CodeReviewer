//! R01: 回退掩盖问题检测。
//!
//! Rust: AST 检测 unwrap_or / unwrap_or_default / unwrap_or_else method call。
//! Python: AST 检测 except_clause (bare except 或 except Exception)。
//! TypeScript/C#/Java: AST 检测 catch_clause 内有 return。

use codereviewer_core::ast::{node_text, walk};
use codereviewer_core::finding::{Finding, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct FallbackMasksError;

impl Rule for FallbackMasksError {
    fn id(&self) -> &'static str {
        "R01"
    }
    fn name(&self) -> &'static str {
        "fallback-masks-error"
    }
    fn severity(&self) -> Severity {
        Severity::Error
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
        match ctx.language {
            Language::Rust => find_rust_fallbacks(ctx, &mut findings),
            Language::Python => find_python_fallbacks(ctx, &mut findings),
            Language::TypeScript | Language::TypeScriptTsx | Language::CSharp | Language::Java => {
                find_catch_fallbacks(ctx, &mut findings)
            }
        }
        Ok(findings)
    }
}

fn find_rust_fallbacks(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != "call_expression" {
            return;
        }
        let Some(method_name) = extract_method_name(&node, ctx) else {
            return;
        };
        let message = match method_name {
            "unwrap_or_default" => {
                "unwrap_or_default() 掩盖错误情况 | unwrap_or_default() masks error case"
            }
            "unwrap_or" => "unwrap_or() 掩盖 None/Err 情况 | unwrap_or() masks None/Err case",
            "unwrap_or_else" => {
                "unwrap_or_else() 可能掩盖错误情况 | unwrap_or_else() may mask error case"
            }
            _ => return,
        };
        push_finding(findings, ctx, &node, Severity::Error, message);
    });
}

fn extract_method_name<'a>(node: &tree_sitter::Node, ctx: &AnalysisContext<'a>) -> Option<&'a str> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "field_expression" {
            let mut inner = child.walk();
            for c in child.children(&mut inner) {
                if c.kind() == "field_identifier" {
                    return Some(node_text(&c, ctx.source));
                }
            }
        }
    }
    None
}

fn find_python_fallbacks(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() == "except_clause" {
            let text = node_text(&node, ctx.source);
            let is_bare = text.trim_start().starts_with("except:");
            let is_broad =
                text.contains("except Exception:") || text.contains("except BaseException:");
            if is_bare || is_broad {
                push_finding(
                    findings,
                    ctx,
                    &node,
                    Severity::Error,
                    "裸/宽泛的 except 吞掉错误 | bare/broad except masks errors",
                );
            }
        }
    });
}

fn find_catch_fallbacks(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() == "catch_clause" && has_return_in_subtree(&node) {
            let text = node_text(&node, ctx.source);
            // 有日志的 catch+return 是防御式处理，降为 info；无日志才是静默吞错误
            let (severity, msg) = if has_logging(text) {
                (
                    Severity::Info,
                    "catch 有日志但以默认返回值掩盖异常（防御式处理） | catch logs but masks exception with default return (defensive handling)",
                )
            } else {
                (
                    Severity::Error,
                    "catch 以默认返回值掩盖异常 | catch with default return masks exception",
                )
            };
            push_finding(findings, ctx, &node, severity, msg);
        }
    });
}

/// catch 体内是否有日志调用（C#/TS/Java 常见日志框架）。
fn has_logging(text: &str) -> bool {
    [
        "LogManager",
        "Logger",
        "logger",
        "ILog",
        "log4net",
        "NLog",
        "Serilog",
        "Log.",
        "Log(",
        "log.",
        "log(",
        "Debug.",
        "Debug.WriteLine",
        "Debug.Print",
        "Trace.",
        "Trace.WriteLine",
        "Console.Error",
        "Console.WriteLine",
        "Console.Write",
        ".Error(",
        ".Warn(",
        ".Warning(",
        ".Fatal(",
        ".Debug(",
        ".Info(",
    ]
    .iter()
    .any(|s| text.contains(s))
}

fn has_return_in_subtree(node: &tree_sitter::Node) -> bool {
    let mut stack = vec![*node];
    while let Some(n) = stack.pop() {
        if n.kind() == "return_statement" || n.kind() == "return" {
            return true;
        }
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push(child);
        }
    }
    false
}

fn push_finding(
    findings: &mut Vec<Finding>,
    ctx: &AnalysisContext,
    node: &tree_sitter::Node,
    severity: Severity,
    message: &str,
) {
    findings.push(Finding::new(
        "R01",
        "fallback-masks-error",
        severity,
        ctx.file_path,
        node,
        ctx.source,
        message.to_string(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::analyze_source;

    #[test]
    fn catch_with_logging_is_info() {
        let source = "public class A { public int M() { try { return 1; } catch (Exception ex) { LogManager.Error(ex); return 0; } } }";
        let findings = analyze_source(&FallbackMasksError, source, Language::CSharp);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[test]
    fn catch_without_logging_is_error() {
        let source = "public class A { public int M() { try { return 1; } catch (Exception ex) { return 0; } } }";
        let findings = analyze_source(&FallbackMasksError, source, Language::CSharp);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }
}
