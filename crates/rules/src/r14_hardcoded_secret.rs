//! R14: 硬编码密钥检测。
//!
//! 检测源码中硬编码的 API key/token/私钥。
//! 启发式：(1) 已知前缀（sk-/ghp_/AKIA/xoxb-/AIza/BEGIN PRIVATE KEY）；
//! (2) 高熵字符串（≥20 字符 base64/hex）赋给 secret 命名变量。

use codereviewer_core::ast::{node_text, walk};
use codereviewer_core::finding::{Finding, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct HardcodedSecret;

impl Rule for HardcodedSecret {
    fn id(&self) -> &'static str {
        "R14"
    }
    fn name(&self) -> &'static str {
        "hardcoded-secret"
    }
    fn severity(&self) -> Severity {
        Severity::Error
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
        let min_len = ctx.rule_config.threshold_i64("min_length", 16) as usize;
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if !literal_kinds.contains(&node.kind()) {
                return;
            }
            let text = node_text(&node, ctx.source);
            let inner = strip_quotes(text);
            if inner.len() < 6 {
                return;
            }
            let reason = classify(inner, min_len);
            if reason.is_none() {
                return;
            }
            let reason = reason.unwrap();
            // 高熵串必须赋给 secret 命名变量；已知前缀直接报
            if matches!(reason, Reason::HighEntropy) {
                let var = enclosing_var_name(&node, ctx);
                if !is_secret_named(&var) {
                    return;
                }
            }
            findings.push(Finding::new(
                "R14",
                "hardcoded-secret",
                Severity::Error,
                ctx.file_path,
                &node,
                ctx.source,
                format!(
                    "疑似硬编码密钥（{}）：{} | suspected hardcoded secret ({}): {}",
                    reason_zh(reason),
                    inner,
                    reason_en(reason),
                    inner
                ),
            ));
        });

        Ok(findings)
    }
}

#[derive(Clone, Copy)]
enum Reason {
    KnownPrefix,
    PrivateKey,
    HighEntropy,
}

fn reason_zh(r: Reason) -> &'static str {
    match r {
        Reason::KnownPrefix => "已知密钥前缀",
        Reason::PrivateKey => "私钥头",
        Reason::HighEntropy => "高熵字符串",
    }
}

fn reason_en(r: Reason) -> &'static str {
    match r {
        Reason::KnownPrefix => "known key prefix",
        Reason::PrivateKey => "private key header",
        Reason::HighEntropy => "high-entropy string",
    }
}

fn classify(s: &str, min_len: usize) -> Option<Reason> {
    if s.contains("BEGIN") && s.contains("PRIVATE KEY") {
        return Some(Reason::PrivateKey);
    }
    let prefixes = [
        "sk-",
        "sk_",
        "ghp_",
        "gho_",
        "github_pat_",
        "AKIA",
        "xoxb-",
        "xoxp-",
        "AIza",
        "eyJ", // JWT 头
    ];
    if prefixes.iter().any(|p| s.starts_with(p)) {
        return Some(Reason::KnownPrefix);
    }
    if s.len() >= min_len && is_high_entropy(s) {
        return Some(Reason::HighEntropy);
    }
    None
}

fn is_high_entropy(s: &str) -> bool {
    // URL 路径/文件路径含 '/'，不像随机密钥；JWT 含 '.'，但 eyJ 前缀已被 KnownPrefix 覆盖
    if s.contains('/') || s.contains('.') {
        return false;
    }
    // 全大写蛇形标识符（如 COMPREFACE_API_KEY）是环境变量名本身，不是密钥值
    let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = s.chars().any(|c| c.is_ascii_digit());
    if !has_lower && !has_digit {
        return false;
    }
    // base64 / hex 字符集
    let is_b64 = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '_' || c == '-');
    let is_hex = s.chars().all(|c| c.is_ascii_hexdigit());
    if !is_b64 && !is_hex {
        return false;
    }
    // 唯一字符数 / 长度 比例高 → 看起来像随机串
    let mut chars: Vec<char> = s.chars().collect();
    chars.sort();
    chars.dedup();
    chars.len() as f64 / s.len().max(1) as f64 >= 0.4
}

fn is_secret_named(name: &str) -> bool {
    let lower = name.to_lowercase();
    [
        "api_key",
        "apikey",
        "secret",
        "token",
        "password",
        "passwd",
        "pwd",
        "private_key",
        "priv_key",
        "privkey",
        "access_key",
        "auth",
        "credential",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn enclosing_var_name(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    let mut current = node.parent();
    while let Some(parent) = current {
        // 赋值左侧 identifier
        if parent.kind() == "assignment" || parent.kind() == "assignment_expression" {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "variable_name" {
                    return node_text(&child, ctx.source).to_string();
                }
            }
        }
        // let / const / var 声明
        for decl_kind in [
            "let_declaration",
            "let_statement",
            "variable_declaration",
            "local_variable_declaration",
            "field_declaration",
            "constant_declaration",
            "const_item",
            "static_item",
        ] {
            if parent.kind() == decl_kind {
                let mut cursor = parent.walk();
                for child in parent.children(&mut cursor) {
                    if child.kind() == "identifier"
                        || child.kind() == "type_identifier"
                        || child.kind() == "variable_name"
                    {
                        return node_text(&child, ctx.source).to_string();
                    }
                }
            }
        }
        current = parent.parent();
    }
    String::new()
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s);

    (s.strip_prefix('\'')
        .and_then(|x| x.strip_suffix('\''))
        .unwrap_or(s)) as _
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
    let has_test_dir = path.components().any(|c| {
        matches!(
            c.as_os_str().to_string_lossy().to_lowercase().as_str(),
            "tests" | "test" | "__tests__"
        )
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
}
