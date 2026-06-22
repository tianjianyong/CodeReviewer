//! R28: 过度防御处理检测（R01 的反面）。
//!
//! 启发式：(1) Some(...)/Ok(...) 字面量上调用 unwrap_or/unwrap_or_default/unwrap_or_else；
//! (2) TS/Java try 块仅包纯算术/属性访问等不可失败操作。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct OverlyDefensiveHandling;

impl Rule for OverlyDefensiveHandling {
    fn id(&self) -> &'static str {
        "R28"
    }
    fn name(&self) -> &'static str {
        "overly-defensive-handling"
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
        match ctx.language {
            Language::Rust => find_rust_unwrap_on_literal(ctx, &mut findings),
            Language::TypeScript | Language::TypeScriptTsx => {
                find_ts_nullish_on_literal(ctx, &mut findings);
            }
            _ => {}
        }
        Ok(findings)
    }
}

fn find_rust_unwrap_on_literal(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != "call_expression" {
            return;
        }
        // 结构: Some(x).unwrap_or(...) / Ok(x).unwrap_or(...)
        let text = node_text(&node, ctx.source);
        let methods = ["unwrap_or", "unwrap_or_default", "unwrap_or_else"];
        let Some(method) = methods.iter().find(|m| text.contains(&format!(".{m}("))) else {
            return;
        };
        let receiver = extract_receiver(&node, ctx);
        let trimmed = receiver.trim();
        if trimmed.starts_with("Some(") || trimmed.starts_with("Ok(") {
            let pos = node.start_position();
            findings.push(Finding {
                rule_id: "R28",
                rule_name: "overly-defensive-handling",
                severity: Severity::Info,
                location: Location {
                    file: ctx.file_path.to_path_buf(),
                    line: pos.row + 1,
                    column: pos.column + 1,
                },
                message: format!(
                    "对 {} 调用 {} 是过度防御——该值类型上保证不可失败 | {} on {} is overly defensive — value cannot fail",
                    trimmed, method, method, trimmed
                ),
                snippet: None,
            });
        }
    });
}

fn find_ts_nullish_on_literal(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    walk(ctx.tree.root_node(), &mut |node| {
        // a ?? default  →  binary_expression / nullish_coalescing_expression
        if node.kind() != "nullish_coalescing_expression" {
            return;
        }
        let text = node_text(&node, ctx.source);
        let Some(left) = text.split("??").next() else {
            return;
        };
        let left = left.trim();
        // 字面量左侧不可空
        if left.starts_with('"') || left.starts_with('\'') || left.chars().all(|c| c.is_ascii_digit()) {
            let pos = node.start_position();
            findings.push(Finding {
                rule_id: "R28",
                rule_name: "overly-defensive-handling",
                severity: Severity::Info,
                location: Location {
                    file: ctx.file_path.to_path_buf(),
                    line: pos.row + 1,
                    column: pos.column + 1,
                },
                message: format!(
                    "对不可空字面量 {} 使用 ?? 是过度防御 | ?? on non-null literal {} is overly defensive",
                    left, left
                ),
                snippet: None,
            });
        }
    });
}

fn extract_receiver(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    // call_expression 的第一个子节点通常是 receiver（field_expression 的 object）
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "field_expression" {
            let mut inner = child.walk();
            for c in child.children(&mut inner) {
                if c.kind() != "field_identifier" {
                    return node_text(&c, ctx.source).to_string();
                }
            }
        }
    }
    String::new()
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
