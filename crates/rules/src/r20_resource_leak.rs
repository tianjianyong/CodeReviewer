//! R20: 资源泄漏检测。
//!
//! 启发式：(1) Python open()/connect() 不在 with 且同函数无 .close()；
//! (2) JS/TS 循环内 addEventListener/.on() 无配对 removeEventListener/.off()。
//! Rust 因 RAII 不适用。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct ResourceLeak;

impl Rule for ResourceLeak {
    fn id(&self) -> &'static str {
        "R20"
    }
    fn name(&self) -> &'static str {
        "resource-leak"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn languages(&self) -> &'static [Language] {
        &[
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
            Language::Python => find_python_leaks(ctx, &mut findings),
            Language::TypeScript | Language::TypeScriptTsx => {
                find_ts_listener_in_loop(ctx, &mut findings)
            }
            _ => {}
        }
        Ok(findings)
    }
}

fn find_python_leaks(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    let function_kinds = ["function_definition"];
    walk(ctx.tree.root_node(), &mut |node| {
        if !function_kinds.contains(&node.kind()) {
            return;
        }
        let body = node_text(&node, ctx.source);
        // 找 open( / .connect( 调用
        let has_resource_open = body.contains("open(") || body.contains(".connect(");
        if !has_resource_open {
            return;
        }
        // 是否在 with 语句内（with_statement 包裹）
        let in_with = has_ancestor_kind(&node, "with_statement", ctx)
            || count_in_with(&node, ctx) > 0;
        let has_close = body.contains(".close()");
        if !in_with && !has_close {
            let pos = node.start_position();
            findings.push(Finding {
                rule_id: "R20",
                rule_name: "resource-leak",
                severity: Severity::Warning,
                location: Location {
                    file: ctx.file_path.to_path_buf(),
                    line: pos.row + 1,
                    column: pos.column + 1,
                },
                message: format!(
                    "open()/connect() 未用 with 且无 .close()，疑似资源泄漏 | open()/connect() not in with and no .close(), possible resource leak"
                ),
                snippet: None,
            });
        }
    });
}

fn count_in_with(func_node: &tree_sitter::Node, _ctx: &AnalysisContext) -> usize {
    let mut count = 0;
    walk(*func_node, &mut |n| {
        if n.kind() == "with_statement" {
            count += 1;
        }
    });
    count
}

fn find_ts_listener_in_loop(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    let loop_kinds = ["for_statement", "while_statement", "for_in_statement"];
    walk(ctx.tree.root_node(), &mut |node| {
        if !loop_kinds.contains(&node.kind()) {
            return;
        }
        let body = node_text(&node, ctx.source);
        let has_add = body.contains("addEventListener(") || body.contains(".on(");
        if !has_add {
            return;
        }
        let has_remove = body.contains("removeEventListener(") || body.contains(".off(");
        if !has_remove {
            let pos = node.start_position();
            findings.push(Finding {
                rule_id: "R20",
                rule_name: "resource-leak",
                severity: Severity::Warning,
                location: Location {
                    file: ctx.file_path.to_path_buf(),
                    line: pos.row + 1,
                    column: pos.column + 1,
                },
                message: format!(
                    "循环内 addEventListener/.on() 无配对 removeEventListener/.off()，监听器泄漏 | addEventListener/.on() in loop without removeEventListener/.off(), listener leak"
                ),
                snippet: None,
            });
        }
    });
}

fn has_ancestor_kind(node: &tree_sitter::Node, kind: &str, _ctx: &AnalysisContext) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == kind {
            return true;
        }
        current = parent.parent();
    }
    false
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
