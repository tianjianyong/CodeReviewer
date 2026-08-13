//! R06: 过度设计启发式检测。
//!
//! 启发式信号：单实现 trait、过度泛型化。

use codereviewer_core::ast::{node_text, walk};
use codereviewer_core::finding::{Finding, Severity};
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

struct TraitInfo<'a> {
    name: String,
    node: tree_sitter::Node<'a>,
}

fn detect_single_impl_traits<'a>(ctx: &'a AnalysisContext<'a>) -> Vec<Finding> {
    let mut traits: Vec<TraitInfo<'a>> = Vec::new();
    let mut impl_names: Vec<String> = Vec::new();

    // 内联栈遍历：Node<'a> 无法安全地经 FnMut 闭包存入外部 Vec（&mut 不变性限制）
    let mut stack = vec![ctx.tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "trait_item" {
            if let Some(name) = extract_trait_name(&node, ctx) {
                traits.push(TraitInfo { name, node });
            }
        }
        if node.kind() == "impl_item" {
            if let Some(name) = extract_impl_trait_name(&node, ctx) {
                impl_names.push(name);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    let mut findings = Vec::new();
    for tr in &traits {
        let impl_count = impl_names.iter().filter(|n| *n == &tr.name).count();
        if impl_count <= 1 {
            findings.push(Finding::new(
                "R06",
                "over-engineering",
                Severity::Info,
                ctx.file_path,
                &tr.node,
                ctx.source,
                format!(
                    "trait {} 仅有 {} 处实现——请考虑是否需要此抽象 | trait {} has only {} implementation(s) - consider if abstraction is needed",
                    tr.name, impl_count, tr.name, impl_count
                ),
            ));
        }
    }
    findings
}

/// 提取 impl 块的 trait 名：`impl Trait for X` 中 for 之前的 type_identifier；
/// 无 for 的是固有 impl（impl Struct），不属于 trait 实现，跳过。
fn extract_impl_trait_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> Option<String> {
    let mut cursor = node.walk();
    let children: Vec<tree_sitter::Node> = node.children(&mut cursor).collect();
    let for_pos = children.iter().position(|c| c.kind() == "for")?;
    children[..for_pos].iter().rev().find_map(|c| {
        if c.kind() == "type_identifier" || c.kind() == "scoped_type_identifier" {
            let text = node_text(c, ctx.source);
            // scoped 名取最后一段：std::io::Read -> Read
            Some(text.rsplit("::").next().unwrap_or(text).to_string())
        } else {
            None
        }
    })
}

fn detect_excess_generics(ctx: &AnalysisContext) -> Vec<Finding> {
    let max_generics = ctx.rule_config.threshold_i64("max_generics", 3) as usize;
    let mut findings = Vec::new();
    walk(ctx.tree.root_node(), &mut |node| {
        if node.kind() == "type_parameters" {
            let count = node.named_child_count();
            if count > max_generics {
                findings.push(Finding::new(
                    "R06",
                    "over-engineering",
                    Severity::Info,
                    ctx.file_path,
                    &node,
                    ctx.source,
                    format!(
                        "泛型过多：{} 个类型参数（上限 {}） | excessive generics: {} type parameters (max {})",
                        count, max_generics, count, max_generics
                    ),
                ));
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
