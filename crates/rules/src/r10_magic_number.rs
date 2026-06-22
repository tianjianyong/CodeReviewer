//! R10: 魔法数字/字符串检测。
//!
//! 检测未命名的数字/字符串字面量，排除常见值与属性/宏/match pattern 上下文。

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
        // 测试文件里的数字几乎都是测试输入数据，跳过 R10
        if is_test_file(ctx.file_path) {
            return Ok(Vec::new());
        }
        let literal_kinds = literal_kinds(ctx.language);
        let skip_parent_kinds = skip_parent_kinds(ctx.language);
        let min_string_len = ctx.rule_config.threshold_i64("min_string_length", 10) as usize;
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if !literal_kinds.contains(&node.kind()) {
                return;
            }
            if in_skip_context(&node, &skip_parent_kinds) {
                return;
            }
            let text = node_text(&node, ctx.source);
            if !is_magic(node.kind(), text, min_string_len) {
                return;
            }
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
                message: format!("魔法字面量：{} | magic literal: {}", text, text),
                snippet: None,
            });
        });

        Ok(findings)
    }
}

fn in_skip_context(node: &tree_sitter::Node, skip_kinds: &[&str]) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if skip_kinds.contains(&parent.kind()) {
            return true;
        }
        current = parent.parent();
    }
    false
}

fn skip_parent_kinds(lang: Language) -> Vec<&'static str> {
    match lang {
        Language::Rust => vec![
            "attribute_item",
            "meta_item",
            "token_tree",
            "macro_invocation",
            "match_pattern",
        ],
        Language::Python => vec!["decorator", "match_pattern"],
        Language::TypeScript | Language::TypeScriptTsx => vec!["decorator", "type_annotation"],
        Language::CSharp => vec!["attribute", "attribute_argument"],
        Language::Java => vec!["annotation", "annotation_argument"],
    }
}

fn literal_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["integer_literal", "float_literal", "string_literal"],
        Language::Python => &["integer", "float", "string"],
        Language::TypeScript | Language::TypeScriptTsx => &["number", "string", "template_string"],
        Language::CSharp => &["integer_literal", "real_literal", "string_literal"],
        Language::Java => &[
            "decimal_integer_literal",
            "decimal_floating_point_literal",
            "string_literal",
        ],
    }
}

fn is_test_file(path: &std::path::Path) -> bool {
    // 遍历路径各段目录名，判断是否位于 tests/test/__tests__ 目录下。
    // 用 components() 而非字符串 contains，避免相对路径（开头无分隔符）漏判。
    let has_test_dir = path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy().to_lowercase();
        matches!(s.as_str(), "tests" | "test" | "__tests__")
    });
    if has_test_dir {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    name.starts_with("test_")
        || name.starts_with("test.")
        || name.contains("_test.")
        || name.contains(".test.")
        || name.contains(".spec.")
        || name.ends_with("tests.cs")
        || name.ends_with("test.cs")
        || name.ends_with("test.java")
        || name.ends_with("tests.java")
}

fn is_magic(kind: &str, text: &str, min_string_len: usize) -> bool {
    if kind.contains("string") || kind == "template_string" {
        if text.len() <= min_string_len {
            return false;
        }
        if text.contains("{}") || text.contains("{0}") || text.contains("{1}") {
            return false;
        }
        return true;
    }
    let n: i64 = text.trim().parse().unwrap_or(0);
    !matches!(n, 0 | 1 | -1 | 2 | 10 | 100 | 1000)
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::is_test_file;

    #[test]
    fn test_relative_path_with_tests_dir() {
        assert!(is_test_file(Path::new("tests/fixtures/problematic.rs")));
        assert!(is_test_file(Path::new("tests/foo.rs")));
    }

    #[test]
    fn test_absolute_path_with_tests_dir() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests")
            .join("fixtures")
            .join("problematic.rs");
        assert!(is_test_file(&p));
    }

    #[test]
    fn test_unix_style_relative_path() {
        assert!(is_test_file(Path::new("tests/fixtures/foo.rs")));
        assert!(is_test_file(Path::new("src/__tests__/foo.ts")));
    }

    #[test]
    fn test_non_test_path_not_flagged() {
        assert!(!is_test_file(Path::new("src/main.rs")));
        assert!(!is_test_file(Path::new("crates/core/src/finding.rs")));
    }

    #[test]
    fn test_test_file_by_name() {
        assert!(is_test_file(Path::new("src/test_foo.rs")));
        assert!(is_test_file(Path::new("src/foo_test.rs")));
        assert!(is_test_file(Path::new("src/foo.test.ts")));
        assert!(is_test_file(Path::new("src/foo.spec.ts")));
    }
}
