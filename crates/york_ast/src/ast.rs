use crate::span::{Span, Spanned};

// ─────────────────────────────────────────────────────────────
//  Program Structure
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Spanned<Item>>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    Enum(EnumDef),
    Impl(ImplBlock),
    Trait(TraitDef),
    Import(ImportDecl),
    Const(ConstDecl),
    Static(StaticDecl),
    TypeAlias(TypeAliasDecl),
}

// ─────────────────────────────────────────────────────────────
//  Functions
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Spanned<Param>>,
    pub return_type: Option<Spanned<TypeAnnotation>>,
    pub body: Option<Block>,
    pub is_extern: bool,
    pub is_thread: bool,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Spanned<String>,
    pub ty: Spanned<TypeAnnotation>,
    pub default: Option<Spanned<Expr>>,
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: Spanned<String>,
    pub bounds: Vec<Spanned<TypeAnnotation>>,
}

// ─────────────────────────────────────────────────────────────
//  Types
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    /// Primitive type keyword (i32, f64, bool, etc.)
    Primitive(PrimitiveType),
    /// Named type (Vec2, Sprite, etc.)
    Named(Path),
    /// Parameterized type (Arena<Player>, Vec<T>, ...)
    Generic {
        base: Path,
        args: Vec<Spanned<TypeAnnotation>>,
    },
    /// Pointer type (*T)
    Pointer(Box<Spanned<TypeAnnotation>>),
    /// Mutable pointer type (*mut T)
    MutPointer(Box<Spanned<TypeAnnotation>>),
    /// Reference type (&T)
    Reference(Box<Spanned<TypeAnnotation>>),
    /// Mutable reference type (&mut T)
    MutReference(Box<Spanned<TypeAnnotation>>),
    /// Array type [T; N]
    Array(Box<Spanned<TypeAnnotation>>, Spanned<Box<Expr>>),
    /// Slice type [T]
    Slice(Box<Spanned<TypeAnnotation>>),
    /// Tuple type (T1, T2, ...)
    Tuple(Vec<Spanned<TypeAnnotation>>),
    /// Optional type T?
    Optional(Box<Spanned<TypeAnnotation>>),
    /// Result type T!E
    Result(Box<Spanned<TypeAnnotation>>, Box<Spanned<TypeAnnotation>>),
    /// Function type fn(T1, T2) -> R
    Function(Vec<Spanned<TypeAnnotation>>, Box<Spanned<TypeAnnotation>>),
    /// Arena type arena(T)
    Arena(Box<Spanned<TypeAnnotation>>),
    /// Void
    Void,
    /// Never (!)
    Never,
    /// Inferred (_)
    Inferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    F16, F32, F64,
    Bool,
    Char,
    String,
}

impl PrimitiveType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "f16" => Some(Self::F16),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            "bool" => Some(Self::Bool),
            "char" => Some(Self::Char),
            "string" => Some(Self::String),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<Spanned<String>>,
}

// ─────────────────────────────────────────────────────────────
//  Structs & Enums
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StructDef {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<Spanned<FieldDef>>,
    pub is_packed: bool,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub ty: Spanned<TypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<Spanned<VariantDef>>,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: Spanned<String>,
    pub fields: Vec<Spanned<TypeAnnotation>>,
}

// ─────────────────────────────────────────────────────────────
//  Impl Blocks & Traits
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub self_type: Spanned<TypeAnnotation>,
    pub generics: Vec<GenericParam>,
    pub methods: Vec<Spanned<FunctionDef>>,
}

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<GenericParam>,
    pub methods: Vec<Spanned<TraitMethod>>,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: Spanned<String>,
    pub params: Vec<Spanned<Param>>,
    pub return_type: Option<Spanned<TypeAnnotation>>,
    pub has_body: bool,
}

// ─────────────────────────────────────────────────────────────
//  Imports, Constants, Statics, Type Aliases
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<Spanned<String>>,
    pub aliases: Vec<ImportAlias>,
    /// Source-file import: `import "util/math.yk";`
    pub file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImportAlias {
    pub name: Spanned<String>,
    pub alias: Option<Spanned<String>>,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub ty: Option<Spanned<TypeAnnotation>>,
    pub value: Spanned<Expr>,
}

