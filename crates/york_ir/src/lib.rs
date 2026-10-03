//! York IR — typed intermediate representation produced by semantic analysis
//! and consumed by the C and WebAssembly backends.

use york_ast::PrimitiveType;

// ─────────────────────────────────────────────────────────────
//  Types
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Void,
    Never,
    Bool,
    Char,
    /// String — a statically-allocated byte string.
    Str,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    /// A user struct, referenced by name.
    Struct(String),
    /// An arena / bump-allocator with typed elements.
    Arena(Box<Ty>),
    /// A built-in open-addressing hash map: HashMap<K, V>
    HashMap(Box<Ty>, Box<Ty>),
    /// Slices: T[]
    Slice(Box<Ty>),
    /// Fixed-size arrays with a compile-time length: T[N]
    Array(Box<Ty>, usize),
    /// An enum, referenced by name.
    Enum(String),
    /// Inferred (`_`) — filled in during type checking.
    Inferred,
}

impl Ty {
    pub fn from_primitive(p: PrimitiveType) -> Ty {
        match p {
            PrimitiveType::I8 => Ty::I8,
            PrimitiveType::I16 => Ty::I16,
            PrimitiveType::I32 => Ty::I32,
            PrimitiveType::I64 => Ty::I64,
            PrimitiveType::I128 => Ty::I128,
            PrimitiveType::U8 => Ty::U8,
            PrimitiveType::U16 => Ty::U16,
            PrimitiveType::U32 => Ty::U32,
            PrimitiveType::U64 => Ty::U64,
            PrimitiveType::U128 => Ty::U128,
            PrimitiveType::F16 => Ty::F32, // no native f16; promote
            PrimitiveType::F32 => Ty::F32,
            PrimitiveType::F64 => Ty::F64,
            PrimitiveType::Bool => Ty::Bool,
            PrimitiveType::Char => Ty::Char,
            PrimitiveType::String => Ty::Str,
        }
    }
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Void => write!(f, "void"),
            Ty::Never => write!(f, "!"),
            Ty::Bool => write!(f, "bool"),
            Ty::Char => write!(f, "char"),
            Ty::Str => write!(f, "String"),
            Ty::I8 => write!(f, "i8"),
            Ty::I16 => write!(f, "i16"),
            Ty::I32 => write!(f, "int"),
            Ty::I64 => write!(f, "long"),
            Ty::I128 => write!(f, "long long"),
            Ty::U8 => write!(f, "unsigned char"),
            Ty::U16 => write!(f, "unsigned short"),
            Ty::U32 => write!(f, "unsigned int"),
            Ty::U64 => write!(f, "unsigned long"),
            Ty::U128 => write!(f, "unsigned long long"),
            Ty::F32 => write!(f, "float"),
            Ty::F64 => write!(f, "double"),
            Ty::Struct(name) => write!(f, "{name}"),
            Ty::Enum(name) => write!(f, "{name}"),
            Ty::Arena(inner) => write!(f, "Arena<{inner}>"),
            Ty::HashMap(k, v) => write!(f, "HashMap<{k}, {v}>"),
            Ty::Slice(inner) => write!(f, "{inner}[]"),
        Ty::Array(inner, n) => write!(f, "{inner}[{n}]"),
            Ty::Inferred => write!(f, "_"),
        }
    }
}

