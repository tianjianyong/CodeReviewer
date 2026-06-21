//! R07: 死代码检测（单文件内未用的 import / 变量）。
//!
//! MVP 仅做单文件内未用 import 检测，跨文件死代码需全局分析（Phase 2.5）。

use std::collections::HashSet;

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct DeadCode;

impl Rule for DeadCode {
    fn id(&self) -> &'static str {
        "R07"
    }
    fn name(&self) -> &'static str {
        "dead-code"
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
        let imports = collect_imports(ctx);
        if imports.is_empty() {
            return Ok(Vec::new());
        }

        let used = collect_used_identifiers(ctx);
        let mut findings = Vec::new();
        for imp in &imports {
            if !used.contains(&imp.name) {
                findings.push(Finding {
                    rule_id: "R07",
                    rule_name: "dead-code",
                    severity: Severity::Warning,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: imp.line,
                        column: 1,
                    },
                    message: format!("unused import: {}", imp.name),
                    snippet: None,
                });
            }
        }
        Ok(findings)
    }
}

struct Import {
    name: String,
    line: usize,
}

fn collect_imports(ctx: &AnalysisContext) -> Vec<Import> {
    let import_kinds: &[&str] = match ctx.language {
        Language::Rust => &["use_declaration"],
        Language::Python => &["import_statement", "import_from_statement"],
        Language::TypeScript | Language::TypeScriptTsx => &["import_statement"],
        Language::CSharp => &["using_directive"],
        Language::Java => &["import_declaration"],
    };

    let mut imports = Vec::new();
    walk(ctx.tree.root_node(), &mut |node| {
        if import_kinds.contains(&node.kind()) {
            let text = node_text(&node, ctx.source);
            let pos = node.start_position();
            for name in extract_import_names(text, ctx.language) {
                imports.push(Import {
                    name,
                    line: pos.row + 1,
                });
            }
        }
    });
    imports
}

fn extract_import_names(text: &str, lang: Language) -> Vec<String> {
    match lang {
        Language::Rust => {
            let text = text.trim_start_matches("use ").trim_start();
            let text = text.trim_end_matches(';').trim();
            if let Some(pos) = text.rfind("::") {
                let last = &text[pos + 2..];
                if last == "*" {
                    return Vec::new();
                }
                return vec![last.trim().to_string()];
            }
            vec![text.trim().to_string()]
        }
        Language::Python => {
            let text = text.trim();
            if text.starts_with("from ") {
                if let Some(pos) = text.find(" import ") {
                    let names = &text[pos + 8..];
                    return names
                        .split(',')
                        .map(|s| s.trim().trim_start_matches("as ").split(" as ").last().unwrap_or("").trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            if text.starts_with("import ") {
                let names = &text[7..];
                return names
                    .split(',')
                    .map(|s| s.trim().split(" as ").last().unwrap_or("").trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            Vec::new()
        }
        Language::TypeScript | Language::TypeScriptTsx => {
            if let Some(pos) = text.find('{') {
                let end = text[pos..].find('}').map(|e| pos + e).unwrap_or(text.len());
                let names = &text[pos + 1..end];
                return names
                    .split(',')
                    .map(|s| s.trim().split(" as ").last().unwrap_or("").trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            if let Some(pos) = text.find("from") {
                let _ = pos;
                return Vec::new();
            }
            Vec::new()
        }
        Language::CSharp => {
            let text = text.trim_start_matches("using ").trim_end_matches(';').trim();
            if let Some(pos) = text.rfind('.') {
                return vec![text[pos + 1..].trim().to_string()];
            }
            vec![text.to_string()]
        }
        Language::Java => {
            let text = text.trim_start_matches("import ").trim_end_matches(';').trim();
            if let Some(pos) = text.rfind('.') {
                return vec![text[pos + 1..].trim().to_string()];
            }
            vec![text.to_string()]
        }
    }
}

fn collect_used_identifiers(ctx: &AnalysisContext) -> HashSet<String> {
    let identifier_kinds: &[&str] = match ctx.language {
        Language::Rust => &["identifier", "type_identifier", "field_identifier"],
        Language::Python => &["identifier"],
        Language::TypeScript | Language::TypeScriptTsx => &["identifier", "type_identifier"],
        Language::CSharp => &["identifier"],
        Language::Java => &["identifier"],
    };

    let import_kinds: &[&str] = match ctx.language {
        Language::Rust => &["use_declaration"],
        Language::Python => &["import_statement", "import_from_statement"],
        Language::TypeScript | Language::TypeScriptTsx => &["import_statement"],
        Language::CSharp => &["using_directive"],
        Language::Java => &["import_declaration"],
    };

    let mut used = HashSet::new();
    walk(ctx.tree.root_node(), &mut |node| {
        if import_kinds.contains(&node.kind()) {
            return;
        }
        if identifier_kinds.contains(&node.kind()) {
            let text = node_text(&node, ctx.source);
            if !text.is_empty() {
                used.insert(text.to_string());
            }
        }
    });
    used
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