#[derive(Debug, Clone)]
pub struct StaticDecl {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub ty: Spanned<TypeAnnotation>,
    pub value: Option<Spanned<Expr>>,
}

#[derive(Debug, Clone)]
pub struct TypeAliasDecl {
    pub visibility: Visibility,
    pub name: Spanned<String>,
    pub generics: Vec<GenericParam>,
    pub ty: Spanned<TypeAnnotation>,
}

// ─────────────────────────────────────────────────────────────
//  Statements
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Spanned<Stmt>>,
    pub result: Option<Box<Spanned<Expr>>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// let name [: type] [= expr];
    Let {
        name: Spanned<String>,
        ty: Option<Spanned<TypeAnnotation>>,
        value: Option<Spanned<Expr>>,
    },
    /// var name [: type] = expr;
    Var {
        name: Spanned<String>,
        ty: Option<Spanned<TypeAnnotation>>,
        value: Spanned<Expr>,
    },
    /// expr;
    Expr(Spanned<Expr>),
    /// return [expr];
    Return(Option<Spanned<Expr>>),
    /// defer expr;
    Defer(Spanned<Expr>),
    /// break [label];
    Break(Option<Spanned<String>>),
    /// continue [label];
    Continue(Option<Spanned<String>>),
    /// Block
    Block(Block),
    /// if expr { block } [else if expr { block }] [else { block }]
    If(IfExpr),
    /// while expr { block }
    While {
        condition: Spanned<Expr>,
        body: Block,
    },
    /// for name in expr { block }
    For {
        variable: Spanned<String>,
        iterable: Spanned<Expr>,
        body: Block,
    },
    /// Classic C-style for loop: `for (init; cond; update) { block }`
    ForC {
        init: Option<Box<Spanned<Stmt>>>,
        condition: Option<Spanned<Expr>>,
        update: Option<Box<Spanned<Stmt>>>,
        body: Block,
    },
    /// loop { block }
    Loop { body: Block },
    /// switch (expr) { case label: ... break; ... default: ... }
    Switch {
        scrutinee: Spanned<Expr>,
        arms: Vec<SwitchArm>,
    },
}

/// One `case`/`default` arm of a switch statement.
#[derive(Debug, Clone)]
pub struct SwitchArm {
    /// Label expression for `case` arms (None for `default`).
    pub label: Option<Spanned<Expr>>,
    pub body: Vec<Spanned<Stmt>>,
}

// ─────────────────────────────────────────────────────────────
//  Expressions
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Null,

    // Identifiers & paths
    Ident(Path),

    // Binary operations
    Binary {
        op: Spanned<BinOp>,
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
    },

    // Unary operations
    Unary {
        op: Spanned<UnOp>,
        operand: Box<Spanned<Expr>>,
    },

    // Increment/decrement: `x++`, `--x`
    Incr {
        operand: Box<Spanned<Expr>>,
        positive: bool,
        is_postfix: bool,
    },

    // Ternary: `cond ? then : else`
    Ternary {
        condition: Box<Spanned<Expr>>,
        then_branch: Box<Spanned<Expr>>,
        else_branch: Box<Spanned<Expr>>,
    },

    // Assignment
    Assign {
        target: Box<Spanned<Expr>>,
        value: Box<Spanned<Expr>>,
    },

    // Compound assignment
    CompoundAssign {
        op: Spanned<CompoundOp>,
        target: Box<Spanned<Expr>>,
        value: Box<Spanned<Expr>>,
    },

    // Function call
    Call {
        callee: Box<Spanned<Expr>>,
        args: Vec<Spanned<Expr>>,
    },

    // Method call
    MethodCall {
        receiver: Box<Spanned<Expr>>,
        method: Spanned<String>,
        args: Vec<Spanned<Expr>>,
    },

    // Field access
    Field {
        object: Box<Spanned<Expr>>,
        field: Spanned<String>,
    },

    // Index access
    Index {
        object: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },

    // Array literal
    Array(Vec<Spanned<Expr>>),

    // Tuple literal
    Tuple(Vec<Spanned<Expr>>),

    // Struct literal
    StructLiteral {
        path: Path,
        fields: Vec<(Spanned<String>, Spanned<Expr>)>,
    },

    // If expression
    If(Box<IfExpr>),

    // Block expression
    Block(Block),

    // Match expression
    Match {
        scrutinee: Box<Spanned<Expr>>,
        arms: Vec<MatchArm>,
    },

    // Closure / lambda
    Closure {
        params: Vec<Spanned<Param>>,
        return_type: Option<Spanned<TypeAnnotation>>,
        body: Box<ClosureBody>,
    },

    // Cast expression
    Cast {
        expr: Box<Spanned<Expr>>,
        ty: Spanned<TypeAnnotation>,
    },

    // Sizeof / alignof
    Sizeof(Spanned<TypeAnnotation>),
    Alignof(Spanned<TypeAnnotation>),

    // Type annotation (ascription)
    TypeAnnotation {
        expr: Box<Spanned<Expr>>,
        ty: Spanned<TypeAnnotation>,
    },

    // Arena allocation
    ArenaAlloc {
        expr: Box<Spanned<Expr>>,
    },

    /// `this` — the implicit receiver inside instance methods
    This,

    // New (heap allocation)
    New {
        ty: Spanned<TypeAnnotation>,
        args: Vec<Spanned<Expr>>,
    },

    // Array repeat [expr; count]
    ArrayRepeat {
        value: Box<Spanned<Expr>>,
        count: Box<Spanned<Expr>>,
    },
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Spanned<Expr>,
    pub then_branch: Block,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    If(Box<IfExpr>),
    Block(Block),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Spanned<Pattern>,
    pub guard: Option<Spanned<Expr>>,
    pub body: Spanned<Expr>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Spanned<Expr>),
    Ident(Spanned<String>),
    Wildcard,
    Tuple(Vec<Spanned<Pattern>>),
    Struct {
        path: Path,
        fields: Vec<(Spanned<String>, Spanned<Pattern>)>,
    },
    Variant {
        path: Path,
        fields: Vec<Spanned<Pattern>>,
    },
}

