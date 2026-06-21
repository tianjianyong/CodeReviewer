//! CodeReviewer core engine: parsing, rules, findings, reporting.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod analyzer;
pub mod config;
pub mod finding;
pub mod llm;
pub mod parser;
pub mod reporter;
pub mod rule;
