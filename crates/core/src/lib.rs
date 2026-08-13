//! CodeReviewer core engine: parsing, rules, findings, reporting.

pub mod analyzer;
pub mod ast;
pub mod config;
pub mod diff;
pub mod finding;
pub mod parser;
pub mod reporter;
pub mod rule;

#[cfg(test)]
mod tests {
    #[test]
    fn version_matches_version_md() {
        // 版本号单一维护点：Cargo.toml（编译事实来源）与 VERSION.md 必须一致
        let declared = include_str!("../../../VERSION.md").trim();
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            declared,
            "Cargo.toml version and VERSION.md are out of sync"
        );
    }
}
