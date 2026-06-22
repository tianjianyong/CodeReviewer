//! R16: 自验证测试检测。
//!
//! 启发式：测试函数名只描述方法不描述行为（test_foo vs test_foo_when_bar），
//! 或测试访问被测单元的私有成员（_foo/__foo/#foo）。
//! 与 R04/R05 互补——断言够多但仍只测了实现视角。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct SelfValidatingTest;

impl Rule for SelfValidatingTest {
    fn id(&self) -> &'static str {
        "R16"
    }
    fn name(&self) -> &'static str {
        "self-validating-test"
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
        let function_kinds = function_kinds(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if !function_kinds.contains(&node.kind()) {
                return;
            }
            if !is_test_function(&node, ctx) {
                return;
            }
            let name = extract_function_name(&node, ctx);
            if has_vague_name(&name) {
                let pos = node.start_position();
                findings.push(Finding {
                    rule_id: "R16",
                    rule_name: "self-validating-test",
                    severity: Severity::Warning,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                    },
                    message: format!(
                        "测试名 '{}' 只描述方法未描述行为（建议加 _when_/_should_ 等行为后缀） | test '{}' name describes method not behavior (add _when_/_should_ suffix)",
                        name, name
                    ),
                    snippet: None,
                });
                return;
            }
            if let Some(member) = first_private_member_access(&node, ctx) {
                let pos = node.start_position();
                findings.push(Finding {
                    rule_id: "R16",
                    rule_name: "self-validating-test",
                    severity: Severity::Warning,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                    },
                    message: format!(
                        "测试访问被测单元的私有成员 '{}'，可能只验证实现内部状态 | test accesses private member '{}' of unit under test",
                        member, member
                    ),
                    snippet: None,
                });
            }
        });

        Ok(findings)
    }
}

/// 测试名只描述方法不描述行为：test_foo / testFoo / fooTest 无行为后缀。
fn has_vague_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    if !lower.starts_with("test") {
        return false;
    }
    // 行为后缀信号词（含动词与条件状语）
    let behavior_signals = [
        "_when", "_if", "_given", "_should", "_with", "_for", "_on_",
        "_returns", "_throws", "_fails", "_succeeds", "_is", "_has",
        "_finds", "_skips", "_returns", "_has", "_does", "_not_",
        "_flagged", "_count", "_all", "_multiple", "_errors",
        "when", "should", "given", "if_",
    ];
    !behavior_signals.iter().any(|s| lower.contains(s))
}

fn first_private_member_access(node: &tree_sitter::Node, ctx: &AnalysisContext) -> Option<String> {
    let mut found: Option<String> = None;
    walk(*node, &mut |n| {
        if found.is_some() {
            return;
        }
        // Python: self._foo  /  obj.__foo
        // Rust:   self._foo  (convention)
        // TS/JS:  this._foo  /  obj.#foo
        let text = node_text(&n, ctx.source);
        if let Some(rest) = text
            .strip_prefix("self._")
            .or_else(|| text.strip_prefix("self.__"))
            .or_else(|| text.strip_prefix("this._"))
            .or_else(|| text.strip_prefix("this.__"))
        {
            let member = rest.split([' ', '.', ';', ',', '(', ')']).next().unwrap_or("");
            if !member.is_empty() && member.chars().all(|c| c.is_alphanumeric() || c == '_') {
                found = Some(member.to_string());
            }
        }
    });
    found
}

fn function_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["function_item"],
        Language::Python => &["function_definition"],
        Language::TypeScript | Language::TypeScriptTsx => &["function_declaration", "method_definition"],
        Language::CSharp => &["method_declaration"],
        Language::Java => &["method_declaration"],
    }
}

fn is_test_function(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    match ctx.language {
        Language::Rust => {
            // #[test] 属性
            let mut prev = node.prev_sibling();
            while let Some(sibling) = prev {
                if sibling.kind() == "attribute_item" {
                    let text = node_text(&sibling, ctx.source);
                    if text.contains("#[test]") || text.contains("#[tokio::test]") {
                        return true;
                    }
                    prev = sibling.prev_sibling();
                } else {
                    break;
                }
            }
            false
        }
        Language::Python => extract_function_name(node, ctx).starts_with("test_"),
        Language::TypeScript | Language::TypeScriptTsx => {
            let name = extract_function_name(node, ctx);
            name.starts_with("test") || name.starts_with("it_") || name == "it"
        }
        Language::CSharp | Language::Java => {
            let text = node_text(node, ctx.source);
            text.contains("@Test") || text.contains("[Test]") || text.contains("[TestMethod]")
        }
    }
}

fn extract_function_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return node_text(&child, ctx.source).to_string();
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
