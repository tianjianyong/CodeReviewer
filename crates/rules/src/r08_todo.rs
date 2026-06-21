//! R08: TODO/FIXME 堆积检测。
//!
//! 用 AST 只扫描 comment 节点，不扫标识符或代码。

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
        let comment_kinds = comment_kinds(ctx.language);
        let markers = ["todo", "fixme", "xxx"];
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if !comment_kinds.contains(&node.kind()) {
                return;
            }
            let text = node_text(&node, ctx.source);
            let lower = text.to_lowercase();
            if markers.iter().any(|m| lower.contains(m)) {
                let pos = node.start_position();
                findings.push(Finding {
                    rule_id: "R08",
                    rule_name: "todo-fixme-accumulation",
                    severity: Severity::Info,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                    },
                    message: "TODO/FIXME marker found".to_string(),
                    snippet: Some(text.trim().to_string()),
                });
            }
        });

        Ok(findings)
    }
}

fn comment_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["line_comment", "block_comment", "documentation"],
        Language::Python => &["comment"],
        Language::TypeScript | Language::TypeScriptTsx => &["comment"],
        Language::CSharp => &["comment", "extern_alias_directive"],
        Language::Java => &["comment", "block_comment", "line_comment"],
    }
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
