//! Reporter: 输出 finding，终端文本、Markdown 汇总或 JSON。

use std::path::Path;

use crate::analyzer::AnalysisResult;
use crate::finding::{Finding, Severity};

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

    /// Markdown 汇总报告：统计 + error 级问题按规则分组（附修复建议与重灾区文件）。
    pub fn render_markdown(&self, scan_path: &Path) -> String {
        let (e, w, i) = count_by_severity(&self.result.findings);
        let mut out = String::new();
        out.push_str("# CodeReviewer 报告\n\n");
        out.push_str(&format!("扫描路径：{}\n\n", scan_path.display()));
        out.push_str(&format!(
            "文件 {} 个（跳过 {} 个） | 共 {} 条问题\n\n",
            self.result.files_scanned,
            self.result.files_skipped,
            self.result.findings.len(),
        ));

        out.push_str("## 统计\n\n| 严重级 | 数量 |\n|---|---|\n");
        out.push_str(&format!(
            "| error | {} |\n| warning | {} |\n| info | {} |\n\n",
            e, w, i
        ));

        if !self.result.parse_errors.is_empty() {
            out.push_str(&format!(
                "## 解析失败（{} 个）\n\n",
                self.result.parse_errors.len()
            ));
            for (file, err) in &self.result.parse_errors {
                out.push_str(&format!("- {}：{}\n", file.display(), err));
            }
            out.push('\n');
        }

        out.push_str("## 最严重问题（error 级）\n\n");
        let errors: Vec<&Finding> = self
            .result
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .collect();
        if errors.is_empty() {
            out.push_str("无 error 级问题。\n\n");
        } else {
            for group in group_by_rule(&errors) {
                out.push_str(&format!(
                    "### {} {} — {} 处\n\n",
                    group.rule_id,
                    group.findings[0].rule_name,
                    group.findings.len(),
                ));
                let suggestion = suggestion_for(group.rule_id);
                if !suggestion.is_empty() {
                    out.push_str(&format!("**修复建议**：{}\n\n", suggestion));
                }
                out.push_str("涉及最多的文件：\n\n");
                for (file, count) in top_files(&group.findings, 5) {
                    out.push_str(&format!("- {} — {} 处\n", file, count));
                }
                out.push_str("\n示例：\n\n");
                for f in group.findings.iter().take(3) {
                    let snippet = f.snippet.as_deref().unwrap_or("").trim();
                    out.push_str(&format!(
                        "- {}:{} — {}\n",
                        f.location.file.display(),
                        f.location.line,
                        snippet,
                    ));
                }
                out.push('\n');
            }
        }

        out.push_str("## 其他问题一览\n\n| 规则 | 严重级 | 数量 |\n|---|---|---|\n");
        let others: Vec<&Finding> = self
            .result
            .findings
            .iter()
            .filter(|f| f.severity != Severity::Error)
            .collect();
        for group in group_by_rule_severity(&others) {
            out.push_str(&format!(
                "| {} {} | {} | {} |\n",
                group.rule_id,
                group.findings[0].rule_name,
                group.findings[0].severity.label(),
                group.findings.len(),
            ));
        }
        out
    }
}

/// 每条规则对应的修复建议（展示层内容，随规则演进同步维护）。
fn suggestion_for(rule_id: &str) -> &'static str {
    match rule_id {
        "R01" => "不要静默吞掉错误：至少记录日志；能上抛的上抛，或让返回值携带错误信息。",
        "R02" => "拆分长函数、降低嵌套、减少参数（传参对象），提高可读性与可测试性。",
        "R03" => {
            "为公开 API 补充文档注释；若团队文档集中在 docs/ 且不写内联注释，可在配置中禁用本规则。"
        }
        "R04" => "增加断言数量，覆盖边界与失败路径，而非只验证 happy path。",
        "R05" => "断言响应体与副作用，而非仅状态码。",
        "R06" => "评估抽象是否必要：单实现 trait/接口可先内联为具体类型。",
        "R07" => "删除未使用的 import。",
        "R08" => "清理 TODO/FIXME，或转成正式 issue 跟踪。",
        "R09" => "删除注释掉的代码，历史保留在版本控制中。",
        "R10" => "为魔法常量命名（常量/枚举），提高可维护性。",
        "R14" => "密钥移出源码（环境变量/密钥管理），已泄露的密钥应立即轮换。",
        "R15" => "对边界函数的输入做空值/越界校验。",
        "R16" => "测试名描述行为而非实现；避免直接断言私有状态。",
        "R18" => "给 async 调用补上 await，或显式处理返回的 Future。",
        "R19" => "用 select_related/prefetch_related/include 预取关系字段，避免 N+1。",
        "R20" => "为资源配对 close/remove，或使用 with/try-finally 保证释放。",
        "R23" => "保留错误类型信息：区分错误码/异常类型，让调用方能正确响应。",
        "R24" => "路径/URL 集中到配置或常量，避免散落与客户端/服务端不一致。",
        "R28" => "移除对不可失败操作的防御性处理，简化代码。",
        _ => "",
    }
}

