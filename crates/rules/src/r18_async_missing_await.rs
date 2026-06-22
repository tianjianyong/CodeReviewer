//! R18: async 调用未 await 检测。
//!
//! 启发式：同文件内收集 async fn 名（Python async def / TS async function），
//! 找对这些函数的调用未加 await（Python）或未加 await/未链 .then/.catch（TS）。
//! 仅单文件内分析，无跨文件类型信息，漏报外部 async 调用。

use std::collections::HashSet;

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct AsyncMissingAwait;

impl Rule for AsyncMissingAwait {
    fn id(&self) -> &'static str {
        "R18"
    }
    fn name(&self) -> &'static str {
        "async-missing-await"
    }
    fn severity(&self) -> Severity {
        Severity::Error
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
            Language::Python => find_python(ctx, &mut findings),
            Language::TypeScript | Language::TypeScriptTsx => find_ts(ctx, &mut findings),
            _ => {}
        }
        Ok(findings)
    }
}

fn find_python(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    let async_fns = collect_async_fn_names(ctx, "async", "function_definition");
    if async_fns.is_empty() {
        return;
    }
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != "call" {
            return;
        }
        let name = node_text(&node, ctx.source);
        let fname = name.split('(').next().unwrap_or("");
        if !async_fns.contains(fname) {
            return;
        }
        // 排除定义本身
        if is_definition_site(&node, ctx) {
            return;
        }
        // 检查是否被 await 包裹
        if is_awaited(&node, ctx) {
            return;
        }
        let pos = node.start_position();
        findings.push(Finding {
            rule_id: "R18",
            rule_name: "async-missing-await",
            severity: Severity::Error,
            location: Location {
                file: ctx.file_path.to_path_buf(),
                line: pos.row + 1,
                column: pos.column + 1,
            },
            message: format!(
                "async 函数 {} 的调用未加 await，返回 coroutine 而非结果 | call to async function {} without await, returns coroutine instead of result",
                fname, fname
            ),
            snippet: None,
        });
    });
}

fn find_ts(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    let async_fns = collect_ts_async_names(ctx);
    if async_fns.is_empty() {
        return;
    }
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != "call_expression" {
            return;
        }
        let callee = ts_callee_name(&node, ctx);
        if callee.is_empty() || !async_fns.contains(callee.as_str()) {
            return;
        }
        if is_awaited(&node, ctx) {
            return;
        }
        let text = node_text(&node, ctx.source);
        // .then/.catch/.finally 链式也算处理
        let parent_text = node
            .parent()
            .map(|p| node_text(&p, ctx.source).to_string())
            .unwrap_or_default();
        if parent_text.contains(".then(")
            || parent_text.contains(".catch(")
            || parent_text.contains(".finally(")
        {
            return;
        }
        let _ = text;
        let pos = node.start_position();
        findings.push(Finding {
            rule_id: "R18",
            rule_name: "async-missing-await",
            severity: Severity::Error,
            location: Location {
                file: ctx.file_path.to_path_buf(),
                line: pos.row + 1,
                column: pos.column + 1,
            },
            message: format!(
                "async 函数 {} 的调用未加 await，返回 Promise 而非结果 | call to async function {} without await, returns Promise instead of result",
                callee, callee
            ),
            snippet: None,
        });
    });
}

fn collect_async_fn_names(ctx: &AnalysisContext, _kw: &str, fn_kind: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != fn_kind {
            return;
        }
        // Python: async def foo  →  function_definition 前有 'async' 关键字
        let text = node_text(&node, ctx.source);
        if text.starts_with("async def") || text.starts_with("asyncdef") {
            if let Some(name) = extract_fn_name(&node, ctx) {
                names.insert(name);
            }
        }
    });
    names
}

fn collect_ts_async_names(ctx: &AnalysisContext) -> HashSet<String> {
    let mut names = HashSet::new();
    walk(ctx.tree.root_node(), &mut |node| {
        let text = node_text(&node, ctx.source);
        // async function foo( / async foo( / const foo = async (
        if node.kind() == "function_declaration" && text.starts_with("async function") {
            if let Some(name) = extract_fn_name(&node, ctx) {
                names.insert(name);
            }
        }
        if node.kind() == "variable_declaration" || node.kind() == "lexical_declaration" {
            if text.contains("async") {
                // const foo = async () =>  → 找 identifier
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declarator" {
                        let mut inner = child.walk();
                        for c in child.children(&mut inner) {
                            if c.kind() == "identifier" {
                                names.insert(node_text(&c, ctx.source).to_string());
                            }
                        }
                    }
                }
            }
        }
    });
    names
}

fn extract_fn_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(node_text(&child, ctx.source).to_string());
        }
    }
    None
}

fn ts_callee_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return node_text(&child, ctx.source).to_string();
        }
    }
    String::new()
}

fn is_definition_site(node: &tree_sitter::Node, _ctx: &AnalysisContext) -> bool {
    // 调用节点的父节点是 function_definition → 是定义内的递归？保守跳过 def 内首行
    let Some(parent) = node.parent() else {
        return false;
    };
    matches!(parent.kind(), "function_definition" | "function_declaration") && {
        // 如果是函数体内首个 identifier（def 名），跳过
        node.start_byte() == parent.start_byte()
    }
}

fn is_awaited(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    // Python: parent 是 await 节点；TS: parent 含 await 关键字前缀
    let Some(parent) = node.parent() else {
        return false;
    };
    let parent_text = node_text(&parent, ctx.source);
    parent_text.starts_with("await ")
        || parent_text.starts_with("await(")
        || parent_text.starts_with("await\t")
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
