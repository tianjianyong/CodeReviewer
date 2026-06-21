//! R02: 结构臃肿 - 函数行数/嵌套深度/参数个数检测。
//!
//! MVP 先实现函数行数检测（最简单，作为样板规则）。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct StructuralBloat {
    max_function_lines: i64,
}

impl Default for StructuralBloat {
    fn default() -> Self {
        Self {
            max_function_lines: 50,
        }
    }
}

impl Rule for StructuralBloat {
    fn id(&self) -> &'static str {
        "R02"
    }
    fn name(&self) -> &'static str {
        "structural-bloat"
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
        let max = ctx
            .rule_config
            .threshold_i64("max_function_lines", self.max_function_lines) as usize;
        let max_nesting = ctx
            .rule_config
            .threshold_i64("max_nesting_depth", 4) as usize;
        let max_params = ctx
            .rule_config
            .threshold_i64("max_parameters", 5) as usize;
        let mut findings = Vec::new();

        let function_kinds = function_kinds(ctx.language);
        walk_nodes(ctx.tree.root_node(), &mut |node| {
            if function_kinds.contains(&node.kind()) {
                let start = node.start_position();
                let end = node.end_position();
                let lines = end.row.saturating_sub(start.row) + 1;
                if lines > max {
                    findings.push(Finding {
                        rule_id: "R02",
                        rule_name: "structural-bloat",
                        severity: Severity::Warning,
                        location: Location {
                            file: ctx.file_path.to_path_buf(),
                            line: start.row + 1,
                            column: start.column + 1,
                        },
                        message: format!(
                            "function too long: {} lines (max {})",
                            lines, max
                        ),
                        snippet: None,
                    });
                }

                if let Some(params) = parameter_count(node, ctx.language) {
                    if params > max_params {
                        findings.push(Finding {
                            rule_id: "R02",
                            rule_name: "structural-bloat",
                            severity: Severity::Warning,
                            location: Location {
                                file: ctx.file_path.to_path_buf(),
                                line: start.row + 1,
                                column: start.column + 1,
                            },
                            message: format!(
                                "too many parameters: {} (max {})",
                                params, max_params
                            ),
                            snippet: None,
                        });
                    }
                }

                let depth = max_nesting_depth(node);
                if depth > max_nesting {
                    findings.push(Finding {
                        rule_id: "R02",
                        rule_name: "structural-bloat",
                        severity: Severity::Warning,
                        location: Location {
                            file: ctx.file_path.to_path_buf(),
                            line: start.row + 1,
                            column: start.column + 1,
                        },
                        message: format!(
                            "nesting too deep: {} levels (max {})",
                            depth, max_nesting
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
        Language::Rust => &["function_item", "function_definition"],
        Language::Python => &["function_definition"],
        Language::TypeScript | Language::TypeScriptTsx => &[
            "function_declaration",
            "method_definition",
            "arrow_function",
        ],
        Language::CSharp => &["method_declaration", "constructor_declaration"],
        Language::Java => &["method_declaration", "constructor_declaration"],
    }
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

fn parameter_count(node: tree_sitter::Node, lang: Language) -> Option<usize> {
    let param_kind = match lang {
        Language::Rust => "parameters",
        Language::Python => "parameters",
        Language::TypeScript | Language::TypeScriptTsx => "formal_parameters",
        Language::CSharp => "parameter_list",
        Language::Java => "formal_parameters",
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == param_kind {
            let mut count = 0;
            let mut child_cursor = child.walk();
            for param in child.children(&mut child_cursor) {
                if is_parameter_node(&param, lang) {
                    count += 1;
                }
            }
            return Some(count);
        }
    }
    None
}

fn is_parameter_node(node: &tree_sitter::Node, lang: Language) -> bool {
    match lang {
        Language::Rust => node.kind() == "parameter",
        Language::Python => node.kind() == "identifier" || node.kind() == "typed_parameter",
        Language::TypeScript | Language::TypeScriptTsx => {
            node.kind() == "required_parameter" || node.kind() == "optional_parameter"
        }
        Language::CSharp => node.kind() == "parameter",
        Language::Java => node.kind() == "formal_parameter",
    }
}

fn max_nesting_depth(node: tree_sitter::Node) -> usize {
    fn depth(n: tree_sitter::Node, nesting_kinds: &[&str]) -> usize {
        let mut max = 0;
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            if nesting_kinds.contains(&child.kind()) {
                max = max.max(depth(child, nesting_kinds));
            }
        }
        if nesting_kinds.contains(&n.kind()) {
            1 + max
        } else {
            max
        }
    }
    let nesting_kinds = [
        "if_statement",
        "if_expression",
        "match_expression",
        "for_statement",
        "while_statement",
        "loop_expression",
        "block",
        "try_statement",
        "catch_clause",
        "with_statement",
        "switch_statement",
    ];
    depth(node, &nesting_kinds)
}
