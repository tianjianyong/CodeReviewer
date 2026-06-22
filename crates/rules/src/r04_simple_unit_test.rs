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
                            "测试 '{}' 仅有 {} 条断言（下限 {}） | test '{}' has only {} assertion(s) (min {})",
                            name, assert_count, min_asserts, name, assert_count, min_asserts
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
        // TS/TSX: test/it 是 call_expression，不是 function_declaration
        Language::TypeScript | Language::TypeScriptTsx => &["call_expression"],
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
        // TS/TSX: 必须是 test(...)/it(...) 调用，callee 是 identifier
        Language::TypeScript | Language::TypeScriptTsx => is_test_or_it_call(node, ctx),
        Language::CSharp | Language::Java => {
            let text = node_text(node, ctx.source);
            text.contains("@Test") || text.contains("[Test]") || text.contains("[TestMethod]")
        }
    }
}

/// TS: 检查 call_expression 的 callee 是否为 identifier `test` 或 `it`
fn is_test_or_it_call(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(callee) = callee_identifier(node, ctx) else {
        return false;
    };
    matches!(callee, "test" | "it")
}

/// 提取 call_expression 的 callee identifier 名字（支持 `test(...)` 和 `describe.each(...)` 等链式调用的最内层）
fn callee_identifier<'a>(node: &tree_sitter::Node, ctx: &AnalysisContext<'a>) -> Option<&'a str> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(node_text(&child, ctx.source));
        }
        // 链式调用 test.only(...) / test.skip(...) 等：callee 是 member_expression
        if child.kind() == "member_expression" {
            let mut inner = child.walk();
            for c in child.children(&mut inner) {
                if c.kind() == "identifier" {
                    let name = node_text(&c, ctx.source);
                    // 只认最左侧 identifier 是 test/it
                    if matches!(name, "test" | "it") {
                        return Some(name);
                    }
                }
            }
        }
    }
    None
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
    // TS call_expression: 名字是 callee（如 test/it）
    if ctx.language == Language::TypeScript || ctx.language == Language::TypeScriptTsx {
        if node.kind() == "call_expression" {
            return callee_identifier(node, ctx).unwrap_or("").to_string();
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return node_text(&child, ctx.source).to_string();
        }
    }
    String::new()
}

fn count_assertions(node: &tree_sitter::Node, ctx: &AnalysisContext) -> usize {
    // TS/TSX: AST 数 expect(...) call_expression
    if ctx.language == Language::TypeScript || ctx.language == Language::TypeScriptTsx {
        return count_expect_calls(node, ctx);
    }
    let assert_keywords: &[&str] = match ctx.language {
        Language::Rust => &["assert!", "assert_eq!", "assert_ne!", "panic!"],
        Language::Python => &["assert ", "self.assert"],
        Language::CSharp => &["Assert."],
        Language::Java => &["assertEquals", "assertTrue", "assertFalse", "assertThrows"],
        _ => &[],
    };
    let text = node_text(node, ctx.source);
    assert_keywords.iter().map(|k| text.matches(k).count()).sum()
}

/// TS: 统计子树内 expect(...) call_expression 数量
fn count_expect_calls(node: &tree_sitter::Node, ctx: &AnalysisContext) -> usize {
    let mut count = 0;
    walk(*node, &mut |n| {
        if n.kind() == "call_expression" {
            if let Some(callee) = callee_identifier(&n, ctx) {
                if matches!(callee, "expect" | "assert") {
                    count += 1;
                }
            }
        }
    });
    count
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
