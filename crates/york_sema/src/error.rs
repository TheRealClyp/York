use thiserror::Error;
use york_ast::span::Span;

#[derive(Debug, Clone, Error)]
pub enum SemanticError {
    #[error("unknown type `{0}`")]
    UnknownType(String),

    #[error("undefined variable `{0}`")]
    UndefinedVariable(String),

    #[error("undefined function `{0}`")]
    UndefinedFunction(String),

    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("cannot call non-function `{0}`")]
    NotCallable(String),

    #[error("no method `{method}` on type `{ty}`")]
    NoMethod { method: String, ty: String },

    #[error("too many/few arguments: expected {expected}, got {got}")]
    ArgCount { expected: usize, got: usize },

    #[error("cannot assign to immutable variable `{0}")]
    AssignImmutable(String),

    #[error("cannot index into type `{0}`")]
    NotIndexable(String),

    #[error("fixed array length mismatch: expected {expected}, found {found}")]
    ArrayLength { expected: usize, found: usize },

    #[error("`{0}` is not supported yet")]
    NotSupported(String),
}

#[derive(Debug, Clone)]
pub struct SemanticErrorWithSpan {
    pub error: SemanticError,
    pub span: Span,
}

impl std::fmt::Display for SemanticErrorWithSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at {}..{}: {}", self.span.lo, self.span.hi, self.error)
    }
}