struct RuleGroup<'a> {
    rule_id: &'a str,
    findings: Vec<&'a Finding>,
}

/// 按 rule_id 分组，保持首次出现顺序。
fn group_by_rule<'a>(findings: &[&'a Finding]) -> Vec<RuleGroup<'a>> {
    let mut groups: Vec<RuleGroup<'_>> = Vec::new();
    for f in findings {
        match groups.iter_mut().find(|g| g.rule_id == f.rule_id) {
            Some(g) => g.findings.push(f),
            None => groups.push(RuleGroup {
                rule_id: f.rule_id,
                findings: vec![f],
            }),
        }
    }
    groups
}

/// 按 (rule_id, severity) 分组，保持首次出现顺序。
fn group_by_rule_severity<'a>(findings: &[&'a Finding]) -> Vec<RuleGroup<'a>> {
    let mut groups: Vec<RuleGroup<'_>> = Vec::new();
    for f in findings {
        match groups
            .iter_mut()
            .find(|g| g.rule_id == f.rule_id && g.findings[0].severity == f.severity)
        {
            Some(g) => g.findings.push(f),
            None => groups.push(RuleGroup {
                rule_id: f.rule_id,
                findings: vec![f],
            }),
        }
    }
    groups
}

/// 组内按文件计数，取前 limit 个（文件路径取文件名展示）。
fn top_files(findings: &[&Finding], limit: usize) -> Vec<(String, usize)> {
    let mut files: Vec<(String, usize)> = Vec::new();
    for f in findings {
        let name = f
            .location
            .file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| f.location.file.to_string_lossy().into_owned());
        match files.iter_mut().find(|(n, _)| *n == name) {
            Some((_, c)) => *c += 1,
            None => files.push((name, 1)),
        }
    }
    files.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    files.truncate(limit);
    files
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::AnalysisResult;
    use crate::finding::Location;
    use std::path::PathBuf;

    fn sample_finding(rule_id: &'static str, severity: Severity, file: &str) -> Finding {
        Finding {
            rule_id,
            rule_name: "test-rule",
            severity,
            location: Location {
                file: PathBuf::from(file),
                line: 1,
                column: 1,
            },
            message: "m".to_string(),
            snippet: Some("catch (Exception ex)".to_string()),
        }
    }

    #[test]
    fn markdown_report_contains_stats_and_suggestions() {
        let result = AnalysisResult {
            findings: vec![
                sample_finding("R01", Severity::Error, "a.cs"),
                sample_finding("R01", Severity::Error, "b.cs"),
                sample_finding("R01", Severity::Error, "b.cs"),
                sample_finding("R07", Severity::Warning, "c.cs"),
            ],
            files_scanned: 10,
            files_skipped: 1,
            parse_errors: vec![],
        };
        let report = Report { result };
        let md = report.render_markdown(Path::new("src"));
        assert!(md.contains("# CodeReviewer 报告"));
        assert!(md.contains("| error | 3 |"));
        assert!(md.contains("| warning | 1 |"));
        assert!(md.contains("**修复建议**"));
        assert!(md.contains("R01 test-rule — 3 处"));
        assert!(md.contains("b.cs — 2 处"));
        assert!(md.contains("R07 test-rule | warning | 1 |"));
    }
}
