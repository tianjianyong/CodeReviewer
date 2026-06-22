//! R19: N+1 查询检测。
//!
//! 启发式：for 循环遍历 queryset（含 .objects/.filter/.all/.query），
//! 体内访问 x.<关系字段>，而 queryset 源未调用 select_related/prefetch_related/include/with。
//! 无 schema 信息，靠信号词启发式，可能误报普通循环。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct NPlusOneQuery;

impl Rule for NPlusOneQuery {
    fn id(&self) -> &'static str {
        "R19"
    }
    fn name(&self) -> &'static str {
        "n-plus-one-query"
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
            Language::Python => find_python_nplus1(ctx, &mut findings),
            Language::TypeScript | Language::TypeScriptTsx => {
                find_ts_nplus1(ctx, &mut findings)
            }
            _ => {}
        }
        Ok(findings)
    }
}

fn find_python_nplus1(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != "for_statement" {
            return;
        }
        // for x in <source>:  source 是 queryset
        let source = extract_for_source(&node, ctx);
        if !is_queryset_source(&source) {
            return;
        }
        // 源已用 prefetch
        if has_prefetch(&source) {
            return;
        }
        // 体内访问 x.<field>（属性访问）
        let iter_var = extract_for_var(&node, ctx);
        if iter_var.is_empty() {
            return;
        }
        if accesses_relation(&node, &iter_var, ctx) {
            let pos = node.start_position();
            findings.push(Finding {
                rule_id: "R19",
                rule_name: "n-plus-one-query",
                severity: Severity::Warning,
                location: Location {
                    file: ctx.file_path.to_path_buf(),
                    line: pos.row + 1,
                    column: pos.column + 1,
                },
                message: format!(
                    "循环遍历 queryset 访问关系字段可能触发 N+1 查询，建议用 select_related/prefetch_related | iterating queryset and accessing relation may cause N+1 query, use select_related/prefetch_related"
                ),
                snippet: None,
            });
        }
    });
}

fn find_ts_nplus1(ctx: &AnalysisContext, findings: &mut Vec<Finding>) {
    // .map(x => x.<field>) / .forEach over query result
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() != "call_expression" {
            return;
        }
        let text = node_text(&node, ctx.source);
        if !text.contains(".map(") && !text.contains(".forEach(") {
            return;
        }
        // 源是 DB 查询结果
        if !is_ts_query_source(&text) {
            return;
        }
        if has_ts_prefetch(&text) {
            return;
        }
        // 箭头函数体内访问 x.<field>
        if text.contains("=> x.") || text.contains("=> item.") || text.contains("=> row.") {
            let pos = node.start_position();
            findings.push(Finding {
                rule_id: "R19",
                rule_name: "n-plus-one-query",
                severity: Severity::Warning,
                location: Location {
                    file: ctx.file_path.to_path_buf(),
                    line: pos.row + 1,
                    column: pos.column + 1,
                },
                message: format!(
                    ".map/.forEach 遍历查询结果访问关系字段可能触发 N+1 查询，建议用 include/with | .map/.forEach over query result accessing relation may cause N+1 query, use include/with"
                ),
                snippet: None,
            });
        }
    });
}

fn is_queryset_source(source: &str) -> bool {
    let signals = [".objects", ".filter(", ".all()", ".query", ".exclude(", ".order_by("];
    signals.iter().any(|s| source.contains(s))
}

fn is_ts_query_source(text: &str) -> bool {
    let signals = [
        ".findMany(", ".find(", "prisma.", ".query(", "from(\"", "Model.objects",
        "db.collection", ".findAll(",
    ];
    signals.iter().any(|s| text.contains(s))
}

fn has_prefetch(source: &str) -> bool {
    let signals = ["select_related", "prefetch_related"];
    signals.iter().any(|s| source.contains(s))
}

fn has_ts_prefetch(text: &str) -> bool {
    let signals = [".include(", ".with(", "eager_load", ".joins("];
    signals.iter().any(|s| text.contains(s))
}

fn extract_for_source(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call" || child.kind() == "identifier" || child.kind() == "attribute" {
            return node_text(&child, ctx.source).to_string();
        }
    }
    String::new()
}

fn extract_for_var(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return node_text(&child, ctx.source).to_string();
        }
    }
    String::new()
}

fn accesses_relation(func_node: &tree_sitter::Node, iter_var: &str, ctx: &AnalysisContext) -> bool {
    let prefix = format!("{}.", iter_var);
    let mut found = false;
    walk(*func_node, &mut |n| {
        if found {
            return;
        }
        let text = node_text(&n, ctx.source);
        // x.author / x.user.profile
        if text.starts_with(&prefix) {
            // 排除 x.save() / x.delete() / x.id / x.pk 等内置
            let rest = &text[prefix.len()..];
            let field = rest.split(['(', '.', ';', ' ']).next().unwrap_or("");
            if !matches!(field, "id" | "pk" | "save" | "delete" | "objects" | "exists" | "count") {
                found = true;
            }
        }
    });
    found
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
