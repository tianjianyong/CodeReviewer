//! 跨规则共享的 catch/except 分析工具。

use codereviewer_core::ast::{contains_word, node_text};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::AnalysisContext;

/// catch (Exception e) → e；except Exception as e → e。
/// 支持 Python 的 as_pattern 节点与 C# 的 catch_declaration 子节点。
pub fn exception_var(node: &tree_sitter::Node, ctx: &AnalysisContext) -> String {
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
    // C#: "catch (Exception ex)" 的类型与变量在 catch_declaration 子节点里
    for child in &children {
        if child.kind() == "catch_declaration" {
            let mut inner = child.walk();
            for c in child.children(&mut inner) {
                if c.kind() == "identifier" {
                    let t = node_text(&c, ctx.source);
                    if !matches!(t, "Exception" | "Throwable" | "Error" | "BaseException") {
                        return t.to_string();
                    }
                }
            }
        }
    }
    // Java/TS: catch (Exception e) 的 e 是 identifier 子节点
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

/// 是否宽泛 catch（Exception/Throwable/Error/bare except）。
/// 注意：只精确匹配基类（含 System.Exception 限定名），
/// IOException/ArgumentException 等具体异常不算宽泛。
pub fn is_broad_catch(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let text = node_text(node, ctx.source);
    match ctx.language {
        Language::Python => {
            let trimmed = text.trim_start();
            trimmed.starts_with("except:")
                || trimmed.starts_with("except Exception")
                || trimmed.starts_with("except BaseException")
        }
        _ => {
            if text.contains("catch {") {
                return true;
            }
            let Some(open) = text.find('(') else {
                return false;
            };
            let type_name = text[open + 1..]
                .split([')', ' ', '\t'])
                .next()
                .unwrap_or("")
                .trim();
            if type_name.is_empty() {
                return true;
            }
            let last_seg = type_name.rsplit('.').next().unwrap_or(type_name);
            matches!(
                last_seg,
                "Exception" | "Throwable" | "RuntimeException" | "Error" | "BaseException"
            ) || matches!(type_name, "e" | "err" | "error" | "ex")
        }
    }
}

/// 是否返回固定状态码 / None / null / false / 泛型错误。
pub fn returns_generic(node: &tree_sitter::Node, ctx: &AnalysisContext) -> bool {
    let text = node_text(node, ctx.source);
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

/// 头部（':' 或 '{'）之后的体内是否按词边界引用了变量。
pub fn body_uses_word(text: &str, word: &str) -> bool {
    let body = text
        .find(':')
        .or_else(|| text.find('{'))
        .map(|i| &text[i + 1..])
        .unwrap_or("");
    contains_word(body, word)
}
