pub mod error;

use std::collections::HashMap;
use std::sync::LazyLock;

use york_ast::span::{BytePos, Spanned};
use york_ast::Token as Tok;

use crate::error::{LexError, LexErrorWithSpan};

static KEYWORDS: LazyLock<HashMap<&'static str, Tok>> = LazyLock::new(|| {
    use Tok::*;
    let mut m = HashMap::new();
    // Control flow
    m.insert("fn", Fn);
    m.insert("let", Let);
    m.insert("var", Var);
    m.insert("if", If);
    m.insert("else", Else);
    m.insert("while", While);
    m.insert("for", For);
    m.insert("in", In);
    m.insert("loop", Loop);
    m.insert("break", Break);
    m.insert("continue", Continue);
    m.insert("return", Return);
    m.insert("defer", Defer);
    m.insert("switch", Switch);
    m.insert("match", Match);

    // Declarations
    m.insert("struct", Struct);
    m.insert("enum", Enum);
    m.insert("impl", Impl);
    m.insert("trait", Trait);
    m.insert("pub", Pub);
    m.insert("priv", Priv);
    m.insert("public", Pub);
    m.insert("private", Priv);
    m.insert("import", Import);
    m.insert("from", From);
    m.insert("as", As);
    m.insert("const", Const);
    m.insert("static", Static);
    m.insert("extern", Extern);
    m.insert("type", Type);

    // Type modifiers
    m.insert("packed", Packed);
    m.insert("dyn", Dyn);
    m.insert("thread", Thread);
    m.insert("arena", Arena);

    // Intrinsics
    m.insert("sizeof", Sizeof);
    m.insert("alignof", Alignof);
    m.insert("typeof", Typeof);
    m.insert("Self", SelfType);
    m.insert("this", This);
    m.insert("new", New);
    m.insert("drop", Drop);
    m.insert("true", BoolLiteral(true));
    m.insert("false", BoolLiteral(false));
    m.insert("null", NullLiteral);

    // Primitive types
    m.insert("i8", TyI8);
    m.insert("i16", TyI16);
    m.insert("i32", TyI32);
    m.insert("i64", TyI64);
    m.insert("i128", TyI128);
    m.insert("u8", TyU8);
    m.insert("u16", TyU16);
    m.insert("u32", TyU32);
    m.insert("u64", TyU64);
    m.insert("u128", TyU128);
    m.insert("f16", TyF16);
    m.insert("f32", TyF32);
    m.insert("f64", TyF64);
    m.insert("bool", TyBool);
    m.insert("boolean", TyBool);
    m.insert("char", TyChar);
    m.insert("void", TyVoid);
    m.insert("string", TyString);
    m.insert("String", TyString);
    // Java-style primitive aliases
    m.insert("int", TyI32);
    m.insert("long", TyI64);
    m.insert("short", TyI16);
    m.insert("byte", TyI8);
    m.insert("float", TyF32);
    m.insert("double", TyF64);
    m
});

/// Result of a single tokenization pass.
pub struct LexResult {
    pub tokens: Vec<Spanned<Tok>>,
    pub errors: Vec<LexErrorWithSpan>,
}

/// Tokenizes York source code.
pub fn lex(source: &str) -> LexResult {
    let mut lexer = Lexer {
        src: source,
        pos: 0,
        tokens: Vec::new(),
        errors: Vec::new(),
    };
    lexer.run();
    LexResult {
        tokens: lexer.tokens,
        errors: lexer.errors,
    }
}

struct Lexer<'a> {
    src: &'a str,
    /// Byte offset into `src`.
    pos: usize,
    tokens: Vec<Spanned<Tok>>,
    errors: Vec<LexErrorWithSpan>,
}

impl<'a> Lexer<'a> {
    fn run(&mut self) {
        loop {
            self.skip_whitespace();
            let start = self.pos;
            let Some(ch) = self.peek_char() else { break };

            // Comments
            if ch == '/' {
                match self.peek_next_char() {
                    Some('/') => {
                        self.pos += 2;
                        while let Some(c) = self.peek_char() {
                            if c == '\n' {
                                break;
                            }
                            self.pos += c.len_utf8();
                        }
                        continue;
                    }
                    Some('*') => {
                        self.pos += 2;
                        self.lex_block_comment();
                        continue;
                    }
                    _ => {}
                }
            }

            let token = match ch {
                'a'..='z' | 'A'..='Z' | '_' => self.lex_ident(),
                '0'..='9' => self.lex_number(start),
                '"' => self.lex_string(start),
                '\'' => self.lex_char_literal(start),
                _ => self.lex_special(),
            };

            let span = self.make_span(start, self.pos);
            self.tokens.push(Spanned::new(token, span));
        }

        self.tokens.push(Spanned::new(Tok::Eof, self.make_span(self.pos, self.pos)));
    }

