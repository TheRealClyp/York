use crate::ast::*;
use crate::span::Spanned;

/// Visitor trait for walking the AST.
/// Each `walk_*` function provides a default traversal;
/// implementors can override specific node visits.
pub trait Visitor: Sized {
    fn visit_program(&mut self, program: &Program) {
        walk_program(self, program);
    }

    fn visit_item(&mut self, item: &Spanned<Item>) {
        walk_item(self, item);
    }

    fn visit_function(&mut self, func: &FunctionDef) {
        walk_function(self, func);
    }

    fn visit_struct(&mut self, s: &StructDef) {
        for field in &s.fields {
            self.visit_type_annotation(&field.node.ty);
        }
    }

    fn visit_enum(&mut self, e: &EnumDef) {
        for variant in &e.variants {
            for ty in &variant.node.fields {
                self.visit_type_annotation(ty);
            }
        }
    }

    fn visit_impl(&mut self, imp: &ImplBlock) {
        self.visit_type_annotation(&imp.self_type);
        for method in &imp.methods {
            self.visit_function(&method.node);
        }
    }

    fn visit_trait(&mut self, _t: &TraitDef) {}

    fn visit_import(&mut self, _i: &ImportDecl) {}

    fn visit_const(&mut self, c: &ConstDecl) {
        self.visit_expr(&c.value);
    }

    fn visit_stmt(&mut self, stmt: &Spanned<Stmt>) {
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &Spanned<Expr>) {
        walk_expr(self, expr);
    }

    fn visit_type_annotation(&mut self, ty: &Spanned<TypeAnnotation>) {
        walk_type_annotation(self, ty);
    }
}

pub fn walk_type_annotation(v: &mut impl Visitor, ty: &Spanned<TypeAnnotation>) {
    match &ty.node {
        TypeAnnotation::Pointer(i) | TypeAnnotation::MutPointer(i)
        | TypeAnnotation::Reference(i) | TypeAnnotation::MutReference(i)
        | TypeAnnotation::Optional(i) | TypeAnnotation::Slice(i)
        | TypeAnnotation::Arena(i) => {
            v.visit_type_annotation(i);
        }
        TypeAnnotation::Generic { args, .. } => {
            for arg in args {
                v.visit_type_annotation(arg);
            }
        }
        TypeAnnotation::Array(elem, _) => {
            v.visit_type_annotation(elem);
        }
        TypeAnnotation::Tuple(elems) => {
            for e in elems {
                v.visit_type_annotation(e);
            }
        }
        TypeAnnotation::Result(ok, err) => {
            v.visit_type_annotation(ok);
            v.visit_type_annotation(err);
        }
        TypeAnnotation::Function(params, ret) => {
            for param in params {
                v.visit_type_annotation(param);
            }
            v.visit_type_annotation(ret);
        }
        _ => {}
    }
}

pub fn walk_program(v: &mut impl Visitor, program: &Program) {
    for item in &program.items {
        v.visit_item(item);
    }
}

pub fn walk_item(v: &mut impl Visitor, item: &Spanned<Item>) {
    match &item.node {
        Item::Function(f) => v.visit_function(f),
        Item::Struct(s) => v.visit_struct(s),
        Item::Enum(e) => v.visit_enum(e),
        Item::Impl(i) => v.visit_impl(i),
        Item::Trait(t) => v.visit_trait(t),
        Item::Import(i) => v.visit_import(i),
        Item::Const(c) => v.visit_const(c),
        Item::Static(s) => {
            v.visit_type_annotation(&s.ty);
            if let Some(ref val) = s.value {
                v.visit_expr(val);
            }
        }
        Item::TypeAlias(t) => {
            v.visit_type_annotation(&t.ty);
        }
    }
}

pub fn walk_function(v: &mut impl Visitor, func: &FunctionDef) {
    for param in &func.params {
        v.visit_type_annotation(&param.node.ty);
        if let Some(ref default) = param.node.default {
            v.visit_expr(default);
        }
    }
    if let Some(ref ret) = func.return_type {
        v.visit_type_annotation(ret);
    }
    if let Some(ref body) = func.body {
        for stmt in &body.stmts {
            v.visit_stmt(stmt);
        }
        if let Some(ref result) = body.result {
            v.visit_expr(result);
        }
    }
}

