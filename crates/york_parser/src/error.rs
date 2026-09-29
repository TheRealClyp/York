use thiserror::Error;
use york_ast::span::{Span, Spanned};
use york_ast::Token;

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("expected {0}, found {1}")]
    Expected(&'static str, String),

    #[error("unexpected token {0}")]
    Unexpected(String),

    #[error("unexpected end of file")]
    UnexpectedEof,

    #[error("expected expression, found {0}")]
    ExpectedExpr(String),

    #[error("expected statement, found {0}")]
    ExpectedStmt(String),

    #[error("expected type annotation, found {0}")]
    ExpectedType(String),

    #[error("expected block body, found {0}")]
    ExpectedBlock(String),

    #[error("expected function name, found {0}")]
    ExpectedFnName(String),

    #[error("duplicate field `{0}` in struct/struct literal")]
    DuplicateField(String),

    #[error("floating-point literals are not allowed in array sizes")]
    FloatArraySize,

    #[error("type annotations cannot contain `mut` here")]
    UnexpectedMut,
}

impl ParseError {
    /// A human-readable category used for grouped diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            ParseError::UnexpectedEof => "unexpected end of file",
            _ => "parse error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseErrorWithSpan {
    pub error: ParseError,
    pub span: Span,
}

impl std::fmt::Display for ParseErrorWithSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "at {}..{}: {}",
            self.span.lo, self.span.hi, self.error
        )
    }
}

/// Convert a raw `&str` into a `Spanned<String>` identifier with the given span.
#[allow(dead_code)]
pub(crate) fn spanned_ident(name: String, span: Span) -> Spanned<String> {
    Spanned::new(name, span)
}

/// Retrieve the token text for diagnostics, without consuming.
pub(crate) fn token_name(tok: &Token) -> String {
    match tok {
        Token::Eof => "end of file".to_string(),
        Token::Newline => "newline".to_string(),
        other => format!("`{other}`"),
    }
}

pub(crate) fn span_for(first: Span, last: Span) -> Span {
    if first.lo <= last.hi {
        Span::new(first.lo, last.hi)
    } else {
        first
    }
}