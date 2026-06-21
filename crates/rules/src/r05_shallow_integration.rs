//! R05: 集成测试只为通过检测。
//!
//! 启发式：测试只断言状态码，不验证副作用或响应体内容。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct ShallowIntegration;

impl Rule for ShallowIntegration {
    fn id(&self) -> &'static str {
        "R05"
    }
    fn name(&self) -> &'static str {
        "shallow-integration-test"
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
        let min_checks = ctx.rule_config.threshold_i64("min_checks", 3) as usize;
        let function_kinds = function_kinds(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if function_kinds.contains(&node.kind()) && is_integration_test(&node, ctx) {
                let checks = count_meaningful_checks(&node, ctx);
                if checks < min_checks {
                    let pos = node.start_position();
                    findings.push(Finding {
                        rule_id: "R05",
                        rule_name: "shallow-integration-test",
                        severity: Severity::Warning,
                        location: Location {
                            file: ctx.file_path.to_path_buf(),
                            line: pos.row + 1,
                            column: pos.column + 1,
                        },
                        message: format!(
                            "integration test has only {} meaningful check(s) (min {})",
                            checks, min_checks
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

fn is_integration_test(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let name = extract_name(node, ctx);
    let text = node_text(node, ctx.source);
    let has_http = text.contains("request")
        || text.contains("response")
        || text.contains("client")
        || text.contains("get(")
        || text.contains("post(")
        || text.contains("Client::new")
        || text.contains("TestClient")
        || text.contains("http")
        || text.contains("api");
    let name_suggests = name.contains("integration") || name.contains("e2e") || name.contains("end_to_end");
    has_http && (name_suggests || name.starts_with("test_"))
}

fn count_meaningful_checks(node: &tree_sitter::Node, ctx: &AnalysisContext) -> usize {
    let text = node_text(node, ctx.source);
    let shallow_signals = [
        "status()", "is_success", "is_ok", "is_success()", "status_code",
        "assert_ok", "is_success", "Ok(", "unwrap()", "expect(",
    ];
    let deep_signals = [
        "json(", "body", "header", "content", "contains", "eq(",
        "assert_eq!", "assert_ne!", "expect(", "body(", "text()",
        "json_body", "response.body", "data[", "result[",
    ];
    let shallow: usize = shallow_signals.iter().map(|s| text.matches(s).count()).sum();
    let deep: usize = deep_signals.iter().map(|s| text.matches(s).count()).sum();
    if shallow > 0 && deep == 0 {
        1
    } else {
        deep + 1
    }
}

fn extract_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
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
