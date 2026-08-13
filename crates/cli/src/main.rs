use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser as ClapParser, Subcommand};

use codereviewer_core::analyzer::Analyzer;
use codereviewer_core::config::Config;
use codereviewer_core::finding::Severity;
use codereviewer_core::reporter::Report;

#[derive(ClapParser)]
#[command(name = "codereviewer", version, about = "AI code review tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a path and report findings
    Check {
        /// Path to scan (file or directory)
        path: PathBuf,
        /// Output format: text, md (summary) or json (machine-readable)
        #[arg(long, default_value = "text")]
        format: String,
        /// Write the report to a file instead of stdout
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional config file path
        #[arg(long)]
        config: Option<PathBuf>,
        /// Only run these rules (comma-separated IDs, e.g. R01,R02)
        #[arg(long, value_delimiter = ',')]
        rules: Option<Vec<String>>,
        /// Minimum severity to show (error, warning, info)
        #[arg(long)]
        severity: Option<String>,
    },
    /// List all available rules
    ListRules,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            path,
            format,
            output,
            config,
            rules,
            severity,
        } => {
            if !matches!(format.as_str(), "text" | "json" | "md") {
                bail!(
                    "无效输出格式：{format}（可选 text/json/md） | invalid format: {format} (text/json/md)"
                );
            }
            let cfg = match config.as_deref() {
                Some(p) => Config::load_from_file(p).context("failed to load config")?,
                None => Config::load_auto_from(&path).context("failed to load project config")?,
            };
            let mut all_rules = codereviewer_rules::all_rules();
            if let Some(filter) = &rules {
                let unknown: Vec<&str> = filter
                    .iter()
                    .map(String::as_str)
                    .filter(|f| !all_rules.iter().any(|r| r.id() == *f))
                    .collect();
                if !unknown.is_empty() {
                    bail!(
                        "未知规则 ID：{} | unknown rule id(s): {}",
                        unknown.join(", "),
                        unknown.join(", ")
                    );
                }
                all_rules.retain(|r| filter.iter().any(|f| r.id() == f));
            }
            let min_severity = match severity.as_deref() {
                Some(s) => Some(
                    parse_severity(s).with_context(|| {
                        format!("无效严重级：{s}（可选 error/warning/info） | invalid severity: {s} (error/warning/info)")
                    })?,
                ),
                None => None,
            };
            let analyzer = Analyzer::new(all_rules, cfg);
            let mut result = analyzer.analyze_path(&path)?;
            if let Some(min) = min_severity {
                result.findings.retain(|f| f.severity <= min);
            }
            let report = Report { result };
            let rendered = match format.as_str() {
                "json" => report.render_json(),
                "md" => report.render_markdown(&path),
                _ => {
                    // 写文件时禁用颜色，避免 ANSI 码混入
                    let color =
                        output.is_none() && std::io::IsTerminal::is_terminal(&std::io::stdout());
                    if color {
                        report.render_text_colored()
                    } else {
                        report.render_text()
                    }
                }
            };
            if let Some(out_path) = &output {
                std::fs::write(out_path, rendered).context("failed to write report")?;
            } else {
                println!("{rendered}");
            }
            // 有 error 级 finding 时非零退出，供 CI 门禁使用
            if report
                .result
                .findings
                .iter()
                .any(|f| f.severity == Severity::Error)
            {
                std::process::exit(1);
            }
        }
        Command::ListRules => {
            for rule in codereviewer_rules::all_rules() {
                println!(
                    "{} {} [{}]",
                    rule.id(),
                    rule.name(),
                    rule.severity().display_label()
                );
            }
        }
    }
    Ok(())
}

fn parse_severity(s: &str) -> Option<Severity> {
    match s.to_lowercase().as_str() {
        "error" => Some(Severity::Error),
        "warning" => Some(Severity::Warning),
        "info" => Some(Severity::Info),
        _ => None,
    }
}
