//! 语言识别与 tree-sitter 解析封装。

use std::path::Path;

use thiserror::Error;
use tree_sitter::{Language as TsLanguage, Parser, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    TypeScriptTsx,
    CSharp,
    Java,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "ts" => Some(Language::TypeScript),
            "tsx" => Some(Language::TypeScriptTsx),
            "cs" => Some(Language::CSharp),
            "java" => Some(Language::Java),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Language> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_extension)
    }

    pub fn tree_sitter_language(self) -> TsLanguage {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::TypeScriptTsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Language::Java => tree_sitter_java::LANGUAGE.into(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::TypeScriptTsx => "tsx",
            Language::CSharp => "csharp",
            Language::Java => "java",
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unsupported file extension: {0}")]
    UnsupportedExtension(String),
    #[error("failed to set parser language: {0}")]
    SetLanguage(String),
    #[error("parser returned no tree")]
    NoTree,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn parse(source: &str, language: Language) -> Result<Tree, ParseError> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.tree_sitter_language())
        .map_err(|e| ParseError::SetLanguage(e.to_string()))?;
    parser
        .parse(source, None)
        .ok_or(ParseError::NoTree)
}

pub fn parse_file(path: &Path) -> Result<(Tree, Language), ParseError> {
    let language = Language::from_path(path)
        .ok_or_else(|| ParseError::UnsupportedExtension(path.to_string_lossy().into_owned()))?;
    let source = std::fs::read_to_string(path)?;
    let tree = parse(&source, language)?;
    Ok((tree, language))
}
