use std::fmt;

use thiserror::Error;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        assert!(start <= end, "span start must not exceed end");

        Self { start, end }
    }

    pub fn extract<'a>(&self, source: &'a str) -> &'a str {
        let start = self.start as usize;
        let end = self.end as usize;

        assert!(
            end <= source.len(),
            "span end {} exceeds source length {}",
            self.end,
            source.len(),
        );

        assert!(
            source.is_char_boundary(start) && source.is_char_boundary(end),
            "span boundaries must align to UTF-8 character boundaries",
        );

        &source[start..end]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub position: u32,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.severity {
            Severity::Warning => "WARN",
            Severity::Error => "ERROR",
        };

        write!(
            formatter,
            "[{}] at position {}: {}",
            label,
            self.position,
            self.message,
        )
    }
}

#[derive(Error, Debug)]
pub enum LexError {
    #[error("Unterminated variable at position {0}")]
    UnterminatedVariable(u32),

    #[error("Unterminated block tag at position {0}")]
    UnterminatedBlock(u32),

    #[error("Unterminated comment at position {0}")]
    UnterminatedComment(u32),

    #[error("Unterminated string at position {0}")]
    UnterminatedString(u32),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected token at line {line}, col {col}: {message}")]
    UnexpectedToken {
        line: u32,
        col: u32,
        message: String,
    },

    #[error("Unclosed block tag: {tag}")]
    UnclosedBlock { tag: String },

    #[error("Mismatched end tag: expected {expected}, got {got}")]
    MismatchedEndTag { expected: String, got: String },

    #[error("Parse error: {0}")]
    Generic(String),
}

#[derive(Error, Debug)]
pub enum CompileError {
    #[error(transparent)]
    Lex(#[from] LexError),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("Circular inheritance detected: {template}")]
    CircularInheritance { template: String },

    #[error("Parent template not found: {path}")]
    ParentNotFound { path: String },

    #[error("Inheritance chain is empty for template: {template}")]
    EmptyInheritanceChain { template: String },

    #[error(
        "Inheritance chain exceeds maximum depth {max_depth} for template: {template}"
    )]
    InheritanceDepthExceeded { max_depth: u32, template: String },

    #[error("AST depth exceeds maximum {max_depth}")]
    AstDepthExceeded { max_depth: u32 },

    #[error("Node count exceeds maximum {max} for template: {template}")]
    NodeLimitExceeded { template: String, max: u32 },

    #[error(
        "Parsed template '{path}' produced {count} nodes, exceeds maximum {max}"
    )]
    ParsedNodeLimitExceeded { path: String, count: usize, max: u32 },

    #[error("Failed to read template '{path}': {message}")]
    TemplateRead { path: String, message: String },
}
