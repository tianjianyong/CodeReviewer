//! R03: 文档/注释缺失检测。
//!
//! 检测 public item（pub 关键字）无 doc comment。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct MissingDoc;

impl Rule for MissingDoc {
    fn id(&self) -> &'static str {
        "R03"
    }
    fn name(&self) -> &'static str {
        "missing-doc"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
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
        let public_kinds = public_kinds(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if public_kinds.contains(&node.kind()) && is_public(&node, ctx) {
                if !has_doc_comment(&node, ctx) {
                    let pos = node.start_position();
                    findings.push(Finding {
                        rule_id: "R03",
                        rule_name: "missing-doc",
                        severity: Severity::Warning,
                        location: Location {
                            file: ctx.file_path.to_path_buf(),
                            line: pos.row + 1,
                            column: pos.column + 1,
                        },
                        message: format!("公开项缺少文档注释：{} | public item without doc comment: {}", node.kind(), node.kind()),
                        snippet: None,
                    });
                }
            }
        });

        Ok(findings)
    }
}

fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    source.get(node.start_byte()..node.end_byte()).unwrap_or("")
}

fn public_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &[
            "function_item",
            "struct_item",
            "enum_item",
            "trait_item",
            "const_item",
            "static_item",
            "type_item",
        ],
        Language::Python => &["function_definition", "class_definition"],
        Language::TypeScript | Language::TypeScriptTsx => &[
            "function_declaration",
            "class_declaration",
            "method_definition",
        ],
        Language::CSharp => &["method_declaration", "class_declaration"],
        Language::Java => &["method_declaration", "class_declaration"],
    }
}

fn is_public(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    match ctx.language {
        Language::Rust => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "visibility_modifier" {
                    return true;
                }
            }
            false
        }
        Language::Python => {
            let pos = node.start_position();
            pos.column == 0
        }
        Language::TypeScript | Language::TypeScriptTsx => {
            let text = node_text(node, ctx.source);
            text.contains("export")
        }
        Language::CSharp | Language::Java => {
            let text = node_text(node, ctx.source);
            text.contains("public")
        }
    }
}

fn has_doc_comment(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        let kind = sibling.kind();
        if kind == "line_comment" || kind == "block_comment" || kind == "documentation" {
            let text = node_text(&sibling, ctx.source);
            if text.starts_with("///")
                || text.starts_with("/**")
                || text.starts_with("//!")
                || text.starts_with("\"\"\"")
                || text.starts_with("/**")
            {
                return true;
            }
            prev = sibling.prev_sibling();
        } else if kind == "attribute_item" || kind == "meta" || kind == "decorator" {
            prev = sibling.prev_sibling();
        } else {
            break;
        }
    }
    false
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
