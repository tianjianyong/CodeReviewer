//! R04: 单元测试简单检测。
//!
//! 启发式：测试函数中断言密度低、只覆盖 happy path。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct SimpleUnitTest;

impl Rule for SimpleUnitTest {
    fn id(&self) -> &'static str {
        "R04"
    }
    fn name(&self) -> &'static str {
        "simple-unit-test"
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
        let min_asserts = ctx.rule_config.threshold_i64("min_assertions", 2) as usize;
        let function_kinds = function_kinds(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if function_kinds.contains(&node.kind()) && is_test_function(&node, ctx) {
                let name = extract_function_name(&node, ctx);
                let assert_count = count_assertions(&node, ctx);
                if assert_count < min_asserts {
                    let pos = node.start_position();
                    findings.push(Finding {
                        rule_id: "R04",
                        rule_name: "simple-unit-test",
                        severity: Severity::Warning,
                        location: Location {
                            file: ctx.file_path.to_path_buf(),
                            line: pos.row + 1,
                            column: pos.column + 1,
                        },
                        message: format!(
                            "test '{}' has only {} assertion(s) (min {})",
                            name, assert_count, min_asserts
                        ),
                        snippet: None,
                    });
                }
            }
        });

        Ok(findings)
    }
}

fn function_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["function_item"],
        Language::Python => &["function_definition"],
        Language::TypeScript | Language::TypeScriptTsx => &[
            "function_declaration",
            "method_definition",
            "call_expression",
        ],
        Language::CSharp => &["method_declaration"],
        Language::Java => &["method_declaration"],
    }
}

fn is_test_function(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    match ctx.language {
        Language::Rust => {
            if has_test_attribute_as_child(node, ctx) {
                return true;
            }
            if has_test_attribute_as_prev_sibling(node, ctx) {
                return true;
            }
            false
        }
        Language::Python => extract_function_name(node, ctx).starts_with("test_"),
        Language::TypeScript | Language::TypeScriptTsx => {
            let text = node_text(node, ctx.source);
            text.contains("test(") || text.contains("it(")
        }
        Language::CSharp | Language::Java => {
            let text = node_text(node, ctx.source);
            text.contains("@Test") || text.contains("[Test]") || text.contains("[TestMethod]")
        }
    }
}

fn has_test_attribute_as_child(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            let text = node_text(&child, ctx.source);
            if text.contains("#[test]") || text.contains("#[tokio::test]") {
                return true;
            }
        }
    }
    false
}

fn has_test_attribute_as_prev_sibling(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        let kind = sibling.kind();
        if kind == "attribute_item" {
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

fn extract_function_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return node_text(&child, ctx.source).to_string();
        }
    }
    String::new()
}

fn count_assertions(node: &tree_sitter::Node, ctx: &AnalysisContext) -> usize {
    let assert_keywords: &[&str] = match ctx.language {
        Language::Rust => &["assert!", "assert_eq!", "assert_ne!", "panic!"],
        Language::Python => &["assert ", "self.assert"],
        Language::TypeScript | Language::TypeScriptTsx => &["expect(", "assert."],
        Language::CSharp => &["Assert."],
        Language::Java => &["assertEquals", "assertTrue", "assertFalse", "assertThrows"],
    };
    let text = node_text(node, ctx.source);
    assert_keywords.iter().map(|k| text.matches(k).count()).sum()
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