pub fn walk_stmt(v: &mut impl Visitor, stmt: &Spanned<Stmt>) {
    match &stmt.node {
        Stmt::Let { ty, value, .. } => {
            if let Some(ty) = ty {
                v.visit_type_annotation(ty);
            }
            if let Some(val) = value {
                v.visit_expr(val);
            }
        }
        Stmt::Var { ty, value, .. } => {
            if let Some(ty) = ty {
                v.visit_type_annotation(ty);
            }
            v.visit_expr(value);
        }
        Stmt::Expr(e) => v.visit_expr(e),
        Stmt::Return(Some(e)) => v.visit_expr(e),
        Stmt::Defer(e) => v.visit_expr(e),
        Stmt::Block(block) => {
            for stmt in &block.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = block.result {
                v.visit_expr(result);
            }
        }
        Stmt::If(if_expr) => {
            v.visit_expr(&if_expr.condition);
            for stmt in &if_expr.then_branch.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = if_expr.then_branch.result {
                v.visit_expr(result);
            }
            match &if_expr.else_branch {
                Some(ElseBranch::If(else_if)) => {
                    v.visit_stmt(&Spanned::new(
                        Stmt::If((**else_if).clone()),
                        stmt.span,
                    ));
                }
                Some(ElseBranch::Block(block)) => {
                    for stmt in &block.stmts {
                        v.visit_stmt(stmt);
                    }
                }
                None => {}
            }
        }
        Stmt::While { condition, body } => {
            v.visit_expr(condition);
            for stmt in &body.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = body.result {
                v.visit_expr(result);
            }
        }
        Stmt::For { iterable, body, .. } => {
            v.visit_expr(iterable);
            for stmt in &body.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = body.result {
                v.visit_expr(result);
            }
        }
        Stmt::ForC { init, condition, update, body } => {
            if let Some(init) = init {
                v.visit_stmt(init);
            }
            if let Some(cond) = condition {
                v.visit_expr(cond);
            }
            for stmt in &body.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = body.result {
                v.visit_expr(result);
            }
            let _ = update;
        }
        Stmt::Loop { body } => {
            for stmt in &body.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = body.result {
                v.visit_expr(result);
            }
        }
        Stmt::Return(None) => {}
        Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Switch { scrutinee, arms } => {
            v.visit_expr(scrutinee);
            for arm in arms {
                if let Some(label) = &arm.label {
                    v.visit_expr(label);
                }
                for stmt in &arm.body {
                    v.visit_stmt(stmt);
                }
            }
        }
    }
}

pub fn walk_expr(v: &mut impl Visitor, expr: &Spanned<Expr>) {
    match &expr.node {
        Expr::Binary { left, right, .. } => {
            v.visit_expr(left);
            v.visit_expr(right);
        }
        Expr::Unary { operand, .. } => v.visit_expr(operand),
        Expr::Incr { operand, .. } => v.visit_expr(operand),
        Expr::Ternary { condition, then_branch, else_branch } => {
            v.visit_expr(condition);
            v.visit_expr(then_branch);
            v.visit_expr(else_branch);
        }
        Expr::Assign { target, value } => {
            v.visit_expr(target);
            v.visit_expr(value);
        }
        Expr::CompoundAssign { target, value, .. } => {
            v.visit_expr(target);
            v.visit_expr(value);
        }
        Expr::Call { callee, args } => {
            v.visit_expr(callee);
            for arg in args {
                v.visit_expr(arg);
            }
        }
        Expr::MethodCall { receiver, args, .. } => {
            v.visit_expr(receiver);
            for arg in args {
                v.visit_expr(arg);
            }
        }
        Expr::Field { object, .. } => v.visit_expr(object),
        Expr::Index { object, index } => {
            v.visit_expr(object);
            v.visit_expr(index);
        }
        Expr::Array(exprs) | Expr::Tuple(exprs) => {
            for e in exprs {
                v.visit_expr(e);
            }
        }
        Expr::ArrayRepeat { value, count } => {
            v.visit_expr(value);
            v.visit_expr(count);
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, val) in fields {
                v.visit_expr(val);
            }
        }
        Expr::If(if_expr) => {
            v.visit_expr(&if_expr.condition);
            for stmt in &if_expr.then_branch.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = if_expr.then_branch.result {
                v.visit_expr(result);
            }
        }
        Expr::Block(block) => {
            for stmt in &block.stmts {
                v.visit_stmt(stmt);
            }
            if let Some(ref result) = block.result {
                v.visit_expr(result);
            }
        }
        Expr::Match { scrutinee, arms } => {
            v.visit_expr(scrutinee);
            for arm in arms {
                if let Some(ref guard) = arm.guard {
                    v.visit_expr(guard);
                }
                v.visit_expr(&arm.body);
            }
        }
        Expr::Closure { return_type, body, .. } => {
            if let Some(rt) = return_type {
                v.visit_type_annotation(rt);
            }
            match body.as_ref() {
                ClosureBody::Expr(e) => v.visit_expr(e),
                ClosureBody::Block(block) => {
                    for stmt in &block.stmts {
                        v.visit_stmt(stmt);
                    }
                }
            }
        }
        Expr::Cast { expr, ty } => {
            v.visit_expr(expr);
            v.visit_type_annotation(ty);
        }
        Expr::Sizeof(ty) | Expr::Alignof(ty) => {
            v.visit_type_annotation(ty);
        }
        Expr::TypeAnnotation { expr, ty } => {
            v.visit_expr(expr);
            v.visit_type_annotation(ty);
        }
        Expr::ArenaAlloc { expr } => v.visit_expr(expr),
        Expr::New { ty, args } => {
            v.visit_type_annotation(ty);
            for arg in args {
                v.visit_expr(arg);
            }
        }
        Expr::This => {}
        Expr::Int(_)
        | Expr::Float(_)
        | Expr::String(_)
        | Expr::Char(_)
        | Expr::Bool(_)
        | Expr::Null
        | Expr::Ident(_) => {}
    }
}
