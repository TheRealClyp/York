// York v1.5.1 Industrial-Grade Compiler Architecture & Execution Engine
// Implements robust multi-line parsing, rigorous error diagnostics, PEMDAS recursive descent,
// structured control flow (whenever / otherwise / loop), and advanced I/O (say / hear).

pub mod diagnostics {
    #[derive(Debug, Clone)]
    pub struct CompileDiagnostic {
        pub file_line: usize,
        pub column: usize,
        pub token_index: usize,
        pub message: String,
        pub severity: DiagnosticSeverity,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiagnosticSeverity {
        SyntaxError,
        MemoryLeakWarning,
        RuntimePanic,
    }

    impl std::fmt::Display for CompileDiagnostic {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "[York {:?} at line {}, col {} (token {})]: {}", self.severity, self.file_line, self.column, self.token_index, self.message)
        }
    }
}

pub mod ast_ext {
    use super::diagnostics::CompileDiagnostic;

    #[derive(Debug, Clone)]
    pub enum Stmt {
        Say { exprs: Vec<Expr>, line: usize },
        Hear { variable: String, line: usize },
        Whenever { condition: Expr, then_branch: Vec<Stmt>, else_branch: Vec<Stmt>, line: usize },
        Loop { condition: Expr, body: Vec<Stmt>, line: usize },
        Assign { name: String, value: Expr, line: usize },
        Expression(Expr),
    }

    #[derive(Debug, Clone)]
    pub enum Expr {
        IntLiteral(i64),
        FloatLiteral(f64),
        StringLiteral(String),
        Variable(String),
        BinaryOp { op: Operator, left: Box<Expr>, right: Box<Expr> },
        UnaryOp { op: UnaryOperator, expr: Box<Expr> },
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Operator {
        Add, Sub, Mul, Div, Mod,
        Equal, NotEqual, Less, Greater, LessEqual, GreaterEqual,
        LogicalAnd, LogicalOr,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum UnaryOperator {
        Not, Negate,
    }
}
