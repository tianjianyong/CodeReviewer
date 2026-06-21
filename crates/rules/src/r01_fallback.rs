//! R01: 回退掩盖问题检测。
//!
//! 检测模式：catch 后返回默认值、unwrap_or 吞错误、? 后紧接 fallback。

use codereviewer_core::finding::{Finding, Location, Severity};
use codereviewer_core::parser::Language;
use codereviewer_core::rule::{AnalysisContext, Rule, RuleError};

pub struct FallbackMasksError;

impl Rule for FallbackMasksError {
    fn id(&self) -> &'static str {
        "R01"
    }
    fn name(&self) -> &'static str {
        "fallback-masks-error"
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
        let mut findings = Vec::new();
        let patterns = patterns(ctx.language);

        for (i, line) in ctx.source.lines().enumerate() {
            let trimmed = line.trim();
            for pat in patterns {
                if trimmed.contains(pat.keyword) {
                if (pat.is_fallback)(trimmed) {
                        findings.push(Finding {
                            rule_id: "R01",
                            rule_name: "fallback-masks-error",
                            severity: Severity::Error,
                            location: Location {
                                file: ctx.file_path.to_path_buf(),
                                line: i + 1,
                                column: 1,
                            },
                            message: pat.message.to_string(),
                            snippet: Some(trimmed.to_string()),
                        });
                    }
                }
            }
        }
        Ok(findings)
    }
}

struct Pattern {
    keyword: &'static str,
    message: &'static str,
    is_fallback: fn(&str) -> bool,
}

fn patterns(lang: Language) -> &'static [Pattern] {
    match lang {
        Language::Rust => &[
            Pattern {
                keyword: "unwrap_or_default",
                message: "unwrap_or_default() masks error case",
                is_fallback: |s| s.contains("unwrap_or_default()"),
            },
            Pattern {
                keyword: "unwrap_or(",
                message: "unwrap_or() masks None/Err case",
                is_fallback: |s| s.contains("unwrap_or(") && !s.contains("unwrap_or_else"),
            },
            Pattern {
                keyword: "unwrap_or_else(",
                message: "unwrap_or_else() may mask error case",
                is_fallback: |s| s.contains("unwrap_or_else(||") && !s.contains("?"),
            },
        ],
        Language::Python => &[
            Pattern {
                keyword: "except:",
                message: "bare except masks errors",
                is_fallback: |s| s.contains("except:") || s.contains("except Exception:"),
            },
            Pattern {
                keyword: ".get(",
                message: ".get() with default masks missing key",
                is_fallback: |s| {
                    s.contains(".get(") && s.contains(",") && !s.contains("##")
                },
            },
        ],
        Language::TypeScript | Language::TypeScriptTsx => &[
            Pattern {
                keyword: "catch",
                message: "catch with default return masks error",
                is_fallback: |s| {
                    (s.contains("catch") && (s.contains("return") || s.contains("||")))
                        || (s.contains("??") && s.contains("return"))
                },
            },
        ],
        Language::CSharp => &[
            Pattern {
                keyword: "catch",
                message: "catch with default return masks exception",
                is_fallback: |s| s.contains("catch") && s.contains("return"),
            },
        ],
        Language::Java => &[
            Pattern {
                keyword: "catch",
                message: "catch with default return masks exception",
                is_fallback: |s| s.contains("catch") && s.contains("return"),
            },
        ],
    }
}