impl Ty {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::I128
            | Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::U128
            | Ty::F32 | Ty::F64
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Ty::F32 | Ty::F64)
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::I128
            | Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::U128
        )
    }

    pub fn default_value(&self) -> String {
        match self {
            Ty::Void | Ty::Never => String::new(),
            Ty::Bool => "false".into(),
            Ty::Char => "'\\0'".into(),
            Ty::Str => "\"\"".into(),
            Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::I128 => "0".into(),
            Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::U128 => "0".into(),
            Ty::F32 | Ty::F64 => "0.0".into(),
            Ty::Struct(_) | Ty::Slice(_) | Ty::Array(_, _) | Ty::Enum(_) | Ty::Inferred | Ty::Arena(_) => "{}".into(),
            Ty::HashMap(_, _) => "{ (void*)0, (void*)0, 0, 0 }".into(),
        }
    }

    /// A sanitized identifier fragment used for C symbols (arena helper names, etc.).
    pub fn tag(&self) -> String {
        match self {
            Ty::Struct(name) => name.clone(),
            Ty::Enum(name) => name.clone(),
            Ty::Arena(inner) => format!("Arena{}", inner.tag()),
            Ty::HashMap(k, v) => format!("HashMap{}To{}", k.tag(), v.tag()),
            Ty::Slice(inner) => format!("Sl{}", inner.tag()),
        Ty::Array(inner, n) => format!("A{}{}", inner.tag(), n),
            Ty::Str => "Str".into(),
            Ty::Bool => "Bool".into(),
            Ty::Char => "Char".into(),
            Ty::I8 => "I8".into(),
            Ty::I16 => "I16".into(),
            Ty::I32 => "I32".into(),
            Ty::I64 => "I64".into(),
            Ty::I128 => "I128".into(),
            Ty::U8 => "U8".into(),
            Ty::U16 => "U16".into(),
            Ty::U32 => "U32".into(),
            Ty::U64 => "U64".into(),
            Ty::U128 => "U128".into(),
            Ty::F32 => "F32".into(),
            Ty::F64 => "F64".into(),
            Ty::Void | Ty::Never => "Void".into(),
            Ty::Inferred => "X".into(),
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  Program
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Struct(StructDef),
    Enum(EnumDef),
    Fn(FnDef),
    Import(String),
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Ty>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub is_packed: bool,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: Ty,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub return_ty: Ty,
    pub body: Vec<Stmt>,
}

// ─────────────────────────────────────────────────────────────
//  Statements
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `Type name;` or `Type name = init;`
    VarDecl { name: String, ty: Ty, init: Option<Expr>, is_const: bool },
    /// `expr` evaluated for side effects
    Expr(Expr),
    Assign { target: Expr, value: Expr },
    Return(Option<Expr>),
    If { condition: Expr, then: Vec<Stmt>, else_: Vec<Stmt> },
    While { condition: Expr, body: Vec<Stmt> },
    For { init: Vec<Stmt>, condition: Option<Expr>, update: Option<Expr>, body: Vec<Stmt> },
    /// `for (Type x : arenaOrList)` — lowered to a normal for loop by codegen.
    ForEach { var: String, elem_ty: Ty, iterable: Expr, body: Vec<Stmt> },
    Break,
    Continue,
    Block(Vec<Stmt>),
    /// `switch (scrutinee) { case ...: ... }` — lower to a C switch by codegen.
    Switch {
        scrutinee: Expr,
        arms: Vec<SwitchArm>,
    },
}

/// One `case` arm of a switch statement.
#[derive(Debug, Clone)]
pub struct SwitchArm {
    /// Label value expression (e.g. an enum variant reference or literal).
    pub label: Expr,
    pub body: Vec<Stmt>,
    pub is_default: bool,
}

// ─────────────────────────────────────────────────────────────
//  Expressions
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Bool(bool),
    Null,

    /// Resolved variable reference
    Var(String),

    /// `this` (implicit receiver)
    This,

    /// Field access on a struct value
    Field { object: Box<Expr>, field: String },

    /// Method call, resolved to a mangled top-level function name.
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        resolved: String,
        args: Vec<Expr>,
    },

    /// Bare call, resolved to a mangled top-level function name.
    Call { callee: String, resolved: String, args: Vec<Expr> },

    /// `print(...)` / `println(...)`
    Print { text: Box<Expr>, newline: bool, ty: Ty },

    /// Compound assignment (needs a statement in C)
    CompoundAssign {
        op: york_ast::CompoundOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },

    /// Simple assignment (typically used as a statement)
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },

    /// Binary operation
    Binary { op: york_ast::BinOp, left: Box<Expr>, right: Box<Expr> },

    /// String concatenation – `str1 + str2`.
    Strcat { left: Box<Expr>, right: Box<Expr> },

    /// String equality/inequality comparison – `str1 == str2` or `str1 != str2`.
    Streq { is_eq: bool, left: Box<Expr>, right: Box<Expr> },

    /// Unary operation
    Unary { op: york_ast::UnOp, operand: Box<Expr> },

    /// Postfix/prefix inc/dec
    Incr { operand: Box<Expr>, positive: bool, is_postfix: bool },

    /// Ternary
    Ternary { condition: Box<Expr>, then: Box<Expr>, else_: Box<Expr> },

    /// Index: `obj[i]`
    Index { object: Box<Expr>, index: Box<Expr> },

    /// Struct literal construction
    StructLiteral {
        struct_name: String,
        fields: Vec<(String, Expr)>,
    },

    /// `new StructName(args...)` — positionally init fields.
    New {
        struct_name: String,
        defaults: Vec<FieldDef>,
        args: Vec<Expr>,
    },

    /// Explicit cast: `(T) expr`.
    Cast { ty: Ty, expr: Box<Expr> },

    /// `sizeof(T)` — a compile-time constant in the generated C.
    Sizeof(Ty),

    /// `alignof(T)` — a compile-time constant in the generated C.
    Alignof(Ty),

    /// Array literal `[a, b, c]` — lowered to a C compound literal.
    ArrayLit {
        elem_ty: Ty,
        elems: Vec<Expr>,
        len: usize,
    },

    /// Reference to an enum variant: `Color.Red`.
    EnumRef { enum_name: String, variant: String },
}