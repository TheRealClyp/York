use std::fmt;

/// All tokens produced by the York lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Literals ──────────────────────────────────────────────
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),
    BoolLiteral(bool),
    NullLiteral,

    // ── Identifier ────────────────────────────────────────────
    Ident(String),

    // ── Keywords ──────────────────────────────────────────────
    Fn,
    Let,
    Var,
    Struct,
    Enum,
    Impl,
    Trait,
    Pub,
    Priv,
    Import,
    From,
    As,
    If,
    Else,
    While,
    For,
    In,
    Loop,
    Break,
    Continue,
    Return,
    Defer,
    Switch,
    Match,
    Thread,
    Arena,
    Packed,
    Dyn,
    Const,
    Static,
    Extern,
    Type,
    Sizeof,
    Alignof,
    Typeof,
    SelfType,
    This,
    New,
    Drop,

    // ── Primitive type keywords ───────────────────────────────
    TyI8,
    TyI16,
    TyI32,
    TyI64,
    TyI128,
    TyU8,
    TyU16,
    TyU32,
    TyU64,
    TyU128,
    TyF16,
    TyF32,
    TyF64,
    TyBool,
    TyChar,
    TyVoid,
    TyNever,
    TyString,

    // ── Operators ─────────────────────────────────────────────
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Bang,
    AmpEq,
    PipeEq,
    CaretEq,
    Eq,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LtLt,
    GtGt,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    And,
    Or,
    Arrow,
    FatArrow,
    Dot,
    DotDot,
    DotDotDot,
    Question,
    QuestionDot,
    QuestionBang,
    Colon,
    ColonColon,
    Semicolon,
    Comma,
    At,
    Hash,
    Dollar,

    // ── Delimiters ────────────────────────────────────────────
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // ── Special ───────────────────────────────────────────────
    Newline,
    Eof,
}

impl Token {
    /// Returns true if this token can start an expression.
    pub fn starts_expr(&self) -> bool {
        matches!(
            self,
            Token::IntLiteral(_)
                | Token::FloatLiteral(_)
                | Token::StringLiteral(_)
                | Token::CharLiteral(_)
                | Token::BoolLiteral(_)
                | Token::Ident(_)
                | Token::Bang
                | Token::Minus
                | Token::Star
                | Token::Amp
                | Token::LParen
                | Token::LBrace
                | Token::LBracket
                | Token::New
                | Token::Sizeof
                | Token::Alignof
                | Token::Typeof
                | Token::If
                | Token::Loop
                | Token::Fn
        )
    }

    /// Returns the precedence of this binary operator (higher = tighter binding).
    pub fn precedence(&self) -> u8 {
        match self {
            Token::Or => 1,
            Token::And => 2,
            Token::Pipe => 3,
            Token::Caret => 4,
            Token::Amp => 5,
            Token::EqEq | Token::BangEq => 6,
            Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => 7,
            Token::LtLt | Token::GtGt => 8,
            Token::Plus | Token::Minus => 9,
            Token::Star | Token::Slash | Token::Percent => 10,
            _ => 0,
        }
    }

    pub fn is_eof(&self) -> bool {
        matches!(self, Token::Eof)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntLiteral(v) => write!(f, "{v}"),
            Token::FloatLiteral(v) => write!(f, "{v}"),
            Token::StringLiteral(v) => write!(f, "\"{v}\""),
            Token::CharLiteral(v) => write!(f, "'{v}'"),
            Token::BoolLiteral(v) => write!(f, "{v}"),
            Token::NullLiteral => write!(f, "null"),
            Token::Ident(v) => write!(f, "{v}"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Var => write!(f, "var"),
            Token::Struct => write!(f, "struct"),
            Token::Enum => write!(f, "enum"),
            Token::Impl => write!(f, "impl"),
            Token::Trait => write!(f, "trait"),
            Token::Pub => write!(f, "pub"),
            Token::Priv => write!(f, "priv"),
            Token::Import => write!(f, "import"),
            Token::From => write!(f, "from"),
            Token::As => write!(f, "as"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::In => write!(f, "in"),
            Token::Loop => write!(f, "loop"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Return => write!(f, "return"),
            Token::Defer => write!(f, "defer"),
            Token::Switch => write!(f, "switch"),
            Token::Match => write!(f, "match"),
            Token::Thread => write!(f, "thread"),
            Token::Arena => write!(f, "arena"),
            Token::Packed => write!(f, "packed"),
            Token::Dyn => write!(f, "dyn"),
            Token::Const => write!(f, "const"),
            Token::Static => write!(f, "static"),
            Token::Extern => write!(f, "extern"),
            Token::Type => write!(f, "type"),
            Token::Sizeof => write!(f, "sizeof"),
            Token::Alignof => write!(f, "alignof"),
            Token::Typeof => write!(f, "typeof"),
            Token::SelfType => write!(f, "Self"),
            Token::This => write!(f, "this"),
            Token::New => write!(f, "new"),
            Token::Drop => write!(f, "drop"),
            Token::TyI8 => write!(f, "i8"),
            Token::TyI16 => write!(f, "i16"),
            Token::TyI32 => write!(f, "i32"),
            Token::TyI64 => write!(f, "i64"),
            Token::TyI128 => write!(f, "i128"),
            Token::TyU8 => write!(f, "u8"),
            Token::TyU16 => write!(f, "u16"),
            Token::TyU32 => write!(f, "u32"),
            Token::TyU64 => write!(f, "u64"),
            Token::TyU128 => write!(f, "u128"),
            Token::TyF16 => write!(f, "f16"),
            Token::TyF32 => write!(f, "f32"),
            Token::TyF64 => write!(f, "f64"),
            Token::TyBool => write!(f, "bool"),
            Token::TyChar => write!(f, "char"),
            Token::TyVoid => write!(f, "void"),
            Token::TyNever => write!(f, "!"),
            Token::TyString => write!(f, "string"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Amp => write!(f, "&"),
            Token::Pipe => write!(f, "|"),
            Token::Caret => write!(f, "^"),
            Token::Tilde => write!(f, "~"),
            Token::Bang => write!(f, "!"),
            Token::AmpEq => write!(f, "&="),
            Token::PipeEq => write!(f, "|="),
            Token::CaretEq => write!(f, "^="),
            Token::Eq => write!(f, "="),
            Token::EqEq => write!(f, "=="),
            Token::BangEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::LtLt => write!(f, "<<"),
            Token::GtGt => write!(f, ">>"),
            Token::PlusEq => write!(f, "+="),
            Token::MinusEq => write!(f, "-="),
            Token::StarEq => write!(f, "*="),
            Token::SlashEq => write!(f, "/="),
            Token::PercentEq => write!(f, "%="),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::Dot => write!(f, "."),
            Token::DotDot => write!(f, ".."),
            Token::DotDotDot => write!(f, "..."),
            Token::Question => write!(f, "?"),
            Token::QuestionDot => write!(f, "?."),
            Token::QuestionBang => write!(f, "!"),
            Token::Colon => write!(f, ":"),
            Token::ColonColon => write!(f, "::"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::At => write!(f, "@"),
            Token::Hash => write!(f, "#"),
            Token::Dollar => write!(f, "$"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Newline => write!(f, "\\n"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

/// A token paired with its source span.
pub type SpannedToken = crate::span::Spanned<Token>;
