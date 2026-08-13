//! 基线对比：过滤掉与上次报告重复的 finding。

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;

use crate::finding::Finding;

/// 基线报告的 JSON 形状（只取对比所需字段）。
#[derive(Deserialize)]
struct BaselineReport {
    findings: Vec<BaselineFinding>,
}

#[derive(Deserialize)]
struct BaselineFinding {
    rule_id: String,
    location: BaselineLocation,
}

#[derive(Deserialize)]
struct BaselineLocation {
    file: std::path::PathBuf,
    line: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("failed to read baseline: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse baseline: {0}")]
    Parse(#[from] serde_json::Error),
}

/// 加载基线报告（`--format json` 产出的文件），返回 (file, line, rule_id) 集合。
pub fn load_baseline(path: &Path) -> Result<HashSet<(String, usize, String)>, DiffError> {
    let text = std::fs::read_to_string(path)?;
    let report: BaselineReport = serde_json::from_str(&text)?;
    Ok(report
        .findings
        .into_iter()
        .map(|f| {
            (
                f.location.file.to_string_lossy().into_owned(),
                f.location.line,
                f.rule_id,
            )
        })
        .collect())
}

/// 只保留基线中不存在的 finding（同文件同行同规则视为已存在）。
pub fn retain_new(findings: &mut Vec<Finding>, baseline: &HashSet<(String, usize, String)>) {
    findings.retain(|f| {
        !baseline.contains(&(
            f.location.file.to_string_lossy().into_owned(),
            f.location.line,
            f.rule_id.to_string(),
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::{Location, Severity};

    fn finding(rule_id: &'static str, file: &str, line: usize) -> Finding {
        Finding {
            rule_id,
            rule_name: "r",
            severity: Severity::Warning,
            location: Location {
                file: Path::new(file).to_path_buf(),
                line,
                column: 1,
            },
            message: "m".to_string(),
            snippet: None,
        }
    }

    #[test]
    fn retain_new_keeps_only_new_findings() {
        let mut current = vec![
            finding("R01", "a.cs", 1),
            finding("R01", "a.cs", 2),
            finding("R07", "a.cs", 1),
            finding("R07", "b.cs", 5),
        ];
        let baseline: HashSet<(String, usize, String)> = vec![
            ("a.cs".to_string(), 1, "R01".to_string()),
            ("a.cs".to_string(), 2, "R01".to_string()),
            ("a.cs".to_string(), 1, "R07".to_string()),
        ]
        .into_iter()
        .collect();
        retain_new(&mut current, &baseline);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].rule_id, "R07");
        assert_eq!(current[0].location.file, Path::new("b.cs"));
    }

    #[test]
    fn load_baseline_parses_report_json() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "codereviewer-baseline-test-{}.json",
            std::process::id()
        ));
        let json = r#"{
            "summary": {"errors": 1, "warnings": 0, "infos": 0, "files": 1, "skipped": 0},
            "findings": [
                {"rule_id": "R01", "rule_name": "r", "severity": "error",
                 "location": {"file": "a.cs", "line": 3, "column": 1},
                 "message": "m", "snippet": null}
            ],
            "parse_errors": []
        }"#;
        std::fs::write(&path, json).unwrap();
        let base = load_baseline(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert!(base.contains(&("a.cs".to_string(), 3, "R01".to_string())));
    }
}
