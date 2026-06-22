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
mod r14_hardcoded_secret;
mod r15_missing_validation;
mod r16_self_validating_test;
mod r18_async_missing_await;
mod r19_n_plus_one;
mod r20_resource_leak;
mod r23_wrong_error_type;
mod r24_hardcoded_path;
mod r28_overly_defensive;

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
pub use r14_hardcoded_secret::HardcodedSecret;
pub use r15_missing_validation::MissingInputValidation;
pub use r16_self_validating_test::SelfValidatingTest;
pub use r18_async_missing_await::AsyncMissingAwait;
pub use r19_n_plus_one::NPlusOneQuery;
pub use r20_resource_leak::ResourceLeak;
pub use r23_wrong_error_type::WrongErrorTypePropagation;
pub use r24_hardcoded_path::HardcodedPathOrUrl;
pub use r28_overly_defensive::OverlyDefensiveHandling;

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
        Box::new(HardcodedSecret),
        Box::new(MissingInputValidation),
        Box::new(SelfValidatingTest),
        Box::new(AsyncMissingAwait),
        Box::new(NPlusOneQuery),
        Box::new(ResourceLeak),
        Box::new(WrongErrorTypePropagation),
        Box::new(HardcodedPathOrUrl),
        Box::new(OverlyDefensiveHandling),
    ]
}