#[derive(Debug, Clone)]
pub enum ClosureBody {
    Expr(Spanned<Expr>),
    Block(Block),
}

// ─────────────────────────────────────────────────────────────
//  Operators
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    BitAnd, BitOr, BitXor,
    Shl, Shr,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
}

impl BinOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Self::Add),
            "-" => Some(Self::Sub),
            "*" => Some(Self::Mul),
            "/" => Some(Self::Div),
            "%" => Some(Self::Mod),
            "&" => Some(Self::BitAnd),
            "|" => Some(Self::BitOr),
            "^" => Some(Self::BitXor),
            "<<" => Some(Self::Shl),
            ">>" => Some(Self::Shr),
            "==" => Some(Self::Eq),
            "!=" => Some(Self::Ne),
            "<" => Some(Self::Lt),
            ">" => Some(Self::Gt),
            "<=" => Some(Self::Le),
            ">=" => Some(Self::Ge),
            "&&" => Some(Self::And),
            "||" => Some(Self::Or),
            _ => None,
        }
    }
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Mod => write!(f, "%"),
            BinOp::BitAnd => write!(f, "&"),
            BinOp::BitOr => write!(f, "|"),
            BinOp::BitXor => write!(f, "^"),
            BinOp::Shl => write!(f, "<<"),
            BinOp::Shr => write!(f, ">>"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Ne => write!(f, "!="),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::Le => write!(f, "<="),
            BinOp::Ge => write!(f, ">="),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
    Deref,
    Ref,
}

impl std::fmt::Display for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnOp::Neg => write!(f, "-"),
            UnOp::Not => write!(f, "!"),
            UnOp::BitNot => write!(f, "~"),
            UnOp::Deref => write!(f, "*"),
            UnOp::Ref => write!(f, "&"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundOp {
    Add, Sub, Mul, Div, Mod,
    BitAnd, BitOr, BitXor,
    Shl, Shr,
}

impl std::fmt::Display for CompoundOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompoundOp::Add => write!(f, "+="),
            CompoundOp::Sub => write!(f, "-="),
            CompoundOp::Mul => write!(f, "*="),
            CompoundOp::Div => write!(f, "/="),
            CompoundOp::Mod => write!(f, "%="),
            CompoundOp::BitAnd => write!(f, "&="),
            CompoundOp::BitOr => write!(f, "|="),
            CompoundOp::BitXor => write!(f, "^="),
            CompoundOp::Shl => write!(f, "<<="),
            CompoundOp::Shr => write!(f, ">>="),
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  Visibility
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    Public,
    #[default]
    Private,
}
