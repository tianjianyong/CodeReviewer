//! Built-in rule set for CodeReviewer.

mod r01_fallback;
mod r02_bloat;
mod r03_missing_doc;
mod r04_simple_unit_test;
mod r05_shallow_integration;
mod r06_over_engineering;
mod r07_dead_code;
mod r08_todo;
mod r09_commented_code;
mod r10_magic_number;

pub use r01_fallback::FallbackMasksError;
pub use r02_bloat::StructuralBloat;
pub use r03_missing_doc::MissingDoc;
pub use r04_simple_unit_test::SimpleUnitTest;
pub use r05_shallow_integration::ShallowIntegration;
pub use r06_over_engineering::OverEngineering;
pub use r07_dead_code::DeadCode;
pub use r08_todo::TodoFixme;
pub use r09_commented_code::CommentedCode;
pub use r10_magic_number::MagicNumber;

use codereviewer_core::rule::Rule;

pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(FallbackMasksError),
        Box::new(StructuralBloat::default()),
        Box::new(MissingDoc),
        Box::new(SimpleUnitTest),
        Box::new(ShallowIntegration),
        Box::new(OverEngineering),
        Box::new(DeadCode),
        Box::new(TodoFixme),
        Box::new(CommentedCode),
        Box::new(MagicNumber),
    ]
}
