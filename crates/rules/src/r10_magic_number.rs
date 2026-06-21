//! R10: 魔法数字/字符串检测。
//!
//! 检测未命名的数字/字符串字面量，排除常见值（0, 1, -1, 空字符串）。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct MagicNumber;

impl Rule for MagicNumber {
    fn id(&self) -> &'static str {
        "R10"
    }
    fn name(&self) -> &'static str {
        "magic-number"
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
        let literal_kinds = literal_kinds(ctx.language);
        let mut findings = Vec::new();

        walk_nodes(ctx.tree.root_node(), &mut |node| {
            if literal_kinds.contains(&node.kind()) {
                let text = node.text_for(ctx).unwrap_or_default();
                if is_magic(node.kind(), &text) {
                    let pos = node.start_position();
                    findings.push(Finding {
                        rule_id: "R10",
                        rule_name: "magic-number",
                        severity: Severity::Info,
                        location: Location {
                            file: ctx.file_path.to_path_buf(),
                            line: pos.row + 1,
                            column: pos.column + 1,
                        },
                        message: format!("magic literal: {}", text),
                        snippet: None,
                    });
                }
            }
        });

        Ok(findings)
    }
}

trait NodeExt {
    fn text_for(&self, ctx: &AnalysisContext) -> Option<String>;
}

impl NodeExt for tree_sitter::Node<'_> {
    fn text_for(&self, ctx: &AnalysisContext) -> Option<String> {
        let start = self.start_byte();
        let end = self.end_byte();
        ctx.source.get(start..end).map(|s| s.to_string())
    }
}

fn literal_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["integer_literal", "float_literal", "string_literal"],
        Language::Python => &["integer", "float", "string"],
        Language::TypeScript | Language::TypeScriptTsx => &[
            "number",
            "string",
            "template_string",
        ],
        Language::CSharp => &["integer_literal", "real_literal", "string_literal"],
        Language::Java => &[
            "decimal_integer_literal",
            "decimal_floating_point_literal",
            "string_literal",
        ],
    }
}

fn is_magic(kind: &str, text: &str) -> bool {
    if kind.contains("string") {
        return !text.is_empty() && text != "\"\"" && text != "''" && text != "\" \"" && text.len() > 5;
    }
    let n: i64 = text.trim().parse().unwrap_or(0);
    !matches!(n, 0 | 1 | -1 | 2)
}

fn walk_nodes<F: FnMut(tree_sitter::Node)>(node: tree_sitter::Node, visit: &mut F) {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        visit(n);
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push(child);
        }
    }
}
