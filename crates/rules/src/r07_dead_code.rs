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
                    message: format!("未使用的 import：{} | unused import: {}", imp.name, imp.name),
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
                    // 点号导入取顶层包名：os.path -> os（代码中只能通过 os 引用）
                    .map(|s| s.split('.').next().unwrap_or("").to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            Vec::new()
        }
        Language::TypeScript | Language::TypeScriptTsx => {
            let text = text.trim();
            // 绑定部分：import 与 from 之间的内容（含 import type 前缀）
            let bindings = text
                .strip_prefix("import ")
                .unwrap_or(text)
                .split(" from ")
                .next()
                .unwrap_or("")
                .trim();
            let mut names = Vec::new();
            // 默认/命名空间导入: React | * as ns（去掉 import type 前缀）
            let mut default_part = bindings.split('{').next().unwrap_or("").trim().trim_end_matches(',');
            if default_part == "type" {
                default_part = "";
            } else if let Some(rest) = default_part.strip_prefix("type ") {
                default_part = rest;
            }
            if let Some(ns) = default_part.strip_prefix("* as ") {
                let ns = ns.trim();
                if !ns.is_empty() {
                    names.push(ns.to_string());
                }
            } else if !default_part.is_empty()
                && default_part != "*"
                && !default_part.starts_with('"')
                && !default_part.starts_with('\'')
            {
                names.push(default_part.to_string());
            }
            // 具名导入: { a, b as c }
            if let Some(pos) = bindings.find('{') {
                let end = bindings[pos..]
                    .find('}')
                    .map(|e| pos + e)
                    .unwrap_or(bindings.len());
                for s in bindings[pos + 1..end].split(',') {
                    let name = s.trim().split(" as ").last().unwrap_or("").trim();
                    if !name.is_empty() {
                        names.push(name.to_string());
                    }
                }
            }
            names
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
                let last = &text[pos + 1..];
                // 通配符导入无法判断使用情况，跳过
                if last == "*" {
                    return Vec::new();
                }
                return vec![last.trim().to_string()];
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
    // 手动遍历以跳过 import 语句子树：import 自身的标识符不应算作"使用"
    let mut stack = vec![ctx.tree.root_node()];
    while let Some(node) = stack.pop() {
        if import_kinds.contains(&node.kind()) {
            continue;
        }
        if identifier_kinds.contains(&node.kind()) {
            let text = node_text(&node, ctx.source);
            if !text.is_empty() {
                used.insert(text.to_string());
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_dotted_import_takes_top_module() {
        assert_eq!(
            extract_import_names("import os.path", Language::Python),
            vec!["os".to_string()]
        );
        assert_eq!(
            extract_import_names("import os.path as p", Language::Python),
            vec!["p".to_string()]
        );
        assert_eq!(
            extract_import_names("import os, sys", Language::Python),
            vec!["os".to_string(), "sys".to_string()]
        );
        assert_eq!(
            extract_import_names("from x import a, b as c", Language::Python),
            vec!["a".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn ts_default_and_namespace_imports_extract_binding() {
        assert_eq!(
            extract_import_names("import React from \"react\"", Language::TypeScript),
            vec!["React".to_string()]
        );
        assert_eq!(
            extract_import_names("import * as ns from \"x\"", Language::TypeScript),
            vec!["ns".to_string()]
        );
        assert_eq!(
            extract_import_names("import { a, b as c } from \"x\"", Language::TypeScript),
            vec!["a".to_string(), "c".to_string()]
        );
        assert_eq!(
            extract_import_names("import \"side-effect\"", Language::TypeScript),
            Vec::<String>::new()
        );
        assert_eq!(
            extract_import_names("import type { x } from \"x\"", Language::TypeScript),
            vec!["x".to_string()]
        );
        assert_eq!(
            extract_import_names("import React, { useState } from \"react\"", Language::TypeScript),
            vec!["React".to_string(), "useState".to_string()]
        );
    }

    #[test]
    fn java_wildcard_import_is_skipped() {
        assert_eq!(
            extract_import_names("import java.util.*;", Language::Java),
            Vec::<String>::new()
        );
        assert_eq!(
            extract_import_names("import java.util.List;", Language::Java),
            vec!["List".to_string()]
        );
    }
}
