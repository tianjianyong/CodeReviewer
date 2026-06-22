//! R24: 硬编码路径/URL 检测。
//!
//! 检测源码中硬编码的绝对路径或带 host 的 URL（非测试文件、非 env/config 包裹）。
//! AI 常把 prompt 里的本地环境路径/URL 复制进代码。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct HardcodedPathOrUrl;

impl Rule for HardcodedPathOrUrl {
    fn id(&self) -> &'static str {
        "R24"
    }
    fn name(&self) -> &'static str {
        "hardcoded-path-or-url"
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
        if is_test_file(ctx.file_path) {
            return Ok(Vec::new());
        }
        let literal_kinds = literal_kinds(ctx.language);
        let env_signals = env_signals(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if !literal_kinds.contains(&node.kind()) {
                return;
            }
            let text = node_text(&node, ctx.source);
            let inner = strip_quotes(text);
            if let Some(kind) = classify(inner) {
                let line_text = line_of(ctx.source, node.start_position().row);
                if env_signals.iter().any(|s| line_text.contains(s)) {
                    return;
                }
                let pos = node.start_position();
                findings.push(Finding {
                    rule_id: "R24",
                    rule_name: "hardcoded-path-or-url",
                    severity: Severity::Warning,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                    },
                    message: format!(
                        "硬编码{}：{} | hardcoded {}: {}",
                        kind_zh(kind), inner, kind_en(kind), inner
                    ),
                    snippet: None,
                });
            }
        });

        Ok(findings)
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Path,
    Url,
}

fn kind_zh(k: Kind) -> &'static str {
    match k {
        Kind::Path => "路径",
        Kind::Url => "URL",
    }
}

fn kind_en(k: Kind) -> &'static str {
    match k {
        Kind::Path => "path",
        Kind::Url => "URL",
    }
}

fn classify(s: &str) -> Option<Kind> {
    // 过滤无意义的短串、占位符、scheme 片段
    if s.len() < 8 || s.contains("{}") || s.contains("{0}") || s.contains("{1}") {
        return None;
    }
    if s == "https://" || s == "http://" || s == "/**" || s == "/*" {
        return None;
    }
    if s.starts_with("https://") || s.starts_with("http://") {
        if !is_localhost_url(s) {
            return Some(Kind::Url);
        }
    }
    if is_absolute_path(s) {
        return Some(Kind::Path);
    }
    None
}

fn is_localhost_url(s: &str) -> bool {
    let rest = s.split("://").nth(1).unwrap_or("");
    let host = rest.split(['/', ':']).next().unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "0.0.0.0" | "::1")
}

fn is_absolute_path(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Unix 绝对路径 /Users /home /tmp /var 等
    if s.starts_with('/') {
        return !s.starts_with("//"); // 排除 URL 残片
    }
    // Windows 绝对路径 C:\ D:\ 等
    let bytes = s.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/') {
        return bytes[0].is_ascii_alphabetic();
    }
    false
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    let s = s.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(s);
    let s = s.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')).unwrap_or(s);
    s
}

fn env_signals(lang: Language) -> Vec<&'static str> {
    match lang {
        Language::Rust => vec!["env::var", "env!", "std::env"],
        Language::Python => vec!["os.environ", "os.getenv", "dotenv", "config("],
        Language::TypeScript | Language::TypeScriptTsx => {
            vec!["process.env", "dotenv", "config("]
        }
        Language::CSharp => vec!["Environment.GetEnvironmentVariable", "Configuration"],
        Language::Java => vec!["System.getenv", "getProperty"],
    }
}

fn literal_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["string_literal"],
        Language::Python => &["string"],
        Language::TypeScript | Language::TypeScriptTsx => &["string", "template_string"],
        Language::CSharp => &["string_literal"],
        Language::Java => &["string_literal"],
    }
}

fn is_test_file(path: &std::path::Path) -> bool {
    let has_test_dir = path
        .components()
        .any(|c| matches!(c.as_os_str().to_string_lossy().to_lowercase().as_str(), "tests" | "test" | "__tests__"));
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
}

fn line_of(source: &str, row: usize) -> &str {
    source.lines().nth(row).unwrap_or("")
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
