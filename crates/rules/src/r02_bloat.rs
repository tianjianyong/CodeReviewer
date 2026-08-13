//! R02: 结构臃肿 - 函数行数/嵌套深度/参数个数检测。

use codereviewer_core::ast::walk;
use codereviewer_core::finding::{Finding, Severity};
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
        let max =
            ctx.rule_config
                .threshold_i64("max_function_lines", self.max_function_lines) as usize;
        let max_nesting = ctx.rule_config.threshold_i64("max_nesting_depth", 4) as usize;
        let max_params = ctx.rule_config.threshold_i64("max_parameters", 5) as usize;
        let mut findings = Vec::new();

        let function_kinds = function_kinds(ctx.language);
        walk(ctx.tree.root_node(), &mut |node| {
            if function_kinds.contains(&node.kind()) {
                let start = node.start_position();
                let end = node.end_position();
                let lines = end.row.saturating_sub(start.row) + 1;
                if lines > max {
                    findings.push(Finding::new(
                        "R02",
                        "structural-bloat",
                        Severity::Warning,
                        ctx.file_path,
                        &node,
                        ctx.source,
                        format!(
                            "函数过长：{} 行（上限 {}） | function too long: {} lines (max {})",
                            lines, max, lines, max
                        ),
                    ));
                }

                if let Some(params) = parameter_count(node, ctx.language)
                    && params > max_params
                {
                    findings.push(Finding::new(
                        "R02",
                        "structural-bloat",
                        Severity::Warning,
                        ctx.file_path,
                        &node,
                        ctx.source,
                        format!(
                            "参数过多：{} 个（上限 {}） | too many parameters: {} (max {})",
                            params, max_params, params, max_params
                        ),
                    ));
                }

                let depth = max_nesting_depth(node);
                if depth > max_nesting {
                    findings.push(Finding::new(
                        "R02",
                        "structural-bloat",
                        Severity::Warning,
                        ctx.file_path,
                        &node,
                        ctx.source,
                        format!(
                            "嵌套过深：{} 层（上限 {}） | nesting too deep: {} levels (max {})",
                            depth, max_nesting, depth, max_nesting
                        ),
                    ));
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
            "arrow_function",
        ],
        Language::CSharp => &["method_declaration", "constructor_declaration"],
        Language::Java => &["method_declaration", "constructor_declaration"],
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
    // 迭代式遍历：深度向下传递，避免深层 AST 递归栈溢出
    let mut max = 0;
    let mut stack: Vec<(tree_sitter::Node, usize)> = vec![(node, 0)];
    while let Some((n, depth)) = stack.pop() {
        let cur = if nesting_kinds.contains(&n.kind()) {
            depth + 1
        } else {
            depth
        };
        max = max.max(cur);
        let mut cursor = n.walk();
        for child in n.children(&mut cursor) {
            stack.push((child, cur));
        }
    }
    max
}
