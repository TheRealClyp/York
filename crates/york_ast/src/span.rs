use std::fmt;

/// A byte offset into the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytePos(pub u32);

impl BytePos {
    pub const ZERO: Self = Self(0);

    pub fn offset(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for BytePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add<u32> for BytePos {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        Self(self.0 + rhs)
    }
}

impl std::ops::Sub<BytePos> for BytePos {
    type Output = u32;
    fn sub(self, rhs: BytePos) -> u32 {
        self.0 - rhs.0
    }
}

/// A span representing a contiguous range of source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub lo: BytePos,
    pub hi: BytePos,
}

impl Span {
    pub fn new(lo: BytePos, hi: BytePos) -> Self {
        Self { lo, hi }
    }

    pub fn to(self, other: Span) -> Self {
        Self {
            lo: self.lo,
            hi: other.hi,
        }
    }

    pub fn is_empty(self) -> bool {
        self.lo == self.hi
    }

    pub fn len(self) -> u32 {
        self.hi.0 - self.lo.0
    }

    pub fn source_text<'a>(self, source: &'a str) -> &'a str {
        &source[self.lo.offset()..self.hi.offset()]
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.lo, self.hi)
    }
}

/// An AST node wrapped with its source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

impl<T: fmt::Display> fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.node.fmt(f)
    }
}

/// A unique identifier for a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

impl FileId {
    pub const MAIN: Self = Self(0);
}

/// A fully qualified source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub file: FileId,
    pub span: Span,
}

impl SourceLocation {
    pub fn new(file: FileId, span: Span) -> Self {
        Self { file, span }
    }
}
