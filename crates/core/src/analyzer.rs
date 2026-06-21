//! Analyzer: 调度规则，收集 finding。

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::finding::{Finding, Severity};
use crate::parser::{parse_file, ParseError};
use crate::rule::{AnalysisContext, Rule, RuleError};

pub struct AnalysisResult {
    pub findings: Vec<Finding>,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub parse_errors: Vec<(PathBuf, ParseError)>,
}

pub struct Analyzer {
    rules: Vec<Box<dyn Rule>>,
    config: Config,
}

impl Analyzer {
    pub fn new(rules: Vec<Box<dyn Rule>>, config: Config) -> Self {
        Self { rules, config }
    }

    pub fn analyze_path(&self, path: &Path) -> AnalysisResult {
        let mut findings = Vec::new();
        let mut files_scanned = 0usize;
        let mut files_skipped = 0usize;
        let mut parse_errors = Vec::new();

        let files = collect_files(path, &self.config);
        for file in files {
            match parse_file(&file) {
                Ok((tree, language)) => {
                    let source = match std::fs::read_to_string(&file) {
                        Ok(s) => s,
                        Err(_) => {
                            files_skipped += 1;
                            continue;
                        }
                    };
                    for rule in &self.rules {
                        if !self.rule_enabled(rule.id()) {
                            continue;
                        }
                        if !rule.languages().contains(&language) {
                            continue;
                        }
                        let default_config = crate::config::RuleConfig::default();
                        let rule_config = self.config.rules.get(rule.id()).unwrap_or(&default_config);
                        let ctx = AnalysisContext {
                            source: &source,
                            tree: &tree,
                            language,
                            file_path: &file,
                            rule_config,
                        };
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            rule.analyze(&ctx)
                        })) {
                            Ok(Ok(mut found)) => findings.append(&mut found),
                            Ok(Err(RuleError::Failed(msg))) => {
                                findings.push(self.rule_failure_finding(&**rule, &file, &msg));
                            }
                            Ok(Err(RuleError::Panic)) => {
                                findings.push(self.rule_failure_finding(&**rule, &file, "panicked"));
                            }
                            Err(_) => {
                                findings.push(self.rule_failure_finding(&**rule, &file, "panicked"));
                            }
                        }
                    }
                    files_scanned += 1;
                }
                Err(e) => {
                    parse_errors.push((file.clone(), e));
                    files_skipped += 1;
                }
            }
        }

        findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then(a.location.file.cmp(&b.location.file))
                .then(a.location.line.cmp(&b.location.line))
        });

        AnalysisResult {
            findings,
            files_scanned,
            files_skipped,
            parse_errors,
        }
    }

    fn rule_enabled(&self, rule_id: &str) -> bool {
        self.config
            .rules
            .get(rule_id)
            .map(|c| c.is_enabled())
            .unwrap_or(true)
    }

    fn rule_failure_finding(
        &self,
        rule: &dyn Rule,
        file: &Path,
        msg: &str,
    ) -> Finding {
        Finding {
            rule_id: rule.id(),
            rule_name: rule.name(),
            severity: Severity::Info,
            location: crate::finding::Location {
                file: file.to_path_buf(),
                line: 1,
                column: 1,
            },
            message: format!("rule failed: {msg}"),
            snippet: None,
        }
    }
}

fn collect_files(path: &Path, config: &Config) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut out = Vec::new();
    walk(path, config, &mut out);
    out
}

fn walk(dir: &Path, config: &Config, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if !is_excluded(&p, &config.global.exclude) {
                walk(&p, config, out);
            }
        } else if p.is_file() && crate::parser::Language::from_path(&p).is_some() {
            if !is_excluded(&p, &config.global.exclude) {
                out.push(p);
            }
        }
    }
}

fn is_excluded(path: &Path, patterns: &[String]) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let s = path.to_string_lossy();
    for pat in patterns {
        if s.contains(pat.trim_end_matches('/')) || name == *pat {
            return true;
        }
    }
    false
}
