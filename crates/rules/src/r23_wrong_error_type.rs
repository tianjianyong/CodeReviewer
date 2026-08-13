//! R23: 错误类型传播丢信息检测。
//!
//! 启发式：宽泛 catch/except（Exception/base）体内返回固定状态码或泛型错误，
//! 且不引用异常变量——丢失了本应区分的错误信息。
//! 与 R01 互补：R01 是吞错误，R23 是错误有传播但类型丢信息。

use codereviewer_core::ast::{node_text, walk};
use codereviewer_core::finding::{Finding, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct WrongErrorTypePropagation;

impl Rule for WrongErrorTypePropagation {
    fn id(&self) -> &'static str {
        "R23"
    }
    fn name(&self) -> &'static str {
        "wrong-error-type-propagation"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn languages(&self) -> &'static [Language] {
        // Rust 无 catch/except 语法
        &[
            Language::Python,
            Language::TypeScript,
            Language::TypeScriptTsx,
            Language::CSharp,
            Language::Java,
        ]
    }

    fn analyze(&self, ctx: &AnalysisContext) -> Result<Vec<Finding>, RuleError> {
        let catch_kind = catch_kind(ctx.language);
        let mut findings = Vec::new();

        walk(ctx.tree.root_node(), &mut |node| {
            if node.kind() != catch_kind {
                return;
            }
            if !is_broad_catch(&node, ctx) {
                return;
            }
            let body_text = node_text(&node, ctx.source);
            let exc_var = exception_var(&node, ctx);
            // 头部之后的体内按词边界引用了异常变量 → 在用信息，不报
            if !exc_var.is_empty() && body_uses_word(body_text, &exc_var) {
                return;
            }
            if returns_generic(&node, ctx) {
                findings.push(Finding::new(
                    "R23",
                    "wrong-error-type-propagation",
                    Severity::Warning,
                    ctx.file_path,
                    &node,
                    ctx.source,
                    format!(
                        "宽泛 {} 返回固定值且不引用异常变量，丢失错误类型信息 | broad catch returns fixed value without inspecting exception, loses error type info",
                        catch_kind
                    ),
                ));
            }
        });

        Ok(findings)
    }
}

fn catch_kind(lang: Language) -> &'static str {
    match lang {
        Language::Python => "except_clause",
        _ => "catch_clause",
    }
}

fn is_broad_catch(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let text = node_text(node, ctx.source);
    match ctx.language {
        Language::Python => {
            let trimmed = text.trim_start();
            trimmed.starts_with("except:")
                || trimmed.starts_with("except Exception")
                || trimmed.starts_with("except BaseException")
        }
        _ => {
            // catch (Exception / Throwable / Error e) 或 bare catch
            text.contains("Exception")
                || text.contains("Throwable")
                || text.contains("(e)")
                || text.contains("catch {")
        }
    }
}

fn exception_var(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
    let mut cursor = node.walk();
    let children: Vec<tree_sitter::Node> = node.children(&mut cursor).collect();
    // Python: "except Exception as e" 解析为 as_pattern 子节点
    for child in &children {
        if child.kind() == "as_pattern" {
            let text = node_text(child, ctx.source);
            if let Some(pos) = text.find(" as ") {
                return text[pos + 4..].trim().to_string();
            }
        }
    }
    // C#/Java/TS: catch (Exception e) 的 e 是 identifier 子节点
    for child in &children {
        if child.kind() == "identifier" {
            let t = node_text(child, ctx.source);
            // 跳过类型名 Exception/Throwable
            if !matches!(t, "Exception" | "Throwable" | "Error" | "BaseException") {
                return t.to_string();
            }
        }
    }
    String::new()
}

fn returns_generic(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let text = node_text(node, ctx.source);
    // 体内 return 一个固定数字状态码 / None / null / false / 泛型 Error
    let generic_returns = [
        "return 500",
        "return 400",
        "return 404",
        "return 422",
        "return None",
        "return null",
        "return False",
        "return false",
        "return 0",
        "return -1",
        "return Err(Generic",
        "return Err(generic",
        "throw new Error(",
        "return ResponseEntity.status(500)",
        "return StatusCode::INTERNAL_SERVER_ERROR",
    ];
    generic_returns.iter().any(|g| text.contains(g))
}

/// 在 catch 头部之后（':' 或 '{' 后）检查异常变量是否被使用。
/// 此前实现直接在头部文本上 counts，"except Exception as e" 里的 "e" 总是命中，
/// 导致带异常变量的常见写法永远不会被报。
fn body_uses_word(text: &str, word: &str) -> bool {
    let body = text
        .find(':')
        .or_else(|| text.find('{'))
        .map(|i| &text[i + 1..])
        .unwrap_or("");
    contains_word(body, word)
}

/// 词边界包含：避免 "e" 命中 "return"、"error"。
fn contains_word(text: &str, word: &str) -> bool {
    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let mut rest = text;
    while let Some(pos) = rest.find(word) {
        let end = pos + word.len();
        let before_ok = pos == 0
            || !rest[..pos]
                .chars()
                .last()
                .map(is_word_char)
                .unwrap_or(false);
        let after_ok = end >= rest.len()
            || !rest[end..]
                .chars()
                .next()
                .map(is_word_char)
                .unwrap_or(false);
        if before_ok && after_ok {
            return true;
        }
        rest = &rest[pos + 1..];
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::analyze_source;

    #[test]
    fn broad_except_unused_var_flagged() {
        let source =
            "def f():\n    try:\n        pass\n    except Exception as e:\n        return None\n";
        let findings = analyze_source(&WrongErrorTypePropagation, source, Language::Python);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "R23");
    }

    #[test]
    fn broad_except_var_used_not_flagged() {
        let source = "def f():\n    try:\n        pass\n    except Exception as e:\n        print(e)\n        return None\n";
        let findings = analyze_source(&WrongErrorTypePropagation, source, Language::Python);
        assert!(findings.is_empty());
    }

    #[test]
    fn contains_word_respects_boundaries() {
        assert!(contains_word("print(e)", "e"));
        assert!(!contains_word("return", "e"));
        assert!(!contains_word("error", "e"));
        assert!(contains_word("log_error(e)", "e"));
    }
}
