//! Semantic analysis and AST → HIR lowering for York.

pub mod error;
pub mod tables;

use york_ast::span::Spanned;
use york_ast::{Expr as AstExpr, Stmt as AstStmt, TypeAnnotation, Item, Program as AstProgram, BinOp};

use crate::error::{SemanticError, SemanticErrorWithSpan};
use crate::tables::{StructInfo, EnumInfo, FnInfo, Scope, Tables};
use york_ir::{self as hir, Ty};

/// Result of semantic analysis.
pub struct SemaResult {
    pub program: hir::Program,
    pub errors: Vec<SemanticErrorWithSpan>,
}

pub fn analyze(ast: &AstProgram) -> SemaResult {
    let mut ctx = SemaCtx::new();
    ctx.collect_top_level(ast);
    let program = ctx.lower_program(ast);
    SemaResult {
        program,
        errors: ctx.errors,
    }
}

struct SemaCtx {
    tables: Tables,
    errors: Vec<SemanticErrorWithSpan>,
}

impl SemaCtx {
    fn new() -> Self {
        SemaCtx {
            tables: Tables::new(),
            errors: Vec::new(),
        }
    }

    // ─────────────────────────────────────────────────────────
    //  Type resolution
    // ─────────────────────────────────────────────────────────

    fn resolve_type(&mut self, ann: &TypeAnnotation) -> Ty {
        match ann {
            TypeAnnotation::Primitive(p) => Ty::from_primitive(*p),
            TypeAnnotation::Void => Ty::Void,
            TypeAnnotation::Never => Ty::Never,
            TypeAnnotation::Named(path) => {
                let name = path.segments.last().map(|s| s.node.as_str()).unwrap_or("?");
                match name {
                    "int" => Ty::I32,
                    "long" => Ty::I64,
                    "short" => Ty::I16,
                    "byte" => Ty::I8,
                    "float" => Ty::F32,
                    "double" => Ty::F64,
                    "bool" | "boolean" => Ty::Bool,
                    "char" => Ty::Char,
                    "string" | "String" => Ty::Str,
                    "void" => Ty::Void,
                    n if self.tables.structs.contains_key(n) => Ty::Struct(n.to_string()),
                    n if self.tables.enums.contains_key(n) => Ty::Enum(n.to_string()),
                    n => {
                        // Could be an alias; treat as struct for now.
                        Ty::Struct(n.to_string())
                    }
                }
            }
            TypeAnnotation::Generic { base, args } => {
                let base_name = base.segments.last().map(|s| s.node.as_str()).unwrap_or("");
                if base_name == "Arena" || base_name == "arena" {
                    if let Some(first) = args.first() {
                        Ty::Arena(Box::new(self.resolve_type(&first.node)))
                    } else {
                        Ty::Arena(Box::new(Ty::Inferred))
                    }
                } else if base_name == "HashMap" || base_name == "hashmap" || base_name == "Map" {
                    let k = args
                        .first()
                        .map(|a| self.resolve_type(&a.node))
                        .unwrap_or(Ty::Inferred);
                    let v = args
                        .get(1)
                        .map(|a| self.resolve_type(&a.node))
                        .unwrap_or(Ty::Inferred);
                    Ty::HashMap(Box::new(k), Box::new(v))
                } else {
                    let mut generic = base_name.to_string();
                    for a in args {
                        let ty = self.resolve_type(&a.node);
                        generic.push_str(&format!("{}", ty));
                    }
                    Ty::Struct(generic)
                }
            }
            TypeAnnotation::Slice(inner) => Ty::Slice(Box::new(self.resolve_type(&inner.node))),
            TypeAnnotation::Optional(inner) => self.resolve_type(&inner.node),
            TypeAnnotation::Pointer(inner) | TypeAnnotation::MutPointer(inner)
            | TypeAnnotation::Reference(inner) | TypeAnnotation::MutReference(inner) => {
                self.resolve_type(&inner.node)
            }
            TypeAnnotation::Tuple(_) => Ty::Struct("Tuple".into()),
            TypeAnnotation::Array(elem, len) => {
                // `T[N]` — the length must be a literal so the C type is fixed.
                let inner = self.resolve_type(&elem.node);
                let n = match len.node.as_ref() {
                    AstExpr::Int(n) if *n >= 0 => *n as usize,
                    _ => {
                        self.errors.push(SemanticErrorWithSpan {
                            error: SemanticError::NotSupported(
                                "array length must be a non-negative literal".into(),
                            ),
                            span: len.span,
                        });
                        0
                    }
                };
                Ty::Array(Box::new(inner), n)
            }
            TypeAnnotation::Result(_, _) => Ty::Inferred,
            TypeAnnotation::Function(_, _) => Ty::Inferred,
            TypeAnnotation::Arena(inner) => {
                Ty::Arena(Box::new(self.resolve_type(&inner.node)))
            }
            TypeAnnotation::Inferred => Ty::Inferred,
        }
    }

    // ─────────────────────────────────────────────────────────
    //  Collection pass — collect struct and function signatures
    // ─────────────────────────────────────────────────────────

