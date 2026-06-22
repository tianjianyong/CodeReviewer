//! 集成测试：在 fixtures 上跑完整 analyze_path，断言 finding 数量与类型。

use std::path::PathBuf;

use codereviewer_core::analyzer::Analyzer;
use codereviewer_core::config::Config;
use codereviewer_core::finding::Severity;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_sample_rust_finds_bloat() {
    let rules = codereviewer_rules::all_rules();
    let analyzer = Analyzer::new(rules, Config::default());
    let path = fixtures_dir().join("sample_rust.rs");
    let result = analyzer.analyze_path(&path);

    let r02: Vec<_> = result.findings.iter().filter(|f| f.rule_id == "R02").collect();
    assert!(!r02.is_empty(), "should find R02 bloat in sample_rust.rs");
    assert!(r02.iter().all(|f| f.severity == Severity::Warning));
}

#[test]
fn test_problematic_finds_multiple_rule_types() {
    let rules = codereviewer_rules::all_rules();
    let analyzer = Analyzer::new(rules, Config::default());
    let path = fixtures_dir().join("problematic.rs");
    let result = analyzer.analyze_path(&path);

    let rule_ids: Vec<&str> = result.findings.iter().map(|f| f.rule_id).collect();
    assert!(rule_ids.contains(&"R01"), "should find R01 fallback");
    assert!(rule_ids.contains(&"R03"), "should find R03 missing doc");
    assert!(rule_ids.contains(&"R06"), "should find R06 over-engineering");
    assert!(rule_ids.contains(&"R08"), "should find R08 TODO");
    assert!(rule_ids.contains(&"R09"), "should find R09 commented code");
    // R10 is skipped because fixture is under tests/ directory
    assert!(!rule_ids.contains(&"R10"), "R10 should skip test files");
}

#[test]
fn test_r10_skips_test_files() {
    let rules = codereviewer_rules::all_rules();
    let analyzer = Analyzer::new(rules, Config::default());
    // problematic.rs is under tests/fixtures/ -> R10 should be skipped
    let path = fixtures_dir().join("problematic.rs");
    let result = analyzer.analyze_path(&path);
    let r10_count = result.findings.iter().filter(|f| f.rule_id == "R10").count();
    assert_eq!(r10_count, 0, "R10 should skip files under tests/ directory");
}

#[test]
fn test_problematic_has_errors() {
    let rules = codereviewer_rules::all_rules();
    let analyzer = Analyzer::new(rules, Config::default());
    let path = fixtures_dir().join("problematic.rs");
    let result = analyzer.analyze_path(&path);

    let errors: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    assert!(!errors.is_empty(), "should have at least one error");
    assert!(errors.iter().all(|f| f.rule_id == "R01"));
}

#[test]
fn test_files_scanned_count() {
    let rules = codereviewer_rules::all_rules();
    let analyzer = Analyzer::new(rules, Config::default());
    let path = fixtures_dir().join("problematic.rs");
    let result = analyzer.analyze_path(&path);
    assert_eq!(result.files_scanned, 1);
    assert_eq!(result.files_skipped, 0);
}

#[test]
fn test_list_rules_returns_all() {
    let rules = codereviewer_rules::all_rules();
    assert_eq!(rules.len(), 19, "should have exactly 19 rules");
}

#[test]
fn test_json_output_format() {
    let rules = codereviewer_rules::all_rules();
    let analyzer = Analyzer::new(rules, Config::default());
    let path = fixtures_dir().join("problematic.rs");
    let result = analyzer.analyze_path(&path);
    let report = codereviewer_core::reporter::Report { result };
    let json = report.render_json();

    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON should parse");
    assert!(parsed["summary"]["errors"].as_u64().unwrap() >= 1);
    assert!(parsed["summary"]["files"].as_u64().unwrap() == 1);
    assert!(parsed["findings"].is_array());
    assert!(parsed["findings"].as_array().unwrap().len() > 0);

    for f in parsed["findings"].as_array().unwrap() {
        assert!(f["rule_id"].is_string());
        assert!(f["severity"].is_string());
        assert!(f["location"]["file"].is_string());
        assert!(f["location"]["line"].is_u64());
    }
}
