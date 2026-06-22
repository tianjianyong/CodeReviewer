//! R01: 回退掩盖问题检测。
//!
//! Rust: AST 检测 unwrap_or / unwrap_or_default / unwrap_or_else method call。
//! Python: AST 检测 except_clause (bare except 或 except Exception)。
//! TypeScript/C#/Java: AST 检测 catch_clause 内有 return。

use codereviewer_core::finding::{Finding, Location, Severity};
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
            Language::TypeScript | Language::TypeScriptTsx => find_catch_fallbacks(ctx, &mut findings, "catch 以默认返回值掩盖错误 | catch with default return masks error"),
            Language::CSharp | Language::Java => find_catch_fallbacks(ctx, &mut findings, "catch 以默认返回值掩盖异常 | catch with default return masks exception"),
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
            "unwrap_or_default" => "unwrap_or_default() 掩盖错误情况 | unwrap_or_default() masks error case",
            "unwrap_or" => "unwrap_or() 掩盖 None/Err 情况 | unwrap_or() masks None/Err case",
            "unwrap_or_else" => "unwrap_or_else() 可能掩盖错误情况 | unwrap_or_else() may mask error case",
            _ => return,
        };
        push_finding(findings, ctx, &node, message);
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
            let is_broad = text.contains("except Exception:") || text.contains("except BaseException:");
            if is_bare || is_broad {
                push_finding(findings, ctx, &node, "裸/宽泛的 except 吞掉错误 | bare/broad except masks errors");
            }
        }
    });
}

fn find_catch_fallbacks(ctx: &AnalysisContext, findings: &mut Vec<Finding>, message: &str) {
    let catch_kind = match ctx.language {
        Language::TypeScript | Language::TypeScriptTsx => "catch_clause",
        Language::CSharp => "catch_clause",
        Language::Java => "catch_clause",
        _ => return,
    };
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() == catch_kind && has_return_in_subtree(&node) {
            push_finding(findings, ctx, &node, message);
        }
    });
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

fn push_finding(findings: &mut Vec<Finding>, ctx: &AnalysisContext, node: &tree_sitter::Node, message: &str) {
    let pos = node.start_position();
    findings.push(Finding {
        rule_id: "R01",
        rule_name: "fallback-masks-error",
        severity: Severity::Error,
        location: Location {
            file: ctx.file_path.to_path_buf(),
            line: pos.row + 1,
            column: pos.column + 1,
        },
        message: message.to_string(),
        snippet: None,
    });
}

fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    source.get(node.start_byte()..node.end_byte()).unwrap_or("")
}

fn walk<F: FnMut(tree_sitter::Node)>(node: tree_sitter::Node, visit: &mut F) {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        visit(n);
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push(child);
        }
    }
}
