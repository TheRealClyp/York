use thiserror::Error;
use york_ast::span::{BytePos, Span};

#[derive(Debug, Clone, Error)]
pub enum LexError {
    #[error("unexpected character {0:?}")]
    UnexpectedChar(char),

    #[error("unterminated string literal")]
    UnterminatedString {

        /// Position just before the opening quote.
        start: BytePos,
    },

    #[error("unterminated character literal")]
    UnterminatedChar { start: BytePos },

    #[error("invalid escape sequence in string: \\{0}")]
    InvalidEscape(char),

    #[error("invalid character literal: empty char literal")]
    EmptyChar,

    #[error("invalid integer literal {0:?}")]
    InvalidInt(String),

    #[error("invalid float literal {0:?}")]
    InvalidFloat(String),

    #[error("numeric literal is too large")]
    IntegerOverflow,
}

#[derive(Debug, Clone)]
pub struct LexErrorWithSpan {
    pub error: LexError,
    pub span: Span,
}

impl std::fmt::Display for LexErrorWithSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "lex error at {}:{}: {}",
            self.span.lo, self.span.hi, self.error
        )
    }
}