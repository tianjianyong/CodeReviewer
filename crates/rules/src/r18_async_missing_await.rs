//! R18: async 调用未 await 检测。
//!
//! 启发式：同文件内收集 async fn 名（Python async def / TS async function），
//! 找对这些函数的调用未加 await（Python）或未加 await/未链 .then/.catch（TS）。
//! 仅单文件内分析，无跨文件类型信息，漏报外部 async 调用。

use std::collections::HashSet;

use codereviewer_core::ast::{node_text, walk};
use codereviewer_core::finding::{Finding, Severity};
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
        // C#/Java 的 async Task 方法缺少 await 需跨方法返回类型分析，暂不支持
        &[Language::Python, Language::TypeScript, Language::TypeScriptTsx]
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
    let async_fns = collect_async_fn_names(ctx);
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
        findings.push(Finding::new(
            "R18",
            "async-missing-await",
            Severity::Error,
            ctx.file_path,
            &node,
            ctx.source,
            format!(
                "async 函数 {} 的调用未加 await，返回 coroutine 而非结果 | call to async function {} without await, returns coroutine instead of result",
                fname, fname
            ),
        ));
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
        findings.push(Finding::new(
            "R18",
            "async-missing-await",
            Severity::Error,
            ctx.file_path,
            &node,
            ctx.source,
            format!(
                "async 函数 {} 的调用未加 await，返回 Promise 而非结果 | call to async function {} without await, returns Promise instead of result",
                callee, callee
            ),
        ));
    });
}

fn collect_async_fn_names(ctx: &AnalysisContext) -> HashSet<String> {
    let mut names = HashSet::new();
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != "function_definition" {
            return;
        }
        // Python: async def foo  →  function_definition 前有 'async' 关键字
        let text = node_text(&node, ctx.source);
        if (text.starts_with("async def") || text.starts_with("asyncdef"))
            && let Some(name) = extract_fn_name(&node, ctx) {
                names.insert(name);
            }
    });
    names
}

fn collect_ts_async_names(ctx: &AnalysisContext) -> HashSet<String> {
    let mut names = HashSet::new();
    walk(ctx.tree.root_node(), &mut |node| {
        let text = node_text(&node, ctx.source);
        // async function foo( / async foo( / const foo = async (
        if node.kind() == "function_declaration" && text.starts_with("async function")
            && let Some(name) = extract_fn_name(&node, ctx) {
                names.insert(name);
            }
        if (node.kind() == "variable_declaration" || node.kind() == "lexical_declaration")
            && text.contains("async") {
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
