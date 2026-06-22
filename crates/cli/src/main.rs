use std::path::PathBuf;

use anyhow::{Context, Result};
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
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
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
        Command::Check { path, format, config, rules, severity } => {
            let cfg = load_config(config.as_deref())?;
            let mut all_rules = codereviewer_rules::all_rules();
            if let Some(filter) = &rules {
                all_rules.retain(|r| filter.iter().any(|f| r.id() == f));
            }
            let min_severity = severity.as_deref().and_then(parse_severity);
            let analyzer = Analyzer::new(all_rules, cfg);
            let mut result = analyzer.analyze_path(&path);
            if let Some(min) = min_severity {
                result.findings.retain(|f| f.severity <= min);
            }
            let report = Report { result };
            match format.as_str() {
                "json" => println!("{}", report.render_json()),
                _ => {
                    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
                    if is_tty {
                        println!("{}", report.render_text_colored());
                    } else {
                        println!("{}", report.render_text());
                    }
                }
            }
        }
        Command::ListRules => {
            for rule in codereviewer_rules::all_rules() {
                println!("{} {} [{}]", rule.id(), rule.name(), rule.severity().display_label());
            }
        }
    }
    Ok(())
}

fn load_config(path: Option<&std::path::Path>) -> Result<Config> {
    if let Some(p) = path {
        return Config::load_from_file(p).context("failed to load config");
    }
    if let Some(found) = find_project_config() {
        Config::load_from_file(&found).context("failed to load project config")
    } else {
        Ok(Config::default())
    }
}

fn find_project_config() -> Option<PathBuf> {
    let dir = std::env::current_dir().ok()?;
    let mut current = dir.as_path();
    loop {
        let candidate = current.join(".codereviewer.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

fn parse_severity(s: &str) -> Option<Severity> {
    match s.to_lowercase().as_str() {
        "error" => Some(Severity::Error),
        "warning" => Some(Severity::Warning),
        "info" => Some(Severity::Info),
        _ => None,
    }
}
