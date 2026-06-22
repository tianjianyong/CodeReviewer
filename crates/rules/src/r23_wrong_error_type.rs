//! R23: 错误类型传播丢信息检测。
//!
//! 启发式：宽泛 catch/except（Exception/base）体内返回固定状态码或泛型错误，
//! 且不引用异常变量——丢失了本应区分的错误信息。
//! 与 R01 互补：R01 是吞错误，R23 是错误有传播但类型丢信息。

use codereviewer_core::finding::{Finding, Location, Severity};
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
            // 体内引用了异常变量 → 在用信息，不报
            if !exc_var.is_empty() && body_text.matches(&exc_var).count() > 1 {
                return;
            }
            if returns_generic(&node, ctx) {
                let pos = node.start_position();
                findings.push(Finding {
                    rule_id: "R23",
                    rule_name: "wrong-error-type-propagation",
                    severity: Severity::Warning,
                    location: Location {
                        file: ctx.file_path.to_path_buf(),
                        line: pos.row + 1,
                        column: pos.column + 1,
                    },
                    message: format!(
                        "宽泛 {} 返回固定值且不引用异常变量，丢失错误类型信息 | broad catch returns fixed value without inspecting exception, loses error type info",
                        catch_kind
                    ),
                    snippet: None,
                });
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
    // catch (Exception e) → e;  except Exception as e → e
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let t = node_text(&child, ctx.source);
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
        "return 500", "return 400", "return 404", "return 422",
        "return None", "return null", "return False", "return false",
        "return 0", "return -1",
        "return Err(Generic", "return Err(generic", "throw new Error(",
        "return ResponseEntity.status(500)",
        "return StatusCode::INTERNAL_SERVER_ERROR",
    ];
    generic_returns.iter().any(|g| text.contains(g))
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
