//! R06: 过度设计启发式检测。
//!
//! 启发式信号：单实现 trait、过度泛型化。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct OverEngineering;

impl Rule for OverEngineering {
    fn id(&self) -> &'static str {
        "R06"
    }
    fn name(&self) -> &'static str {
        "over-engineering"
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
        if ctx.language == Language::Rust {
            findings.extend(detect_single_impl_traits(ctx));
            findings.extend(detect_excess_generics(ctx));
        }
        Ok(findings)
    }
}

struct TraitInfo {
    name: String,
    line: usize,
    column: usize,
}

fn detect_single_impl_traits(ctx: &AnalysisContext) -> Vec<Finding> {
    let mut traits: Vec<TraitInfo> = Vec::new();
    let mut impl_texts: Vec<String> = Vec::new();

    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() == "trait_item" {
            if let Some(name) = extract_trait_name(&node, ctx) {
                let pos = node.start_position();
                traits.push(TraitInfo {
                    name,
                    line: pos.row + 1,
                    column: pos.column + 1,
                });
            }
        }
        if node.kind() == "impl_item" {
            let text = node_text(&node, ctx.source).to_string();
            impl_texts.push(text);
        }
    });

    let mut findings = Vec::new();
    for tr in &traits {
        let impl_count = impl_texts
            .iter()
            .filter(|text| {
                text.contains(&format!("impl {}", tr.name))
                    || text.contains(&format!("impl<{}> {}", tr.name, tr.name))
            })
            .count();
        if impl_count <= 1 {
            findings.push(Finding {
                rule_id: "R06",
                rule_name: "over-engineering",
                severity: Severity::Info,
                location: Location {
                    file: ctx.file_path.to_path_buf(),
                    line: tr.line,
                    column: tr.column,
                },
                message: format!(
                    "trait {} has only {} implementation(s) - consider if abstraction is needed",
                    tr.name, impl_count
                ),
                snippet: None,
            });
        }
    }
    findings
}

fn detect_excess_generics(ctx: &AnalysisContext) -> Vec<Finding> {
    let max_generics = ctx.rule_config.threshold_i64("max_generics", 3) as usize;
    let mut findings = Vec::new();
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() == "type_parameters" {
            let count = node.named_child_count();
            if count > max_generics {
                let pos = node.start_position();
                findings.push(Finding {
                    rule_id: "R06",
                    rule_name: "over-engineering",
                    severity: Severity::Info,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                    },
                    message: format!(
                        "excessive generics: {} type parameters (max {})",
                        count, max_generics
                    ),
                    snippet: None,
                });
            }
        }
    });
    findings
}

fn extract_trait_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return Some(node_text(&child, ctx.source).to_string());
        }
    }
    None
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