    fn collect_top_level(&mut self, ast: &AstProgram) {
        // Collect structs first
        for item in &ast.items {
            match &item.node {
                Item::Enum(e) => {
                    let mut variants = Vec::new();
                    for v in &e.variants {
                        let fields = v.node.fields.iter().map(|f| self.resolve_type(&f.node)).collect();
                        variants.push((v.node.name.node.clone(), fields));
                    }
                    self.tables.enums.insert(e.name.node.clone(), EnumInfo {
                        name: e.name.node.clone(),
                        variants,
                    });
                }
                _ => {}
            }
        }
        // Collect structs first
        for item in &ast.items {
            match &item.node {
                Item::Struct(s) => {
                    let mut fields = Vec::new();
                    for f in &s.fields {
                        fields.push((f.node.name.node.clone(), self.resolve_type(&f.node.ty.node)));
                    }
                    self.tables.structs.insert(s.name.node.clone(), StructInfo {
                        name: s.name.node.clone(),
                        fields,
                    });
                }
                _ => {}
            }
        }
        // Collect functions
        for item in &ast.items {
            match &item.node {
                Item::Function(f) => {
                    let mut params = Vec::new();
                    for p in &f.params {
                        params.push(self.resolve_type(&p.node.ty.node));
                    }
                    let return_ty = match &f.return_type {
                        Some(rt) => self.resolve_type(&rt.node),
                        None => Ty::Void,
                    };
                    if let Some(rt) = &f.return_type {
                        self.reject_array_return(&return_ty, rt.span);
                    }
                    let _qualified = f.name.node.clone();
                    self.tables.functions.insert(f.name.node.clone(), FnInfo {
                        name: f.name.node.clone(),
                        params,
                        return_ty,
                        is_static: f.is_static,
                    });
                }
                Item::Impl(imp) => {
                    let self_type = self.resolve_type(&imp.self_type.node);
                    for m in &imp.methods {
                        let mut params = Vec::new();
                        // First param is `self`/`Self` for instance methods
                        let is_static = m.node.is_static;
                        if !is_static {
                            params.push(self_type.clone());
                        }
                        for p in &m.node.params {
                            if p.node.name.node == "self" {
                                continue; // Already handled above
                            }
                            params.push(self.resolve_type(&p.node.ty.node));
                        }
                        let return_ty = match &m.node.return_type {
                            Some(rt) => self.resolve_type(&rt.node),
                            None => Ty::Void,
                        };
                        if let Some(rt) = &m.node.return_type {
                            self.reject_array_return(&return_ty, rt.span);
                        }
                        let qualified = self.mangle_method(&self_type, &m.node.name.node);
                        self.tables.functions.insert(qualified.clone(), FnInfo {
                            name: m.node.name.node.clone(),
                            params,
                            return_ty,
                            is_static,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    fn mangle_method(&self, self_type: &Ty, method: &str) -> String {
        match self_type {
            Ty::Struct(name) => format!("{}_{}", name, method),
            Ty::Arena(elem) => format!("Arena_{}_{}", elem.tag(), method),
            Ty::HashMap(k, v) => format!("HashMap_{}_{}_{}", k.tag(), v.tag(), method),
            _ => method.to_string(),
        }
    }

    // ─────────────────────────────────────────────────────────
    //  Lowering pass
    // ─────────────────────────────────────────────────────────

    fn lower_program(&mut self, ast: &AstProgram) -> hir::Program {
        let mut items = Vec::new();
        for item in &ast.items {
            match &item.node {
                Item::Enum(e) => {
                    let variants = e
                        .variants
                        .iter()
                        .map(|v| hir::EnumVariant {
                            name: v.node.name.node.clone(),
                            fields: v.node.fields.iter().map(|f| self.resolve_type(&f.node)).collect(),
                        })
                        .collect();
                    items.push(hir::Item::Enum(hir::EnumDef {
                        name: e.name.node.clone(),
                        variants,
                    }));
                }
                Item::Struct(s) => {
                    let mut fields = Vec::new();
                    for f in &s.fields {
                        fields.push(hir::FieldDef {
                            name: f.node.name.node.clone(),
                            ty: self.resolve_type(&f.node.ty.node),
                            default: None,
                        });
                    }
                    items.push(hir::Item::Struct(hir::StructDef {
                        name: s.name.node.clone(),
                        fields,
                        is_packed: s.is_packed,
                    }));
                }
                Item::Function(f) => {
                    let mut params = Vec::new();
                    for p in &f.params {
                        params.push((p.node.name.node.clone(), self.resolve_type(&p.node.ty.node)));
                    }
                    let return_ty = match &f.return_type {
                        Some(rt) => self.resolve_type(&rt.node),
                        None => Ty::Void,
                    };
                    if let Some(rt) = &f.return_type {
                        self.reject_array_return(&return_ty, rt.span);
                    }
                    let body = match &f.body {
                        Some(block) => {
                            let mut scope = Scope::new();
                            for p in &f.params {
                                scope.define(p.node.name.node.clone(), self.resolve_type(&p.node.ty.node));
                            }
                            let mut fnerrs = Vec::new();
                            let lowered = self.lower_block(block, &mut scope, &mut fnerrs);
                            self.errors.extend(fnerrs);
                            lowered
                        }
                        None => Vec::new(),
                    };
                    items.push(hir::Item::Fn(hir::FnDef {
                        name: f.name.node.clone(),
                        params,
                        return_ty,
                        body,
                    }));
                }
                Item::Impl(imp) => {
                    let self_type = self.resolve_type(&imp.self_type.node);
                    for m in &imp.methods {
                        let mut params = Vec::new();
                        let is_static = m.node.is_static;
                        if !is_static {
                            if let Ty::Struct(name) = &self_type {
                                params.push(("self".to_string(), Ty::Struct(name.clone())));
                            }
                        }
                        for p in &m.node.params {
                            if p.node.name.node == "self" { continue; }
                            params.push((p.node.name.node.clone(), self.resolve_type(&p.node.ty.node)));
                        }
                        let return_ty = match &m.node.return_type {
                            Some(rt) => self.resolve_type(&rt.node),
                            None => Ty::Void,
                        };
                        if let Some(rt) = &m.node.return_type {
                            self.reject_array_return(&return_ty, rt.span);
                        }
                        let qualified = self.mangle_method(&self_type, &m.node.name.node);
                        let body = match &m.node.body {
                            Some(block) => {
                                let mut scope = Scope::new();
                                if !is_static {
                                    if let Ty::Struct(name) = &self_type {
                                        scope.define("self".to_string(), Ty::Struct(name.clone()));
                                    }
                                }
                                for p in &m.node.params {
                                    if p.node.name.node == "self" {
                                        continue;
                                    }
                                    scope.define(
                                        p.node.name.node.clone(),
                                        self.resolve_type(&p.node.ty.node),
                                    );
                                }
                                let mut fnerrs = Vec::new();
                                let lowered = self.lower_block(block, &mut scope, &mut fnerrs);
                                self.errors.extend(fnerrs);
                                lowered
                            }
                            None => Vec::new(),
                        };
                        items.push(hir::Item::Fn(hir::FnDef {
                            name: qualified,
                            params,
                            return_ty,
                            body,
                        }));
                    }
                }
                Item::Import(imp) => {
                    // Source-file imports are resolved (and their items merged)
                    // by the driver before analysis; nothing to lower here.
                    if imp.file.is_some() {
                        continue;
                    }
                    let path = imp.path.iter().map(|s| s.node.as_str()).collect::<Vec<_>>().join(".");
                    items.push(hir::Item::Import(path));
                }
                _ => {}
            }
        }
        hir::Program { items }
    }

    // ─────────────────────────────────────────────────────────
    //  Block / statement lowering
    // ─────────────────────────────────────────────────────────

    fn lower_block(&mut self, block: &york_ast::Block, scope: &mut Scope, errors: &mut Vec<SemanticErrorWithSpan>) -> Vec<hir::Stmt> {
        let mut stmts = Vec::new();
        for s in &block.stmts {
            if let Some(stmt) = self.lower_stmt(s, scope, errors) {
                stmts.push(stmt);
            }
        }
        stmts
    }

    /// Lower a bare `Vec<Spanned<Stmt>>` (switch-arm bodies).
    fn lower_block_stmts(&mut self, block: &[Spanned<AstStmt>], scope: &mut Scope, errors: &mut Vec<SemanticErrorWithSpan>) -> Vec<hir::Stmt> {
        let mut stmts = Vec::new();
        for s in block {
            if let Some(stmt) = self.lower_stmt(s, scope, errors) {
                stmts.push(stmt);
            }
        }
        stmts
    }

    /// C functions cannot return arrays by value; report instead of emitting
    /// invalid C.
    fn reject_array_return(&mut self, ty: &Ty, span: york_ast::span::Span) {
        if matches!(ty, Ty::Array(..)) {
            self.errors.push(SemanticErrorWithSpan {
                error: SemanticError::NotSupported(
                    "returning a fixed array by value; return a struct or slice instead".into(),
                ),
                span,
            });
        }
    }

    /// An array literal adopts the declared element type, and its elements are
    /// cast to match: `int[] xs = [1, 2];` / `int[2] xs = [1, 2];`.
    fn coerce_array_literal(
        &self,
        init: &mut hir::Expr,
        declared: &Ty,
        span: york_ast::span::Span,
        errors: &mut Vec<SemanticErrorWithSpan>,
    ) {
        let elem_ty_decl = match declared {
            Ty::Slice(inner) => Some((**inner).clone()),
            Ty::Array(inner, _) => Some((**inner).clone()),
            _ => None,
        };
        let Some(want) = elem_ty_decl else { return };
        let hir::Expr::ArrayLit { elem_ty, elems, len } = init else { return };
        // A fixed array may be under-filled (C zero-fills the tail) but never
        // over-filled.
        if let Ty::Array(_, n) = declared {
            if elems.len() > *n {
                errors.push(SemanticErrorWithSpan {
                    error: SemanticError::ArrayLength { expected: *n, found: elems.len() },
                    span,
                });
            }
            *len = *n;
        }
        if elems.is_empty() {
            *elem_ty = want;
        } else if *elem_ty != want {
            for e in elems.iter_mut() {
                let prev = std::mem::replace(e, hir::Expr::Null);
                *e = hir::Expr::Cast {
                    ty: want.clone(),
                    expr: Box::new(prev),
                };
            }
            *elem_ty = want;
        }
    }

    fn lower_stmt(&mut self, stmt: &Spanned<AstStmt>, scope: &mut Scope, errors: &mut Vec<SemanticErrorWithSpan>) -> Option<hir::Stmt> {
        match &stmt.node {
            AstStmt::Let { name, ty, value } => {
                let resolved_ty = ty.as_ref().map(|t| self.resolve_type(&t.node)).unwrap_or(Ty::Inferred);
                let mut init = value.as_ref().map(|e| self.lower_expr(e, scope, errors));
                if let Some(e) = init.as_mut() {
                    let span = value
                        .as_ref()
                        .map(|v| v.span)
                        .unwrap_or(york_ast::span::Span::new(
                            york_ast::span::BytePos::ZERO,
                            york_ast::span::BytePos::ZERO,
                        ));
                    self.coerce_array_literal(e, &resolved_ty, span, errors);
                }
                let final_ty = match (&resolved_ty, &init) {
                    (Ty::Inferred, Some(e)) => self.infer_expr_type(e, scope).unwrap_or(Ty::Inferred),
                    (t, _) => t.clone(),
                };
                scope.define(name.node.clone(), final_ty.clone());
                Some(hir::Stmt::VarDecl {
                    name: name.node.clone(),
                    ty: final_ty,
                    init,
                    is_const: false,
                })
            }
            AstStmt::Var { name, ty, value } => {
                let resolved_ty = ty.as_ref().map(|t| self.resolve_type(&t.node)).unwrap_or(Ty::Inferred);
                let mut init = self.lower_expr(value, scope, errors);
                self.coerce_array_literal(&mut init, &resolved_ty, value.span, errors);
                let final_ty = match (&resolved_ty, &init) {
                    (Ty::Inferred, e) => self.infer_expr_type(e, scope).unwrap_or(Ty::Inferred),
                    (t, _) => t.clone(),
                };
                scope.define(name.node.clone(), final_ty.clone());
                Some(hir::Stmt::VarDecl {
                    name: name.node.clone(),
                    ty: final_ty,
                    init: Some(init),
                    is_const: false,
                })
            }
            AstStmt::Expr(e) => {
                // Assignments become statements
                match &e.node {
                    AstExpr::Assign { target, value } => {
                        Some(hir::Stmt::Assign {
                            target: self.lower_expr(target, scope, errors),
                            value: self.lower_expr(value, scope, errors),
                        })
                    }
                    AstExpr::CompoundAssign { .. } => {
                        Some(hir::Stmt::Expr(self.lower_expr(e, scope, errors)))
                    }
                    _ => Some(hir::Stmt::Expr(self.lower_expr(e, scope, errors))),
                }
            }
            AstStmt::Return(val) => {
                let v = val.as_ref().map(|e| self.lower_expr(e, scope, errors));
                Some(hir::Stmt::Return(v))
            }
            AstStmt::If(if_expr) => {
                let condition = self.lower_expr(&if_expr.condition, scope, errors);
                let mut then_scope = scope.child();
                let then = self.lower_block(&if_expr.then_branch, &mut then_scope, errors);
                let else_ = match &if_expr.else_branch {
                    Some(york_ast::ElseBranch::Block(b)) => {
                        let mut else_scope = scope.child();
                        self.lower_block(b, &mut else_scope, errors)
                    }
                    Some(york_ast::ElseBranch::If(ei)) => {
                        // Chain: wrap else-if as a single-statement block
                        let mut else_scope = scope.child();
                        let stmt = self.lower_stmt(
                            &Spanned::new(AstStmt::If(*ei.clone()), stmt.span),
                            &mut else_scope,
                            errors,
                        );
                        stmt.into_iter().collect()
                    }
                    None => Vec::new(),
                };
                Some(hir::Stmt::If { condition, then, else_ })
            }
            AstStmt::While { condition, body } => {
                let condition = self.lower_expr(condition, scope, errors);
                let mut body_scope = scope.child();
                let body = self.lower_block(body, &mut body_scope, errors);
                Some(hir::Stmt::While { condition, body })
            }
            AstStmt::For { variable, iterable, body } => {
                let iterable = self.lower_expr(iterable, scope, errors);
                let elem_ty = match self.infer_expr_type(&iterable, scope) {
                    Some(Ty::Slice(t)) => *t,
                    Some(Ty::Arena(t)) => *t,
                    Some(t) => t,
                    None => Ty::Inferred,
                };
                scope.define(variable.node.clone(), elem_ty.clone());
                let mut body_scope = scope.child();
                // Add the variable in inner scope too
                body_scope.define(variable.node.clone(), elem_ty.clone());
                let body = self.lower_block(body, &mut body_scope, errors);
                Some(hir::Stmt::ForEach {
                    var: variable.node.clone(),
                    elem_ty,
                    iterable,
                    body,
                })
            }
            AstStmt::ForC { init, condition, update, body } => {
                let mut for_scope = scope.child();
                let for_init = init.as_ref().map(|s| self.lower_stmt(s, &mut for_scope, errors)).into_iter().flatten().collect();
                let cond = condition.as_ref().map(|e| self.lower_expr(e, &mut for_scope, errors));
                let upd = update.as_ref().map(|s| {
                    match &s.node {
                        AstStmt::Expr(e) => self.lower_expr(e, &mut for_scope, errors),
                        _ => hir::Expr::Null,
                    }
                });
                let body = self.lower_block(body, &mut for_scope, errors);
                Some(hir::Stmt::For { init: for_init, condition: cond, update: upd, body })
            }
            AstStmt::Loop { body } => {
                let mut body_scope = scope.child();
                let body = self.lower_block(body, &mut body_scope, errors);
                // Loop is while(true)
                Some(hir::Stmt::While {
                    condition: hir::Expr::Bool(true),
                    body,
                })
            }
            AstStmt::Defer(_e) => {
                // Defer: for now, just execute at scope exit (simplified)
                None
            }
            AstStmt::Switch { scrutinee, arms } => {
                let scrut = self.lower_expr(scrutinee, scope, errors);
                let mut hir_arms = Vec::new();
                for arm in arms {
                    let label = match &arm.label {
                        Some(l) => self.lower_expr(l, scope, errors),
                        None => hir::Expr::Null, // default arm
                    };
                    let body = self.lower_block_stmts(&arm.body, scope, errors);
                    hir_arms.push(hir::SwitchArm {
                        label,
                        body,
                        is_default: arm.label.is_none(),
                    });
                }
                Some(hir::Stmt::Switch { scrutinee: scrut, arms: hir_arms })
            }
            AstStmt::Break(_label) => Some(hir::Stmt::Break),
            AstStmt::Continue(_label) => Some(hir::Stmt::Continue),
            AstStmt::Block(b) => {
                let mut child_scope = scope.child();
                Some(hir::Stmt::Block(self.lower_block(b, &mut child_scope, errors)))
            }
        }
    }

    // ─────────────────────────────────────────────────────────
    //  Expression lowering
    // ─────────────────────────────────────────────────────────

    fn lower_expr(&mut self, expr: &Spanned<AstExpr>, scope: &mut Scope, errors: &mut Vec<SemanticErrorWithSpan>) -> hir::Expr {
        match &expr.node {
            AstExpr::Int(v) => hir::Expr::Int(*v),
            AstExpr::Float(v) => hir::Expr::Float(*v),
            AstExpr::String(v) => hir::Expr::Str(v.clone()),
            AstExpr::Char(v) => hir::Expr::Char(*v),
            AstExpr::Bool(v) => hir::Expr::Bool(*v),
            AstExpr::Null => hir::Expr::Null,
            AstExpr::This => hir::Expr::This,

            AstExpr::Ident(path) => {
                let name = path.segments.last().map(|s| s.node.as_str()).unwrap_or("?");
                if scope.lookup(name).is_none() {
                    errors.push(SemanticErrorWithSpan {
                        error: SemanticError::UndefinedVariable(name.to_string()),
                        span: expr.span,
                    });
                }
                hir::Expr::Var(name.to_string())
            }

            AstExpr::Binary { op, left, right } => {
                let l = self.lower_expr(left, scope, errors);
                let r = self.lower_expr(right, scope, errors);
                if op.node == york_ast::BinOp::Add {
                    let lt = self.infer_expr_type(&l, scope);
                    let rt = self.infer_expr_type(&r, scope);
                    if matches!(lt, Some(Ty::Str)) || matches!(rt, Some(Ty::Str)) {
                        return hir::Expr::Strcat { left: Box::new(l), right: Box::new(r) };
                    }
                }
                if op.node == york_ast::BinOp::Eq || op.node == york_ast::BinOp::Ne {
                    let lt = self.infer_expr_type(&l, scope);
                    let rt = self.infer_expr_type(&r, scope);
                    if matches!(lt, Some(Ty::Str)) && matches!(rt, Some(Ty::Str)) {
                        return hir::Expr::Streq {
                            is_eq: op.node == york_ast::BinOp::Eq,
                            left: Box::new(l),
                            right: Box::new(r),
                        };
                    }
                }
                hir::Expr::Binary { op: op.node, left: Box::new(l), right: Box::new(r) }
            }

            AstExpr::Unary { op, operand } => {
                let o = self.lower_expr(operand, scope, errors);
                hir::Expr::Unary { op: op.node, operand: Box::new(o) }
            }

            AstExpr::Incr { operand, positive, is_postfix } => {
                let o = self.lower_expr(operand, scope, errors);
                hir::Expr::Incr { operand: Box::new(o), positive: *positive, is_postfix: *is_postfix }
            }

            AstExpr::Ternary { condition, then_branch, else_branch } => {
                let c = self.lower_expr(condition, scope, errors);
                let t = self.lower_expr(then_branch, scope, errors);
                let e = self.lower_expr(else_branch, scope, errors);
                hir::Expr::Ternary { condition: Box::new(c), then: Box::new(t), else_: Box::new(e) }
            }

            AstExpr::Assign { target, value } => {
                let t = self.lower_expr(target, scope, errors);
                let v = self.lower_expr(value, scope, errors);
                hir::Expr::Assign { target: Box::new(t), value: Box::new(v) }
            }

            AstExpr::CompoundAssign { op, target, value } => {
                // Desugar: target = target op value
                let t = self.lower_expr(target, scope, errors);
                let v = self.lower_expr(value, scope, errors);
                hir::Expr::CompoundAssign {
                    op: op.node,
                    target: Box::new(t),
                    value: Box::new(v),
                }
            }

            AstExpr::Call { callee, args } => {
                // Check if callee is a known function
                if let AstExpr::Ident(path) = &callee.node {
                    let name = path.segments.last().map(|s| s.node.as_str()).unwrap_or("");
                    // Special-case: print / println (single or multiple args).
                    if name == "print" || name == "println" {
                        let newline = name == "println";
                        let lowered_args: Vec<hir::Expr> = args
                            .iter()
                            .map(|a| self.lower_expr(a, scope, errors))
                            .collect();
                        if lowered_args.is_empty() {
                            return hir::Expr::Print {
                                text: Box::new(hir::Expr::Str(String::new())),
                                newline,
                                ty: Ty::Str,
                            };
                        }
                        if lowered_args.len() == 1 {
                            let ty = self.infer_expr_type(&lowered_args[0], scope).unwrap_or(Ty::I64);
                            return hir::Expr::Print {
                                text: Box::new(lowered_args[0].clone()),
                                newline,
                                ty,
                            };
                        }
                        // Multiple args: render each as a string and concatenate.
                        let mut acc: Option<hir::Expr> = None;
                        for a in lowered_args {
                            let as_str = self.as_string_expr(a, scope);
                            acc = Some(match acc {
                                None => as_str,
                                Some(prev) => hir::Expr::Strcat {
                                    left: Box::new(prev),
                                    right: Box::new(as_str),
                                },
                            });
                        }
                        return hir::Expr::Print {
                            text: Box::new(acc.unwrap_or(hir::Expr::Str(String::new()))),
                            newline,
                            ty: Ty::Str,
                        };
                    }
                    if name == "assert" {
                        let lowered_args: Vec<hir::Expr> = args
                            .iter()
                            .map(|a| self.lower_expr(a, scope, errors))
                            .collect();
                        if lowered_args.is_empty() || lowered_args.len() > 2 {
                            errors.push(SemanticErrorWithSpan {
                                error: SemanticError::ArgCount { expected: 1, got: lowered_args.len() },
                                span: expr.span,
                            });
                        }
                        // Inject a default message when the 2nd arg is omitted.
                        if lowered_args.len() < 2 {
                            let mut with_msg = lowered_args;
                            with_msg.push(hir::Expr::Str(String::new()));
                            return hir::Expr::Call {
                                callee: "assert".into(),
                                resolved: "__york_assert".into(),
                                args: with_msg,
                            };
                        }
                        return hir::Expr::Call {
                            callee: "assert".into(),
                            resolved: "__york_assert".into(),
                            args: lowered_args,
                        };
                    }
                    if name == "read_line" || name == "readLine" || name == "readline" || name == "input" {
                        let lowered_args: Vec<hir::Expr> = args
                            .iter()
                            .map(|a| self.lower_expr(a, scope, errors))
                            .collect();
                        return hir::Expr::Call {
                            callee: name.to_string(),
                            resolved: "__york_read_line".to_string(),
                            args: lowered_args,
                        };
                    }
                    if name == "read_int" || name == "readInt" || name == "readint" {
                        let lowered_args: Vec<hir::Expr> = args
                            .iter()
                            .map(|a| self.lower_expr(a, scope, errors))
                            .collect();
                        return hir::Expr::Call {
                            callee: name.to_string(),
                            resolved: "__york_read_int".to_string(),
                            args: lowered_args,
                        };
                    }
                    if name == "rand" {
                        let lowered_args: Vec<hir::Expr> = args
                            .iter()
                            .map(|a| self.lower_expr(a, scope, errors))
                            .collect();
                        return hir::Expr::Call {
                            callee: name.to_string(),
                            resolved: "rand".to_string(),
                            args: lowered_args,
                        };
                    }
                    if name == "srand" {
                        let lowered_args: Vec<hir::Expr> = args
                            .iter()
                            .map(|a| self.lower_expr(a, scope, errors))
                            .collect();
                        return hir::Expr::Call {
                            callee: name.to_string(),
                            resolved: "srand".to_string(),
                            args: lowered_args,
                        };
                    }
                    // random_range(lo, hi) -> int in [lo, hi).
                    if name == "random_range" {
                        let lowered_args: Vec<hir::Expr> = args
                            .iter()
                            .map(|a| self.lower_expr(a, scope, errors))
                            .collect();
                        return hir::Expr::Call {
                            callee: name.to_string(),
                            resolved: "__york_random_range".to_string(),
                            args: lowered_args,
                        };
                    }
                    // File I/O builtins — pass directly to C stdlib.
                    if name == "fopen" || name == "fclose" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: name.to_string(), args: lowered_args };
                    }
                    if name == "fprintf" {
                        let mut lowered_args = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect::<Vec<_>>();
                        // C fprintf takes (FILE*, fmt, ...) — inject the file pointer as first arg.
                        let mut c_args = vec![lowered_args.remove(0)];
                        c_args.append(&mut lowered_args);
                        return hir::Expr::Call { callee: name.to_string(), resolved: name.to_string(), args: c_args };
                    }
                    if name == "fputs" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: name.to_string(), args: lowered_args };
                    }
                    // write_file(path, content) — one-shot file write, returns bool.
                    if name == "write_file" || name == "append_file" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let resolved = if name == "write_file" { "__york_write_file" } else { "__york_append_file" };
                        return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                    }
                    if name == "read_file" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_read_file".to_string(), args: lowered_args };
                    }
                    if name == "file_exists" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_file_exists".to_string(), args: lowered_args };
                    }
                    if name == "str_len" || name == "strlen" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_strlen".to_string(), args: lowered_args };
                    }
                    if name == "to_string" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        if let Some(arg0) = lowered_args.first() {
                            let ty = self.infer_expr_type(arg0, scope);
                            let resolved = match ty {
                                Some(Ty::F32 | Ty::F64) => "__york_to_string_float",
                                Some(Ty::Bool) => "__york_to_string_bool",
                                Some(Ty::Str) => return arg0.clone(),
                                _ => "__york_to_string_int",
                            };
                            return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                        }
                    }
                    if name == "to_int" || name == "to_float" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let resolved = if name == "to_int" { "__york_to_int" } else { "__york_to_float" };
                        return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                    }
                    if name == "exec" || name == "system_cmd" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_exec".to_string(), args: lowered_args };
                    }
                    if name == "exit" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_exit".to_string(), args: lowered_args };
                    }
                    if name == "abs" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let is_flt = lowered_args.first().and_then(|a| self.infer_expr_type(a, scope)).map(|t| t.is_float()).unwrap_or(false);
                        let resolved = if is_flt { "fabs" } else { "llabs" };
                        return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                    }
                    if name == "sqrt" || name == "pow" || name == "floor" || name == "ceil" || name == "round" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: name.to_string(), args: lowered_args };
                    }
                    // Trig and log builtins — pass through to C math.h. ln maps to C log.
                    if name == "sin" || name == "cos" || name == "tan" || name == "ln"
                        || name == "log10" || name == "exp" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let resolved = if name == "ln" { "log".to_string() } else { name.to_string() };
                        return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                    }
                    // env(name) -> getenv(name) or "".
                    if name == "env" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_env".to_string(), args: lowered_args };
                    }
                    // platform_name() -> "windows" | "linux" | "macos" | "freebsd" | "unknown".
                    if name == "platform_name" {
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_platform_name".to_string(), args: vec![] };
                    }
                    if name == "min" || name == "max" || name == "clamp" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let is_flt = lowered_args.first().and_then(|a| self.infer_expr_type(a, scope)).map(|t| t.is_float()).unwrap_or(false);
                        let fn_base = match name {
                            "min" => "__york_math_min",
                            "max" => "__york_math_max",
                            _ => "__york_math_clamp",
                        };
                        let resolved = format!("{fn_base}_{}", if is_flt { "f" } else { "i" });
                        return hir::Expr::Call { callee: name.to_string(), resolved, args: lowered_args };
                    }
                    if name == "math_abs" || name == "abs" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let is_flt = lowered_args.first().and_then(|a| self.infer_expr_type(a, scope)).map(|t| t.is_float()).unwrap_or(false);
                        let resolved = format!("__york_math_abs_{}", if is_flt { "f" } else { "i" });
                        return hir::Expr::Call { callee: name.to_string(), resolved, args: lowered_args };
                    }
                    if name == "math_sqrt" || name == "sqrt" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_math_sqrt".to_string(), args: lowered_args };
                    }
                    if name == "math_pow" || name == "pow" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_math_pow".to_string(), args: lowered_args };
                    }
                    if name == "sleep" || name == "sleep_ms" || name == "delay" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_sleep".to_string(), args: lowered_args };
                    }
                    if name == "now" || name == "epoch_ms" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        return hir::Expr::Call { callee: name.to_string(), resolved: "__york_now".to_string(), args: lowered_args };
                    }
                    if name == "os_cpu_count" || name == "os_total_memory" || name == "os_pid"
                        || name == "sys_username" || name == "sys_hostname" || name == "sys_time_str"
                        || name == "fs_file_size" || name == "fs_delete_file" || name == "fs_copy_file"
                        || name == "str_slugify" || name == "str_capitalize" || name == "str_base64_encode"
                        || name == "math_pi" || name == "math_e" || name == "str_repeat" || name == "str_word_count"
                        || name == "degrees_to_radians" || name == "radians_to_degrees"
                        || name == "log2" || name == "fract"
                        || name == "str_trim_left" || name == "str_trim_right"
                        || name == "str_is_alpha" || name == "str_is_digit"
                        || name == "random_float" || name == "math_sign"
                        || name == "math_is_even" || name == "math_is_odd"
                        || name == "str_is_numeric" || name == "str_is_alnum"
                        || name == "math_lerp" || name == "str_is_lower" || name == "str_is_upper"
                        || name == "str_rev_words"
                        || name == "math_is_prime" || name == "math_gcd" || name == "math_lcm"
                        || name == "str_levenshtein"
                        || name == "str_first" || name == "str_last" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let resolved = match name {
                            "random_float" => "__york_random_float",
                            "math_sign" => "__york_math_sign",
                            "math_is_even" => "__york_math_is_even",
                            "math_is_odd" => "__york_math_is_odd",
                            "str_is_numeric" => "__york_str_is_numeric",
                            "str_is_alnum" => "__york_str_is_alnum",
                            "math_lerp" => "__york_math_lerp",
                            "str_is_lower" => "__york_str_is_lower",
                            "str_is_upper" => "__york_str_is_upper",
                            "str_rev_words" => "__york_str_rev_words",
                            "math_is_prime" => "__york_math_is_prime",
                            "math_gcd" => "__york_math_gcd",
                            "math_lcm" => "__york_math_lcm",
                            "str_levenshtein" => "__york_str_levenshtein",
                            "str_first" => "__york_str_first",
                            "str_last" => "__york_str_last",
                            "str_trim_left" => "__york_str_trim_left",
                            "str_trim_right" => "__york_str_trim_right",
                            "str_is_alpha" => "__york_str_is_alpha",
                            "str_is_digit" => "__york_str_is_digit",
                            "degrees_to_radians" => "__york_degrees_to_radians",
                            "radians_to_degrees" => "__york_radians_to_degrees",
                            "log2" => "__york_log2",
                            "fract" => "__york_fract",
                            "math_pi" => "__york_math_pi",
                            "math_e" => "__york_math_e",
                            "str_repeat" => "__york_str_repeat",
                            "str_word_count" => "__york_str_word_count",
                            "os_cpu_count" => "__york_os_cpu_count",
                            "os_total_memory" => "__york_os_total_memory",
                            "os_pid" => "__york_os_pid",
                            "sys_username" => "__york_sys_username",
                            "sys_hostname" => "__york_sys_hostname",
                            "sys_time_str" => "__york_sys_time_str",
                            "fs_file_size" => "__york_fs_file_size",
                            "fs_delete_file" => "__york_fs_delete_file",
                            "fs_copy_file" => "__york_fs_copy_file",
                            "str_slugify" => "__york_str_slugify",
                            "str_capitalize" => "__york_str_capitalize",
                            _ => "__york_str_base64_encode",
                        };
                        return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                    }
                    if name == "net_listen" || name == "net_accept" || name == "net_connect"
                        || name == "net_send" || name == "net_recv" || name == "net_close"
                        || name == "bin_pack" || name == "bin_unpack" || name == "crypto_hash" || name == "thread_spawn"
                        || name == "db_put" || name == "db_get"
                        || name == "json_get" || name == "discord_send" || name == "discord_listen_event"
                        || name == "crypto_sha256" || name == "regex_match" || name == "http_get"
                        || name == "thread_join" || name == "thread_self" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let resolved = match name {
                            "net_listen" => "__york_net_listen",
                            "net_accept" => "__york_net_accept",
                            "net_connect" => "__york_net_connect",
                            "net_send" => "__york_net_send",
                            "net_recv" => "__york_net_recv",
                            "net_close" => "__york_net_close",
                            "bin_pack" => "__york_bin_pack",
                            "bin_unpack" => "__york_bin_unpack",
                            "crypto_hash" => "__york_crypto_hash",
                            "db_put" => "__york_db_put",
                            "db_get" => "__york_db_get",
                            "json_get" => "__york_json_get",
                            "discord_send" => "__york_discord_send",
                            "discord_listen_event" => "__york_discord_listen_event",
                            "crypto_sha256" => "__york_crypto_sha256",
                            "regex_match" => "__york_regex_match",
                            "http_get" => "__york_http_get",
                            "thread_spawn" => "__york_thread_spawn",
                            "thread_join" => "__york_thread_join",
                            _ => "__york_thread_self",
                        };
                        return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                    }
                    if name == "window_create" || name == "window_show" || name == "window_hide" || name == "window_close"
                        || name == "window_is_open" || name == "window_poll_events" || name == "window_run_loop"
                        || name == "window_last_command" || name == "message_box"
                        || name == "control_button" || name == "control_label" || name == "control_textbox" || name == "control_set_text" {
                        let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                        let resolved = match name {
                            "window_create" => "__york_window_create",
                            "window_show" => "__york_window_show",
                            "window_hide" => "__york_window_hide",
                            "window_close" => "__york_window_close",
                            "window_is_open" => "__york_window_is_open",
                            "window_poll_events" => "__york_window_poll_events",
                            "window_run_loop" => "__york_window_run_loop",
                            "window_last_command" => "__york_window_last_command",
                            "message_box" => "__york_message_box",
                            "control_button" => "__york_control_button",
                            "control_label" => "__york_control_label",
                            "control_textbox" => "__york_control_textbox",
                            _ => "__york_control_set_text",
                        };
                        return hir::Expr::Call { callee: name.to_string(), resolved: resolved.to_string(), args: lowered_args };
                    }
                }
                let resolved = self.resolve_callee(callee, scope);
                // A bare identifier with no known function is an undefined call.
                if let AstExpr::Ident(path) = &callee.node {
                    let name = path.segments.last().map(|s| s.node.as_str()).unwrap_or("");
                    if name != "main" && !self.tables.functions.contains_key(name) {
                        errors.push(SemanticErrorWithSpan {
                            error: SemanticError::UndefinedFunction(name.to_string()),
                            span: callee.span,
                        });
                    }
                }
                let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                let callee_name = self.expr_name(callee);
                hir::Expr::Call { callee: callee_name, resolved, args: lowered_args }
            }

            AstExpr::MethodCall { receiver, method, args } => {
                let recv = self.lower_expr(receiver, scope, errors);
                let recv_ty = self.infer_expr_type(&recv, scope);
                let recv_ty_for_mangle = recv_ty.clone();
                let method_name = method.node.clone();
                let resolved = if matches!(recv_ty, Some(Ty::Str)) {
                    match method_name.as_str() {
                        "len" | "length" | "size" => "__york_strlen".to_string(),
                        "contains" => "__york_str_contains".to_string(),
                        "startsWith" | "starts_with" => "__york_str_starts".to_string(),
                        "endsWith" | "ends_with" => "__york_str_ends".to_string(),
                        "toUpper" | "uppercase" | "upper" => "__york_str_upper".to_string(),
                        "toLower" | "lowercase" | "lower" => "__york_str_lower".to_string(),
                        "trim" => "__york_str_trim".to_string(),
                        "substring" | "substr" => "__york_str_sub".to_string(),
                        "indexOf" | "index_of" | "find" => "__york_str_index".to_string(),
                        "lastIndexOf" | "last_index_of" | "rfind" => "__york_str_last_index".to_string(),
                        "charAt" | "char_at" => "__york_str_char_at".to_string(),
                        "repeat" => "__york_str_repeat".to_string(),
                        "isEmpty" | "is_empty" => "__york_str_empty".to_string(),
                        "replace" => "__york_str_replace".to_string(),
                        "reverse" => "__york_str_reverse".to_string(),
                        "trimStart" | "trim_start" => "__york_str_trim_start".to_string(),
                        "trimEnd" | "trim_end" => "__york_str_trim_end".to_string(),
                        "padLeft" | "pad_left" | "ljust" => "__york_str_pad_left".to_string(),
                        "padRight" | "pad_right" | "rjust" => "__york_str_pad_right".to_string(),
                        "count" => "__york_str_count".to_string(),
                        other => {
                            errors.push(SemanticErrorWithSpan {
                                error: SemanticError::NoMethod {
                                    method: other.to_string(),
                                    ty: "String".to_string(),
                                },
                                span: method.span,
                            });
                            "__york_strlen".to_string()
                        }
                    }
                } else {
                    self.mangle_method(&recv_ty_for_mangle.unwrap_or(Ty::Inferred), &method_name)
                };
                let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                // Fixed-size arrays lower to plain C: no runtime helper needed.
                if let Some(Ty::Array(_elem, n)) = recv_ty.clone() {
                    match method_name.as_str() {
                        "len" | "length" | "count" => return hir::Expr::Int(n as i64),
                        "is_empty" | "isEmpty" => return hir::Expr::Bool(n == 0),
                        "at" | "get" => {
                            let idx = lowered_args.into_iter().next().unwrap_or(hir::Expr::Null);
                            return hir::Expr::Index {
                                object: Box::new(recv),
                                index: Box::new(idx),
                            };
                        }
                        "first" => {
                            return hir::Expr::Index {
                                object: Box::new(recv),
                                index: Box::new(hir::Expr::Int(0)),
                            }
                        }
                        "last" => {
                            return hir::Expr::Index {
                                object: Box::new(recv),
                                index: Box::new(hir::Expr::Int(n as i64 - 1)),
                            }
                        }
                        _ => {}
                    }
                    let _ = _elem;
                }
                hir::Expr::MethodCall {
                    receiver: Box::new(recv),
                    method: method_name,
                    resolved,
                    args: lowered_args,
                }
            }

            AstExpr::Field { object, field } => {
                // Enum variant reference: `Color.Red` parses as a field access on an
                // identifier that names a known enum.
                if let AstExpr::Ident(path) = &object.node {
                    let enum_name = path.segments.last().map(|s| s.node.as_str()).unwrap_or("");
                    if self.tables.enums.contains_key(enum_name) {
                        let variant = field.node.clone();
                        let valid = self
                            .tables
                            .enums
                            .get(enum_name)
                            .map(|info| info.variants.iter().any(|(n, _)| *n == variant))
                            .unwrap_or(false);
                        if valid {
                            return hir::Expr::EnumRef {
                                enum_name: enum_name.to_string(),
                                variant,
                            };
                        }
                    }
                }
                let o = self.lower_expr(object, scope, errors);
                hir::Expr::Field { object: Box::new(o), field: field.node.clone() }
            }

            AstExpr::Index { object, index } => {
                let o = self.lower_expr(object, scope, errors);
                let i = self.lower_expr(index, scope, errors);
                hir::Expr::Index { object: Box::new(o), index: Box::new(i) }
            }

            AstExpr::New { ty, args } => {
                let struct_name = match &ty.node {
                    TypeAnnotation::Named(p) => p.segments.last().map(|s| s.node.clone()).unwrap_or_default(),
                    // `new HashMap<string, int>()` — keep the generic base name so the
                    // backend can recognise the built-in collection constructor.
                    TypeAnnotation::Generic { base, .. } => {
                        base.segments.last().map(|s| s.node.clone()).unwrap_or_default()
                    }
                    TypeAnnotation::Arena(_) => "Arena".into(),
                    _ => "Unknown".into(),
                };
                let default_fields = self.tables.structs.get(&struct_name)
                    .map(|s| s.fields.iter().map(|(n, t)| hir::FieldDef {
                        name: n.clone(), ty: t.clone(), default: None
                    }).collect())
                    .unwrap_or_default();
                let lowered_args: Vec<hir::Expr> = args.iter().map(|a| self.lower_expr(a, scope, errors)).collect();
                hir::Expr::New { struct_name, defaults: default_fields, args: lowered_args }
            }

            AstExpr::StructLiteral { path, fields } => {
                let struct_name = path.segments.last().map(|s| s.node.clone()).unwrap_or_default();
                let field_types: Vec<(String, Ty)> = self
                    .tables
                    .structs
                    .get(&struct_name)
                    .map(|s| s.fields.clone())
                    .unwrap_or_default();
                let mut lowered_fields = Vec::with_capacity(fields.len());
                for (name, val) in fields {
                    let mut e = self.lower_expr(val, scope, errors);
                    if let Some((_, fty)) = field_types.iter().find(|(n, _)| *n == name.node) {
                        let fty = fty.clone();
                        self.coerce_array_literal(&mut e, &fty, val.span, errors);
                    }
                    lowered_fields.push((name.node.clone(), e));
                }
                hir::Expr::StructLiteral { struct_name, fields: lowered_fields }
            }

            AstExpr::If(if_expr) => {
                let condition = self.lower_expr(&if_expr.condition, scope, errors);
                let mut then_scope = scope.child();
                let then_stmts = self.lower_block(&if_expr.then_branch, &mut then_scope, errors);
                let else_stmts = match &if_expr.else_branch {
                    Some(york_ast::ElseBranch::Block(b)) => {
                        let mut es = scope.child();
                        self.lower_block(b, &mut es, errors)
                    }
                    Some(york_ast::ElseBranch::If(_)) | None => Vec::new(),
                };
                let as_expr = |stmts: &[hir::Stmt]| -> Option<hir::Expr> {
                    match stmts {
                        [hir::Stmt::Expr(e)] => Some(e.clone()),
                        _ => None,
                    }
                };
                match (as_expr(&then_stmts), as_expr(&else_stmts)) {
                    (Some(then), Some(else_)) => hir::Expr::Ternary {
                        condition: Box::new(condition),
                        then: Box::new(then),
                        else_: Box::new(else_),
                    },
                    _ => {
                        errors.push(SemanticErrorWithSpan {
                            error: SemanticError::NotSupported(
                                "if-expressions with statement branches (use a plain if statement)".into(),
                            ),
                            span: expr.span,
                        });
                        hir::Expr::Null
                    }
                }
            }

            AstExpr::Block(b) => {
                let mut child_scope = scope.child();
                let stmts = self.lower_block(b, &mut child_scope, errors);
                // Return last expression if any, else void
                if let Some(last) = stmts.last() {
                    match last {
                        hir::Stmt::Expr(e) => e.clone(),
                        _ => hir::Expr::Null,
                    }
                } else {
                    hir::Expr::Null
                }
            }

            AstExpr::Array(elems) => {
                // `[a, b, c]` — a fixed-size array whose length is known here.
                // Element type comes from the first element; an empty literal
                // adopts the declared variable type.
                let lowered = elems
                    .iter()
                    .map(|e| self.lower_expr(e, scope, errors))
                    .collect::<Vec<_>>();
                let elem_ty = lowered
                    .first()
                    .and_then(|e| self.infer_expr_type(e, scope))
                    .unwrap_or(Ty::I32);
                let n = lowered.len();
                hir::Expr::ArrayLit { elem_ty, elems: lowered, len: n }
            }

            AstExpr::ArrayRepeat { value, count } => {
                // `[value; count]` — expands the repeat at compile time.
                let n = match &count.node {
                    AstExpr::Int(n) if *n >= 0 => *n as usize,
                    _ => {
                        errors.push(SemanticErrorWithSpan {
                            error: SemanticError::NotSupported(
                                "array repeat count must be a non-negative literal".into(),
                            ),
                            span: count.span,
                        });
                        0
                    }
                };
                let v = self.lower_expr(value, scope, errors);
                let elem_ty = self
                    .infer_expr_type(&v, scope)
                    .unwrap_or(Ty::I32);
                hir::Expr::ArrayLit {
                    elem_ty,
                    elems: (0..n).map(|_| v.clone()).collect(),
                    len: n,
                }
            }

            AstExpr::Tuple(_elems) => {
                errors.push(SemanticErrorWithSpan {
                    error: SemanticError::NotSupported("tuples".into()),
                    span: expr.span,
                });
                hir::Expr::Null
            }

            AstExpr::Match { .. } => {
                errors.push(SemanticErrorWithSpan {
                    error: SemanticError::NotSupported("match expressions".into()),
                    span: expr.span,
                });
                hir::Expr::Null
            }
            AstExpr::Closure { .. } => {
                errors.push(SemanticErrorWithSpan {
                    error: SemanticError::NotSupported("closures".into()),
                    span: expr.span,
                });
                hir::Expr::Null
            }
            AstExpr::Cast { expr: inner, ty } => {
                // `(T) e` — a reinterpretation of the value's type.
                let target = self.resolve_type(&ty.node);
                let lowered = self.lower_expr(inner, scope, errors);
                if let Some(from) = self.infer_expr_type(&lowered, scope) {
                    let ok = from.is_numeric() && target.is_numeric()
                        || matches!(from, Ty::Slice(_)) || matches!(target, Ty::Slice(_))
                        || matches!(from, Ty::Struct(_)) || matches!(target, Ty::Struct(_))
                        || matches!(from, Ty::Enum(_)) || matches!(target, Ty::Enum(_))
                        || matches!(from, Ty::Str) || matches!(target, Ty::Str)
                        || matches!(from, Ty::Bool) || matches!(target, Ty::Bool)
                        || matches!(from, Ty::Inferred) || matches!(target, Ty::Inferred);
                    if !ok {
                        errors.push(SemanticErrorWithSpan {
                            error: SemanticError::TypeMismatch {
                                expected: format!("{target}"),
                                found: format!("{from}"),
                            },
                            span: expr.span,
                        });
                    }
                }
                hir::Expr::Cast {
                    ty: target,
                    expr: Box::new(lowered),
                }
            }
            AstExpr::Sizeof(ty) => hir::Expr::Sizeof(self.resolve_type(&ty.node)),
            AstExpr::Alignof(ty) => hir::Expr::Alignof(self.resolve_type(&ty.node)),
            AstExpr::TypeAnnotation { expr: inner, ty } => {
                // Ascription: verify the value matches, then keep the value.
                let want = self.resolve_type(&ty.node);
                let lowered = self.lower_expr(inner, scope, errors);
                if let Some(g) = self.infer_expr_type(&lowered, scope) {
                    if want != g && want.is_numeric() && !g.is_numeric() {
                        errors.push(SemanticErrorWithSpan {
                            error: SemanticError::TypeMismatch {
                                expected: format!("{want}"),
                                found: format!("{g}"),
                            },
                            span: expr.span,
                        });
                    }
                }
                lowered
            }
            AstExpr::ArenaAlloc { .. } => {
                errors.push(SemanticErrorWithSpan {
                    error: SemanticError::NotSupported("explicit arena allocation".into()),
                    span: expr.span,
                });
                hir::Expr::Null
            }
        }
    }

    /// Render any expression as a `String` expression, converting numbers and
    /// booleans via the runtime `to_string` helpers.
    fn as_string_expr(&self, e: hir::Expr, scope: &Scope) -> hir::Expr {
        match self.infer_expr_type(&e, scope) {
            Some(Ty::Str) => e,
            Some(Ty::F32 | Ty::F64) => hir::Expr::Call {
                callee: "to_string".into(),
                resolved: "__york_to_string_float".into(),
                args: vec![e],
            },
            Some(Ty::Bool) => hir::Expr::Call {
                callee: "to_string".into(),
                resolved: "__york_to_string_bool".into(),
                args: vec![e],
            },
            _ => hir::Expr::Call {
                callee: "to_string".into(),
                resolved: "__york_to_string_int".into(),
                args: vec![e],
            },
        }
    }

    fn expr_name(&self, expr: &Spanned<AstExpr>) -> String {
        match &expr.node {
            AstExpr::Ident(path) => path.segments.last().map(|s| s.node.clone()).unwrap_or_default(),
            _ => "__expr__".into(),
        }
    }

    fn resolve_callee(&self, callee: &Spanned<AstExpr>, _scope: &Scope) -> String {
        match &callee.node {
            AstExpr::Ident(path) => {
                let name = path.segments.last().map(|s| s.node.as_str()).unwrap_or("?");
                if self.tables.functions.contains_key(name) {
                    name.to_string()
                } else {
                    name.to_string() // Hope it exists
                }
            }
            _ => "__unknown__".into(),
        }
    }

    fn infer_expr_type(&self, expr: &hir::Expr, scope: &Scope) -> Option<Ty> {
        match expr {
            hir::Expr::Int(_v) => Some(Ty::I64),
            hir::Expr::Float(_) => Some(Ty::F64),
            hir::Expr::Str(_) => Some(Ty::Str),
            hir::Expr::Char(_) => Some(Ty::Char),
            hir::Expr::Bool(_) => Some(Ty::Bool),
            hir::Expr::Null => None,
            hir::Expr::This => scope.lookup("self"),
            hir::Expr::Var(name) => scope.lookup(name),
            hir::Expr::EnumRef { enum_name, .. } => Some(Ty::Enum(enum_name.clone())),
            hir::Expr::StructLiteral { struct_name, .. } => Some(Ty::Struct(struct_name.clone())),
            hir::Expr::New { struct_name, .. } => Some(Ty::Struct(struct_name.clone())),
            hir::Expr::Cast { ty, .. } => Some(ty.clone()),
            hir::Expr::Sizeof(_) | hir::Expr::Alignof(_) => Some(Ty::I64),
            hir::Expr::ArrayLit { elem_ty, len, .. } => Some(Ty::Array(Box::new(elem_ty.clone()), *len)),
            hir::Expr::Field { object, field } => {
                let recv_ty = self.infer_expr_type(object, scope)?;
                if let Ty::Struct(name) = recv_ty {
                    if let Some(info) = self.tables.structs.get(&name) {
                        return info.fields.iter().find(|(n, _)| n == field).map(|(_, t)| t.clone());
                    }
                }
                None
            }
            hir::Expr::MethodCall { receiver, method, resolved, .. } => {
                if matches!(self.infer_expr_type(receiver, scope), Some(Ty::Str)) {
                    return match method.as_str() {
                        "len" | "length" | "size" => Some(Ty::I64),
                        "contains" | "startsWith" | "starts_with" | "endsWith" | "ends_with"
                        | "isEmpty" | "is_empty" => Some(Ty::Bool),
                        "indexOf" | "index_of" | "find" | "lastIndexOf" | "last_index_of" | "rfind"
                        | "count" => {
                            Some(Ty::I64)
                        }
                        _ => Some(Ty::Str),
                    };
                }
                // If receiver is an arena, resolve get/count/push/pop/last/clear/is_empty semantics
                if let Some(recv_ty) = self.infer_expr_type(receiver, scope) {
                    if let Ty::Arena(elem) = recv_ty {
                        return match method.as_str() {
                            "get" | "at" | "push" | "pop" | "last" => Some((*elem).clone()),
                            "len" | "length" | "count" => Some(Ty::I64),
                            "is_empty" | "isEmpty" => Some(Ty::Bool),
                            "clear" | "set" => Some(Ty::Void),
                            _ => self.tables.functions.get(resolved).map(|f| f.return_ty.clone()),
                        };
                    }
                    if let Ty::Slice(inner) = recv_ty {
                        return match method.as_str() {
                            "len" | "length" | "count" => Some(Ty::I64),
                            "get" | "at" => Some((*inner).clone()),
                            _ => self.tables.functions.get(resolved).map(|f| f.return_ty.clone()),
                        };
                    }
                    if let Ty::Array(inner, _) = &recv_ty {
                        return match method.as_str() {
                            "len" | "length" | "count" => Some(Ty::I64),
                            "get" | "at" => Some((**inner).clone()),
                            "first" => Some((**inner).clone()),
                            "last" => Some((**inner).clone()),
                            "is_empty" | "isEmpty" => Some(Ty::Bool),
                            _ => self.tables.functions.get(resolved).map(|f| f.return_ty.clone()),
                        };
                    }
                    if let Ty::HashMap(_k, v) = recv_ty {
                        return match method.as_str() {
                            "get" | "get_or" | "getOr" | "remove" | "removeKey" => Some((*v).clone()),
                            "count" | "len" | "length" | "size" => Some(Ty::I64),
                            "contains" | "containsKey" | "contains_key" | "isEmpty" | "is_empty"
                            | "has" => Some(Ty::Bool),
                            _ => Some(Ty::Void),
                        };
                    }
                }
                self.tables.functions.get(resolved).map(|f| f.return_ty.clone())
            }
            hir::Expr::Call { resolved, .. } => {
                if resolved == "__york_read_file" || resolved == "__york_to_string_int"
                    || resolved == "__york_to_string_float" || resolved == "__york_to_string_bool"
                    || resolved == "__york_read_line"
                    || resolved == "__york_str_upper" || resolved == "__york_str_lower"
                    || resolved == "__york_str_trim" || resolved == "__york_str_sub" {
                    return Some(Ty::Str);
                }
                if resolved == "__york_file_exists"
                    || resolved == "__york_str_contains"
                    || resolved == "__york_str_starts" || resolved == "__york_str_ends"
                    || resolved == "__york_str_empty"
                    || resolved == "__york_str_is_alpha" || resolved == "__york_str_is_digit"
                    || resolved == "__york_math_is_even" || resolved == "__york_math_is_odd"
                    || resolved == "__york_str_is_numeric" || resolved == "__york_str_is_alnum"
                    || resolved == "__york_str_is_lower" || resolved == "__york_str_is_upper"
                    || resolved == "__york_math_is_prime" {
                    return Some(Ty::Bool);
                }
                if resolved == "__york_strlen" || resolved == "__york_read_int"
                    || resolved == "__york_random_range" || resolved == "__york_str_count"
                    || resolved == "__york_window_create" || resolved == "__york_window_last_command"
                    || resolved == "__york_control_button" || resolved == "__york_control_label"
                    || resolved == "__york_control_textbox"
                    || resolved == "__york_net_listen" || resolved == "__york_net_accept"
                    || resolved == "__york_net_connect" || resolved == "__york_net_send"
                    || resolved == "__york_bin_pack" || resolved == "__york_thread_spawn"
                    || resolved == "__york_os_cpu_count" || resolved == "__york_os_total_memory"
                    || resolved == "__york_os_pid" || resolved == "__york_fs_file_size"
                    || resolved == "__york_str_word_count"
                    || resolved == "__york_thread_join" || resolved == "__york_thread_self"
                    || resolved == "__york_math_gcd" || resolved == "__york_math_lcm"
                    || resolved == "__york_str_levenshtein" {
                    return Some(Ty::I64);
                }
                if resolved == "__york_window_is_open" || resolved == "__york_fs_delete_file"
                    || resolved == "__york_fs_copy_file" || resolved == "__york_discord_send"
                    || resolved == "__york_regex_match" {
                    return Some(Ty::Bool);
                }
                if resolved == "__york_window_show" || resolved == "__york_window_hide"
                    || resolved == "__york_window_close" || resolved == "__york_window_poll_events"
                    || resolved == "__york_window_run_loop" || resolved == "__york_message_box"
                    || resolved == "__york_control_set_text"
                    || resolved == "__york_net_close" || resolved == "__york_bin_unpack" {
                    return Some(Ty::Void);
                }
                if resolved == "__york_net_recv" || resolved == "__york_crypto_hash" || resolved == "__york_db_get"
                    || resolved == "__york_json_get" || resolved == "__york_discord_listen_event"
                    || resolved == "__york_crypto_sha256" || resolved == "__york_http_get"
                    || resolved == "__york_sys_username" || resolved == "__york_sys_hostname"
                    || resolved == "__york_sys_time_str" || resolved == "__york_str_slugify"
                    || resolved == "__york_str_capitalize" || resolved == "__york_str_base64_encode"
                    || resolved == "__york_str_repeat"
                    || resolved == "__york_str_trim_left" || resolved == "__york_str_trim_right"
                    || resolved == "__york_str_first" || resolved == "__york_str_last"
                    || resolved == "__york_str_rev_words" {
                    return Some(Ty::Str);
                }
                if resolved == "__york_to_int" {
                    return Some(Ty::I64);
                }
                if resolved == "__york_to_float" {
                    return Some(Ty::F64);
                }
                if resolved == "rand" {
                    return Some(Ty::I32);
                }
                if resolved == "srand" || resolved == "__york_assert" {
                    return Some(Ty::Void);
                }
                if resolved == "sqrt" || resolved == "pow" || resolved == "floor" || resolved == "ceil"
                    || resolved == "round" || resolved == "fabs"
                    || resolved == "sin" || resolved == "cos" || resolved == "tan"
                    || resolved == "log" || resolved == "log10" || resolved == "exp"
                    || resolved == "__york_math_pi" || resolved == "__york_math_e"
                    || resolved == "__york_degrees_to_radians" || resolved == "__york_radians_to_degrees"
                    || resolved == "__york_log2" || resolved == "__york_fract"
                    || resolved == "__york_random_float" || resolved == "__york_math_sign"
                    || resolved == "__york_math_lerp" {
                    return Some(Ty::F64);
                }
                if resolved == "__york_env" || resolved == "__york_platform_name" {
                    return Some(Ty::Str);
                }
                if resolved.starts_with("__york_math_min")
                    || resolved.starts_with("__york_math_max")
                    || resolved.starts_with("__york_math_clamp") {
                    return if resolved.ends_with("_f") { Some(Ty::F64) } else { Some(Ty::I64) };
                }
                if resolved == "__york_sleep" {
                    return Some(Ty::Void);
                }
                if resolved == "__york_now" {
                    return Some(Ty::I64);
                }
                if resolved == "__york_str_index" || resolved == "__york_str_last_index" {
                    return Some(Ty::I64);
                }
                if resolved == "llabs" || resolved == "__york_exec" {
                    return Some(Ty::I32);
                }
                self.tables.functions.get(resolved).map(|f| f.return_ty.clone())
            }
            hir::Expr::Binary { op, left, .. } => {
                match op {
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge
                    | BinOp::And | BinOp::Or => Some(Ty::Bool),
                    _ => self.infer_expr_type(left, scope),
                }
            }
            hir::Expr::Strcat { .. } => Some(Ty::Str),
            hir::Expr::Streq { .. } => Some(Ty::Bool),
            hir::Expr::Incr { operand, .. } => self.infer_expr_type(operand, scope),
            hir::Expr::Index { object, .. } => {
                match self.infer_expr_type(object, scope)? {
                    Ty::Slice(inner) => Some(*inner),
                    Ty::Array(inner, _) => Some(*inner),
                    Ty::Arena(inner) => Some(*inner),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
