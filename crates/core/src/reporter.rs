//! Reporter: 输出 finding，终端文本或 JSON。

use crate::analyzer::AnalysisResult;
use crate::finding::Severity;

pub struct Report {
    pub result: AnalysisResult,
}

impl Report {
    pub fn render_text(&self) -> String {
        self.render_text_impl(false)
    }

    pub fn render_text_colored(&self) -> String {
        self.render_text_impl(true)
    }

    fn render_text_impl(&self, color: bool) -> String {
        let mut out = String::new();
        for f in &self.result.findings {
            let sev = if color {
                colorize(f.severity)
            } else {
                f.severity.display_label().to_string()
            };
            out.push_str(&format!(
                "{} {} {}:{}:{}  {}\n",
                f.rule_id,
                sev,
                f.location.file.display(),
                f.location.line,
                f.location.column,
                f.message,
            ));
        }
        let (e, w, i) = count_by_severity(&self.result.findings);
        out.push_str(&format!(
            "\n发现 {} 条问题（{} 错误 / {} 警告 / {} 信息） | Found {} findings ({} errors, {} warnings, {} infos) in {} files",
            self.result.findings.len(),
            e,
            w,
            i,
            self.result.findings.len(),
            e,
            w,
            i,
            self.result.files_scanned,
        ));
        if self.result.files_skipped > 0 {
            out.push_str(&format!(
                "，跳过 {} 个 | , {} skipped",
                self.result.files_skipped, self.result.files_skipped
            ));
        }
        out.push('\n');
        for (file, err) in &self.result.parse_errors {
            out.push_str(&format!(
                "解析失败 {}：{} | parse failed {}: {}\n",
                file.display(),
                err,
                file.display(),
                err
            ));
        }
        out
    }

    pub fn render_json(&self) -> String {
        let (e, w, i) = count_by_severity(&self.result.findings);
        #[derive(serde::Serialize)]
        struct Out<'a> {
            summary: Summary,
            findings: &'a [crate::finding::Finding],
            parse_errors: Vec<ParseErrorOut>,
        }
        #[derive(serde::Serialize)]
        struct Summary {
            errors: usize,
            warnings: usize,
            infos: usize,
            files: usize,
            skipped: usize,
        }
        #[derive(serde::Serialize)]
        struct ParseErrorOut {
            file: std::path::PathBuf,
            error: String,
        }
        let out = Out {
            summary: Summary {
                errors: e,
                warnings: w,
                infos: i,
                files: self.result.files_scanned,
                skipped: self.result.files_skipped,
            },
            findings: &self.result.findings,
            parse_errors: self
                .result
                .parse_errors
                .iter()
                .map(|(f, err)| ParseErrorOut {
                    file: f.clone(),
                    error: err.to_string(),
                })
                .collect(),
        };
        serde_json::to_string_pretty(&out).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }
}

fn count_by_severity(findings: &[crate::finding::Finding]) -> (usize, usize, usize) {
    let mut e = 0;
    let mut w = 0;
    let mut i = 0;
    for f in findings {
        match f.severity {
            Severity::Error => e += 1,
            Severity::Warning => w += 1,
            Severity::Info => i += 1,
        }
    }
    (e, w, i)
}

fn colorize(severity: Severity) -> String {
    let code = match severity {
        Severity::Error => "31",
        Severity::Warning => "33",
        Severity::Info => "34",
    };
    format!("\x1b[{}m{}\x1b[0m", code, severity.display_label())
}