    fn lex_ident(&mut self) -> Tok {
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.pos += ch.len_utf8();
            } else {
                break;
            }
        }
        let text = &self.src[start..self.pos];
        if let Some(tok) = KEYWORDS.get(text) {
            tok.clone()
        } else {
            Tok::Ident(text.to_string())
        }
    }

    fn lex_number(&mut self, start: usize) -> Tok {
        // Hex / binary / octal prefixes
        if self.peek_char() == Some('0') {
            match self.peek_next_char() {
                Some('x') | Some('X') => {
                    self.pos += 2;
                    let digits_start = self.pos;
                    while let Some(c) = self.peek_char() {
                        if c.is_ascii_hexdigit() || c == '_' {
                            self.pos += c.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let digits = &self.src[digits_start..self.pos];
                    let clean: String = digits.chars().filter(|c| *c != '_').collect();
                    match i64::from_str_radix(&clean, 16) {
                        Ok(v) => return Tok::IntLiteral(v),
                        Err(_) => {
                            self.record_int_error(digits.to_string(), start, self.pos);
                            return Tok::IntLiteral(0);
                        }
                    }
                }
                Some('b') | Some('B') => {
                    self.pos += 2;
                    let digits_start = self.pos;
                    while let Some(c) = self.peek_char() {
                        if c == '0' || c == '1' || c == '_' {
                            self.pos += c.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let digits = &self.src[digits_start..self.pos];
                    let clean: String = digits.chars().filter(|c| *c != '_').collect();
                    match i64::from_str_radix(&clean, 2) {
                        Ok(v) => return Tok::IntLiteral(v),
                        Err(_) => {
                            self.record_int_error(digits.to_string(), start, self.pos);
                            return Tok::IntLiteral(0);
                        }
                    }
                }
                Some('o') | Some('O') => {
                    self.pos += 2;
                    let digits_start = self.pos;
                    while let Some(c) = self.peek_char() {
                        if c.is_digit(8) || c == '_' {
                            self.pos += c.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let digits = &self.src[digits_start..self.pos];
                    let clean: String = digits.chars().filter(|c| *c != '_').collect();
                    match i64::from_str_radix(&clean, 8) {
                        Ok(v) => return Tok::IntLiteral(v),
                        Err(_) => {
                            self.record_int_error(digits.to_string(), start, self.pos);
                            return Tok::IntLiteral(0);
                        }
                    }
                }
                _ => {}
            }
        }

        let mut is_float = false;

        // Integer part
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }

        // Fraction part (only if a digit follows the dot)
        if self.peek_char() == Some('.')
            && self.peek_char_n(1).is_some_and(|c| c.is_ascii_digit())
        {
            is_float = true;
            self.pos += 1;
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() || c == '_' {
                    self.pos += c.len_utf8();
                } else {
                    break;
                }
            }
        }

        // Exponent
        if matches!(self.peek_char(), Some('e') | Some('E')) {
            let save = self.pos;
            self.pos += 1;
            if matches!(self.peek_char(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            if self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                is_float = true;
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_digit() || c == '_' {
                        self.pos += c.len_utf8();
                    } else {
                        break;
                    }
                }
            } else {
                self.pos = save;
            }
        }

        // Numeric suffix: `5f`, `2.5F`, `1000L`, `3.14d`
        if matches!(self.peek_char(), Some('f') | Some('F') | Some('d') | Some('D')) {
            is_float = true;
            self.pos += self.peek_char().unwrap().len_utf8();
        } else if matches!(self.peek_char(), Some('l') | Some('L') | Some('u') | Some('U')) {
            self.pos += self.peek_char().unwrap().len_utf8();
        }

        let text = &self.src[start..self.pos];
        let mut clean: String = text.chars().filter(|c| *c != '_').collect();
        if let Some(last) = clean.chars().last() {
            if matches!(last, 'f' | 'F' | 'd' | 'D' | 'l' | 'L' | 'u' | 'U') {
                clean.pop();
            }
        }

        if is_float {
            match clean.parse::<f64>() {
                Ok(v) => Tok::FloatLiteral(v),
                Err(_) => {
                    self.record_float_error(clean, start, self.pos);
                    Tok::FloatLiteral(0.0)
                }
            }
        } else {
            match clean.parse::<i64>() {
                Ok(v) => Tok::IntLiteral(v),
                Err(_) => {
                    self.record_int_error(text.to_string(), start, self.pos);
                    Tok::IntLiteral(0)
                }
            }
        }
    }

    fn lex_string(&mut self, start: usize) -> Tok {
        self.pos += 1; // consume opening quote
        let mut value = String::new();
        loop {
            let Some(ch) = self.peek_char() else {
                self.errors.push(LexErrorWithSpan {
                    error: LexError::UnterminatedString {
                        start: BytePos(start as u32),
                    },
                    span: self.make_span(start, self.pos),
                });
                return Tok::StringLiteral(value);
            };
            match ch {
                '"' => {
                    self.pos += 1;
                    return Tok::StringLiteral(value);
                }
                '\\' => {
                    self.pos += 1;
                    match self.peek_char() {
                        Some('n') => {
                            value.push('\n');
                            self.pos += 1;
                        }
                        Some('r') => {
                            value.push('\r');
                            self.pos += 1;
                        }
                        Some('t') => {
                            value.push('\t');
                            self.pos += 1;
                        }
                        Some('\\') => {
                            value.push('\\');
                            self.pos += 1;
                        }
                        Some('"') => {
                            value.push('"');
                            self.pos += 1;
                        }
                        Some('\'') => {
                            value.push('\'');
                            self.pos += 1;
                        }
                        Some('0') => {
                            value.push('\0');
                            self.pos += 1;
                        }
                        Some(c) => {
                            let err_start = self.pos;
                            self.pos += c.len_utf8();
                            self.errors.push(LexErrorWithSpan {
                                error: LexError::InvalidEscape(c),
                                span: self.make_span(err_start, self.pos),
                            });
                            value.push(c);
                        }
                        None => {
                            self.errors.push(LexErrorWithSpan {
                                error: LexError::UnterminatedString {
                                    start: BytePos(start as u32),
                                },
                                span: self.make_span(start, self.pos),
                            });
                            return Tok::StringLiteral(value);
                        }
                    }
                }
                c => {
                    value.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
    }

    fn lex_char_literal(&mut self, start: usize) -> Tok {
        self.pos += 1; // consume opening quote
        let Some(ch) = self.peek_char() else {
            self.errors.push(LexErrorWithSpan {
                error: LexError::UnterminatedChar {
                    start: BytePos(start as u32),
                },
                span: self.make_span(start, self.pos),
            });
            return Tok::CharLiteral('\0');
        };

        let value = if ch == '\\' {
            self.pos += 1;
            match self.peek_char() {
                Some('n') => {
                    self.pos += 1;
                    '\n'
                }
                Some('r') => {
                    self.pos += 1;
                    '\r'
                }
                Some('t') => {
                    self.pos += 1;
                    '\t'
                }
                Some('\\') => {
                    self.pos += 1;
                    '\\'
                }
                Some('\'') => {
                    self.pos += 1;
                    '\''
                }
                Some('0') => {
                    self.pos += 1;
                    '\0'
                }
                Some(c) => {
                    let err_start = self.pos;
                    self.pos += c.len_utf8();
                    self.errors.push(LexErrorWithSpan {
                        error: LexError::InvalidEscape(c),
                        span: self.make_span(err_start, self.pos),
                    });
                    c
                }
                None => {
                    self.errors.push(LexErrorWithSpan {
                        error: LexError::UnterminatedChar {
                            start: BytePos(start as u32),
                        },
                        span: self.make_span(start, self.pos),
                    });
                    return Tok::CharLiteral('\0');
                }
            }
        } else {
            self.pos += ch.len_utf8();
            ch
        };

        if self.peek_char() == Some('\'') {
            self.pos += 1;
            Tok::CharLiteral(value)
        } else {
            self.errors.push(LexErrorWithSpan {
                error: LexError::UnterminatedChar {
                    start: BytePos(start as u32),
                },
                span: self.make_span(start, self.pos),
            });
            Tok::CharLiteral(value)
        }
    }

    fn lex_block_comment(&mut self) {
        let mut depth = 1usize;
        while let Some(ch) = self.peek_char() {
            if ch == '/' && self.peek_next_char() == Some('*') {
                depth += 1;
                self.pos += 2;
            } else if ch == '*' && self.peek_next_char() == Some('/') {
                depth -= 1;
                self.pos += 2;
                if depth == 0 {
                    return;
                }
            } else {
                self.pos += ch.len_utf8();
            }
        }
    }

    fn lex_special(&mut self) -> Tok {
        let ch = self.peek_char().unwrap();
        let next = self.peek_char_n(1);
        self.pos += ch.len_utf8();

        macro_rules! two {
            ($t:expr) => {{
                self.pos += 1;
                $t
            }};
        }

        match ch {
            '+' => {
                if next == Some('=') {
                    two!(Tok::PlusEq)
                } else {
                    Tok::Plus
                }
            }
            '-' => match next {
                Some('=') => two!(Tok::MinusEq),
                Some('>') => two!(Tok::Arrow),
                _ => Tok::Minus,
            },
            '*' => {
                if next == Some('=') {
                    two!(Tok::StarEq)
                } else {
                    Tok::Star
                }
            }
            '/' => {
                if next == Some('=') {
                    two!(Tok::SlashEq)
                } else {
                    Tok::Slash
                }
            }
            '%' => {
                if next == Some('=') {
                    two!(Tok::PercentEq)
                } else {
                    Tok::Percent
                }
            }
            '&' => match next {
                Some('&') => two!(Tok::And),
                Some('=') => two!(Tok::AmpEq),
                _ => Tok::Amp,
            },
            '|' => match next {
                Some('|') => two!(Tok::Or),
                Some('=') => two!(Tok::PipeEq),
                _ => Tok::Pipe,
            },
            '^' => {
                if next == Some('=') {
                    two!(Tok::CaretEq)
                } else {
                    Tok::Caret
                }
            }
            '~' => Tok::Tilde,
            '=' => match next {
                Some('=') => two!(Tok::EqEq),
                Some('>') => two!(Tok::FatArrow),
                _ => Tok::Eq,
            },
            '!' => match next {
                Some('=') => two!(Tok::BangEq),
                Some('?') => two!(Tok::QuestionBang),
                _ => Tok::Bang,
            },
            '<' => match next {
                Some('<') => {
                    self.pos += 1;
                    if self.peek_char() == Some('=') {
                        self.pos += 1;
                        Tok::LtEq
                    } else {
                        Tok::LtLt
                    }
                }
                Some('=') => two!(Tok::LtEq),
                _ => Tok::Lt,
            },
            '>' => match next {
                Some('>') => {
                    self.pos += 1;
                    if self.peek_char() == Some('=') {
                        self.pos += 1;
                        Tok::GtEq
                    } else {
                        Tok::GtGt
                    }
                }
                Some('=') => two!(Tok::GtEq),
                _ => Tok::Gt,
            },
            '.' => match next {
                Some('.') => {
                    self.pos += 1;
                    if self.peek_char() == Some('.') {
                        self.pos += 1;
                        Tok::DotDotDot
                    } else {
                        Tok::DotDot
                    }
                }
                _ => Tok::Dot,
            },
            '?' => {
                if next == Some('.') {
                    two!(Tok::QuestionDot)
                } else {
                    Tok::Question
                }
            }
            ':' => {
                if next == Some(':') {
                    two!(Tok::ColonColon)
                } else {
                    Tok::Colon
                }
            }
            ';' => Tok::Semicolon,
            ',' => Tok::Comma,
            '@' => Tok::At,
            '#' => Tok::Hash,
            '$' => Tok::Dollar,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            _ => {
                let err_start = self.pos - ch.len_utf8();
                self.errors.push(LexErrorWithSpan {
                    error: LexError::UnexpectedChar(ch),
                    span: self.make_span(err_start, self.pos),
                });
                Tok::Ident(String::new())
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == '\u{feff}' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn record_int_error(&mut self, text: String, start: usize, end: usize) {
        self.errors.push(LexErrorWithSpan {
            error: LexError::InvalidInt(text),
            span: self.make_span(start, end),
        });
    }

    fn record_float_error(&mut self, text: String, start: usize, end: usize) {
        self.errors.push(LexErrorWithSpan {
            error: LexError::InvalidFloat(text),
            span: self.make_span(start, end),
        });
    }

    // ── Character helpers ────────────────────────────────────

    fn peek_char(&self) -> Option<char> {
        self.peek_char_n(0)
    }

    fn peek_next_char(&self) -> Option<char> {
        self.peek_char_n(1)
    }

    fn peek_char_n(&self, n: usize) -> Option<char> {
        // `get(self.pos..)` returns None when `pos` isn't on a UTF-8 char
        // boundary, which protects all scanners from misalignment.
        self.src.get(self.pos..)?.char_indices().nth(n).map(|(_, c)| c)
    }

    fn make_span(&self, lo: usize, hi: usize) -> york_ast::span::Span {
        york_ast::span::Span {
            lo: BytePos(lo as u32),
            hi: BytePos(hi as u32),
        }
    }
}

#[allow(dead_code)]
fn decode_utf8_width(bytes: &[u8], pos: usize) -> Option<(usize, usize)> {
    let lead = *bytes.get(pos)?;
    let width = utf8_char_width(lead).min(bytes.len() - pos);
    if width == 0 {
        return None;
    }
    Some((width - 1, width))
}

#[allow(dead_code)]
fn utf8_char_width(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead < 0xE0 {
        2
    } else if lead < 0xF0 {
        3
    } else if lead < 0xF8 {
        4
    } else {
        1
    }
}