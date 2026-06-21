//! LlmReviewer trait: Phase 2 占位，MVP 不实现。

use crate::finding::Finding;

#[derive(Debug, Clone)]
pub struct ReviewVerdict {
    pub original: Finding,
    pub confirmed: bool,
    pub comment: String,
}

pub trait LlmReviewer: Send + Sync {
    fn review(&self, findings: &[Finding]) -> Result<Vec<ReviewVerdict>, LlmError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("not implemented")]
    NotImplemented,
}
