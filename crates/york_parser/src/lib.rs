pub mod error;
pub mod expr;
pub mod types;

use york_ast::span::{BytePos, Span, Spanned};
use york_ast::{Block, EnumDef, FunctionDef, GenericParam, ImportDecl, Item, Param, Path,
               Program, StaticDecl, StructDef, TraitDef, TypeAliasDecl, TypeAnnotation,
               Visibility, SpannedToken};
use york_ast::Token;

use crate::error::{ParseErrorWithSpan, span_for, token_name};
use crate::types::parse_type;

pub struct Parser<'a> {
    tokens: &'a [SpannedToken],
    pos: usize,
    prev_span: Span,
    pub errors: Vec<ParseErrorWithSpan>,
}

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub program: Program,
    pub errors: Vec<ParseErrorWithSpan>,
}

pub fn parse(tokens: &[SpannedToken]) -> ParseResult {
    let mut parser = Parser {
        tokens,
        pos: 0,
        prev_span: Span::new(BytePos::ZERO, BytePos::ZERO),
        errors: Vec::new(),
    };
    let program = parser.parse_program();
    ParseResult {
        program,
        errors: parser.errors,
    }
}

impl<'a> Parser<'a> {
    pub(crate) fn pos(&self) -> usize { self.pos }
    pub(crate) fn set_pos(&mut self, pos: usize) { self.pos = pos; }
    pub(crate) fn previous_span(&self) -> Span { self.prev_span }

    pub(crate) fn peek(&self) -> &SpannedToken {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    pub(crate) fn peek_n(&self, n: usize) -> &SpannedToken {
        let idx = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    pub(crate) fn peek_tok(&self) -> &Token { &self.peek().node }

    pub(crate) fn peek_span(&self) -> Span { self.peek().span }

    pub(crate) fn advance(&mut self) -> SpannedToken {
        let idx = self.pos.min(self.tokens.len() - 1);
        let tok = self.tokens[idx].clone();
        self.prev_span = tok.span;
        if self.pos < self.tokens.len() - 1 { self.pos += 1; }
        tok
    }

    pub(crate) fn at(&self, tok: &Token) -> bool { self.peek_tok() == tok }

    pub(crate) fn eat(&mut self, tok: &Token) -> bool {
        if self.at(tok) { self.advance(); true } else { false }
    }

    pub(crate) fn expect(&mut self, tok: &Token, what: &'static str) -> Option<SpannedToken> {
        if self.at(tok) { Some(self.advance()) } else { self.push_error(what); None }
    }

    pub(crate) fn expect_ident(&mut self, what: &'static str) -> Option<Spanned<String>> {
        match self.peek_tok().clone() {
            Token::Ident(name) => {
                let span = self.peek_span();
                self.advance();
                Some(Spanned::new(name, span))
            }
            _ => {
                self.push_error(what);
                None
            }
        }
    }

    pub(crate) fn push_error(&mut self, what: &'static str) {
        let span = self.peek_span();
        let found = token_name(self.peek_tok());
        if let Some(e) = self.errors.last() {
            if e.span.lo == span.lo { return; }
        }
        self.errors.push(ParseErrorWithSpan {
            error: crate::error::ParseError::Expected(what, found),
            span,
        });
    }

    pub(crate) fn parse_path_segments(&mut self) -> Vec<Spanned<String>> {
        let mut segs = Vec::new();
        let first = self.expect_ident("path segment");
        if let Some(first) = first {
            segs.push(first);
        }
        loop {
            if self.eat(&Token::ColonColon) {
                if let Some(seg) = self.expect_ident("path segment") {
                    segs.push(seg);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        segs
    }

    pub(crate) fn single_path(&self, name: &str) -> Path {
        Path { segments: vec![Spanned::new(name.to_string(), self.peek_span())] }
    }

    // ─────────────────────────────────────────────────────────
    //  Type detection helpers
    // ─────────────────────────────────────────────────────────

    fn is_type_start(&self) -> bool {
        matches!(self.peek_tok(),
            Token::TyI8 | Token::TyI16 | Token::TyI32 | Token::TyI64 | Token::TyI128
            | Token::TyU8 | Token::TyU16 | Token::TyU32 | Token::TyU64 | Token::TyU128
            | Token::TyF16 | Token::TyF32 | Token::TyF64
            | Token::TyBool | Token::TyChar | Token::TyString | Token::TyVoid | Token::TyNever
            | Token::SelfType | Token::Arena
        ) || matches!(self.peek_tok(), Token::Ident(s) if s.chars().next().map_or(false, |c| c.is_uppercase()))
    }

    fn looks_like_local_decl(&mut self) -> bool {
        match self.peek_tok() {
            Token::Let | Token::Var | Token::Const => true,
            _ if self.is_type_start() => {
                // Try parsing as a type and check if next is an identifier.
                // This handles `int x`, `Arena<Player> p`, `String[] args`, etc.
                let save = self.pos;
                let is_decl = if parse_type(self).is_some() {
                    matches!(self.peek_tok(), Token::Ident(_))
                } else {
                    false
                };
                self.set_pos(save);
                is_decl
            }
            _ => false,
        }
    }

    // ─────────────────────────────────────────────────────────
    //  Program entry
    // ─────────────────────────────────────────────────────────

    pub fn parse_program(&mut self) -> Program {
        let mut items = Vec::new();
        while !self.at(&Token::Eof) {
            if let Some(item) = self.parse_item() {
                items.push(item);
            } else {
                self.advance();
            }
        }
        Program { items }
    }

    fn parse_item(&mut self) -> Option<Spanned<Item>> {
        let mut visibility = Visibility::Private;
        if self.at(&Token::Pub) {
            self.advance();
            visibility = Visibility::Public;
        } else if self.at(&Token::Priv) {
            self.advance();
        }

        let item = match self.peek_tok() {
            Token::Struct => self.parse_struct_item(visibility)?,
            Token::Enum => self.parse_enum_item(visibility)?,
            Token::Impl => self.parse_impl_item()?,
            Token::Trait => self.parse_trait_item(visibility)?,
            Token::Import | Token::From => self.parse_import_item()?,
            Token::Const => self.parse_const_item(visibility)?,
            Token::Static => {
                // Check if this is `static RetType name(...)` (static function)
                // by looking ahead for: static <type> <ident> (
                let save = self.pos;
                self.advance(); // consume `static`
                let is_func = self.is_type_start()
                    && matches!(self.peek_n(1).node, Token::Ident(_))
                    && matches!(self.peek_n(2).node, Token::LParen);
                self.set_pos(save);
                if is_func {
                    self.advance(); // consume `static`
                    self.parse_java_fn_or_item(visibility, true)?
                } else {
                    self.parse_static_item(visibility)?
                }
            }
            Token::Type => self.parse_type_alias_item(visibility)?,

            // fn keyword: old-style `fn name(...): RetType { }` or `fn name(...) -> RetType { }`
            Token::Fn => self.parse_fn_item(visibility, false)?,

            // Java-style: `[pub] [static] RetType name(...) { body }`
            _ if self.is_type_start() => {
                let is_static = self.eat(&Token::Static);
                self.parse_java_fn_or_item(visibility, is_static)?
            }

            _ => {
                self.push_error("item (fn, struct, enum, impl, trait, import, const, static, type)");
                self.advance();
                return None;
            }
        };

        while self.at(&Token::Semicolon) { self.advance(); }
        Some(item)
    }

    // ─────────────────────────────────────────────────────────
    //  Functions
    // ─────────────────────────────────────────────────────────

    fn parse_fn_item(&mut self, visibility: Visibility, is_static: bool) -> Option<Spanned<Item>> {
        self.advance(); // consume `fn`
        let name = self.expect_ident("function name")?;

        let mut is_thread = false;
        if self.at(&Token::Thread) { self.advance(); is_thread = true; }

        let generics = self.parse_generic_params();
        let params = self.parse_params();
        let return_type = self.parse_return_type();

        let body = if self.at(&Token::LBrace) {
            Some(self.parse_block().node)
        } else if self.at(&Token::Semicolon) {
            None
        } else {
            self.push_error("function body `{` or `;`");
            None
        };

        let span = name.span;
        Some(Spanned::new(
            Item::Function(FunctionDef {
                visibility, name, generics, params, return_type, body,
                is_extern: false, is_thread, is_static,
            }),
            span,
        ))
    }

    /// Java-style: `[pub] [static] RetType name(params) { body }`
    fn parse_java_fn_or_item(&mut self, visibility: Visibility, is_static: bool) -> Option<Spanned<Item>> {
        let ret_type = parse_type(self)?;
        let name = self.expect_ident("name")?;

        if self.at(&Token::LParen) {
            // It's a function/method
            let generics = Vec::new();
            let params = self.parse_params();
            let body = if self.at(&Token::LBrace) {
                Some(self.parse_block().node)
            } else if self.at(&Token::Semicolon) {
                None
            } else {
                self.push_error("function body `{` or `;`");
                None
            };
            let span = name.span;
            Some(Spanned::new(
                Item::Function(FunctionDef {
                    visibility, name, generics, params,
                    return_type: Some(ret_type), body,
                    is_extern: false, is_thread: false, is_static,
                }),
                span,
            ))
        } else {
            // It's a top-level const/field assignment: `Type name = expr;` — treat as const
            self.push_error("function declaration");
            self.advance();
            None
        }
    }

    fn parse_generic_params(&mut self) -> Vec<GenericParam> {
        let mut params = Vec::new();
        let saved = self.pos;
        if !self.at(&Token::Lt) { return params; }
        let is_generic = matches!(self.peek_n(1).node,
            Token::Ident(_) | Token::TyI8 | Token::TyI16 | Token::TyI32 | Token::TyI64
            | Token::TyU8 | Token::TyU16 | Token::TyU32 | Token::TyU64
            | Token::TyF32 | Token::TyF64)
            && matches!(self.peek_n(2).node, Token::Comma | Token::Gt);
        if !is_generic { return params; }
        self.advance();
        loop {
            let Some(name) = self.expect_ident("generic parameter name") else { break };
            let mut bounds = Vec::new();
            if self.eat(&Token::Colon) {
                loop {
                    if let Some(ty) = parse_type(self) { bounds.push(ty); }
                    if !self.eat(&Token::Plus) { break; }
                }
            }
            params.push(GenericParam { name, bounds });
            if !self.eat(&Token::Comma) { break; }
        }
        if !self.expect(&Token::Gt, "`>` to close generic parameter list").is_some() {
            self.pos = saved;
            return params;
        }
        params
    }

    fn parse_params(&mut self) -> Vec<Spanned<Param>> {
        let mut params = Vec::new();
        if !self.expect(&Token::LParen, "`(` for parameter list").is_some() {
            return params;
        }
        if self.eat(&Token::RParen) { return params; }
        loop {
            if let Some(param) = self.parse_param() {
                params.push(param);
            }
            if !self.eat(&Token::Comma) { break; }
        }
        self.expect(&Token::RParen, "`)` to close parameter list");
        params
    }

    fn parse_param(&mut self) -> Option<Spanned<Param>> {
        let start_span = self.peek_span();

        // `self`, `&self`, `&mut self`
        if self.at(&Token::SelfType) {
            let sp = self.peek_span();
            self.advance();
            return Some(Spanned::new(
                Param {
                    name: Spanned::new("self".to_string(), sp),
                    ty: Spanned::new(TypeAnnotation::Named(Path {
                        segments: vec![Spanned::new("Self".to_string(), sp)],
                    }), sp),
                    default: None,
                },
                span_for(start_span, sp),
            ));
        }

        // Java-style: `Type name`
        if self.is_type_start() {
            let Some(ty) = parse_type(self) else { return None };
            let Some(name) = self.expect_ident("parameter name") else { return None };
            let mut default = None;
            if self.eat(&Token::Eq) {
                default = Some(crate::expr::parse_expression(self));
            }
            let span = span_for(start_span, self.previous_span());
            return Some(Spanned::new(Param { name, ty, default }, span));
        }

        // Fallback: `name: type` (old Rust-style)
        let Some(name) = self.expect_ident("parameter name") else { return None };
        if self.eat(&Token::Colon) {
            let ty = parse_type(self);
            if let Some(ty) = ty {
                let mut default = None;
                if self.eat(&Token::Eq) {
                    default = Some(crate::expr::parse_expression(self));
                }
                let span = span_for(start_span, self.previous_span());
                return Some(Spanned::new(Param { name, ty, default }, span));
            }
        }
        self.push_error("parameter type annotation");
        None
    }

    fn parse_return_type(&mut self) -> Option<Spanned<TypeAnnotation>> {
        // Rust-style `-> Type`
        if self.eat(&Token::Arrow) {
            let ty = parse_type(self);
            if ty.is_none() {
                let sp = self.peek_span();
                return Some(Spanned::new(TypeAnnotation::Void, sp));
            }
            return ty;
        }
        // Java-style `: Type` (but we don't use colon for return; used for type alias etc.)
        None
    }

    // ─────────────────────────────────────────────────────────
    //  Blocks & Statements
    // ─────────────────────────────────────────────────────────

    pub(crate) fn parse_block(&mut self) -> Spanned<Block> {
        let open_span = self.peek_span();
        self.advance(); // consume `{`
        let mut stmts = Vec::new();
        let result = None;

        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            if self.at(&Token::Semicolon) { self.advance(); continue; }
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.advance();
            }
        }

        let close_span = if self.at(&Token::RBrace) {
            let sp = self.peek_span(); self.advance(); sp
        } else {
            self.push_error("`}` to close block"); open_span
        };

        let span = span_for(open_span, close_span);
        Spanned::new(Block { stmts, result, span }, span)
    }

    /// `switch (scrutinee) { case label: stmts ... default: stmts }`
    fn parse_switch(&mut self) -> Spanned<york_ast::Stmt> {
        use york_ast::Stmt;
        let sp = self.peek_span();
        self.advance(); // consume `switch`
        self.expect(&Token::LParen, "`(` after switch");
        let scrutinee = crate::expr::parse_expression(self);
        self.expect(&Token::RParen, "`)` after switch scrutinee");
        self.expect(&Token::LBrace, "`{` to open switch body");

        let mut arms: Vec<york_ast::SwitchArm> = Vec::new();
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            let is_case = matches!(self.peek_tok(), Token::Ident(s) if s == "case");
            let is_default = matches!(self.peek_tok(), Token::Ident(s) if s == "default");
            if is_case {
                self.advance(); // consume `case`
                let label = crate::expr::parse_expression(self);
                self.expect(&Token::Colon, "`:` after case label");
                let body = self.parse_switch_body();
                arms.push(york_ast::SwitchArm {
                    label: Some(label),
                    body,
                });
            } else if is_default {
                self.advance(); // consume `default`
                self.expect(&Token::Colon, "`:` after default");
                let body = self.parse_switch_body();
                arms.push(york_ast::SwitchArm {
                    label: None,
                    body,
                });
            } else {
                self.push_error("expected `case` or `default` in switch body");
                self.advance();
            }
        }

        let close_span = if self.at(&Token::RBrace) {
            let s = self.peek_span(); self.advance(); s
        } else {
            let s = self.peek_span(); self.push_error("`}` to close switch"); s
        };
        let span = span_for(sp, close_span);
        Spanned::new(Stmt::Switch { scrutinee, arms }, span)
    }

    /// Parse statements belonging to one switch arm until the next `case`/`default`/`}`/EOF.
    fn parse_switch_body(&mut self) -> Vec<Spanned<york_ast::Stmt>> {
        let mut body = Vec::new();
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            if matches!(self.peek_tok(), Token::Ident(s) if s == "case" || s == "default") {
                break;
            }
            if self.at(&Token::Semicolon) { self.advance(); continue; }
            if let Some(stmt) = self.parse_stmt() {
                body.push(stmt);
            } else {
                self.advance();
            }
        }
        body
    }

    pub(crate) fn parse_stmt(&mut self) -> Option<Spanned<york_ast::Stmt>> {
        use york_ast::Stmt;

        match self.peek_tok().clone() {
            Token::Let => self.parse_local_decl(true),
            Token::Var => self.parse_local_decl(true),
            Token::Return => {
                let sp = self.peek_span();
                self.advance();
                let value = if self.at(&Token::Semicolon) || self.at(&Token::RBrace) || self.at(&Token::Eof) {
                    None
                } else {
                    Some(crate::expr::parse_expression(self))
                };
                if self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(Stmt::Return(value), sp))
            }
            Token::Defer => {
                let sp = self.peek_span();
                self.advance();
                let expr = crate::expr::parse_expression(self);
                if self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(Stmt::Defer(expr), sp))
            }
            Token::Break => {
                let sp = self.peek_span();
                self.advance();
                let label = if let Token::Ident(l) = self.peek_tok().clone() {
                    let sp2 = self.peek_span(); self.advance(); Some(Spanned::new(l, sp2))
                } else { None };
                if self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(Stmt::Break(label), sp))
            }
            Token::Continue => {
                let sp = self.peek_span();
                self.advance();
                let label = if let Token::Ident(l) = self.peek_tok().clone() {
                    let sp2 = self.peek_span(); self.advance(); Some(Spanned::new(l, sp2))
                } else { None };
                if self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(Stmt::Continue(label), sp))
            }
            Token::If => {
                let if_expr = crate::expr::parse_if_expr(self);
                let sp = if_expr.span;
                match if_expr.node {
                    york_ast::Expr::If(inner) => Some(Spanned::new(Stmt::If(*inner), sp)),
                    _ => None,
                }
            }
            Token::While => {
                let sp = self.peek_span();
                self.advance();
                if self.eat(&Token::LParen) {
                    let condition = crate::expr::parse_expression(self);
                    self.expect(&Token::RParen, "`)` after while condition");
                    let body = self.parse_block().node;
                    Some(Spanned::new(Stmt::While { condition, body }, sp))
                } else {
                    let condition = crate::expr::parse_expression(self);
                    let body = self.parse_block().node;
                    Some(Spanned::new(Stmt::While { condition, body }, sp))
                }
            }
            Token::For => {
                let sp = self.peek_span();
                self.advance();
                // Check if this is C-style for `(init; cond; update)` or enhanced `(Type name : iter)`
                if self.at(&Token::LParen) {
                    let Some(for_rest) = crate::expr::parse_for_rest(self) else { return None };
                    let body = self.parse_block().node;
                    match for_rest {
                        crate::expr::ForRest::Enhanced { variable, iterable } => {
                            Some(Spanned::new(Stmt::For { variable, iterable, body }, sp))
                        }
                        crate::expr::ForRest::Classic { init, condition, update } => {
                            Some(Spanned::new(Stmt::ForC { init, condition, update, body }, sp))
                        }
                    }
                } else {
                    // Rust-style: `for name in iterable { body }`
                    let Some(variable) = self.expect_ident("loop variable") else { return None };
                    self.expect(&Token::In, "`in` in for loop");
                    let iterable = crate::expr::parse_expression(self);
                    let body = self.parse_block().node;
                    Some(Spanned::new(Stmt::For { variable, iterable, body }, sp))
                }
            }
            Token::Loop => {
                let sp = self.peek_span();
                self.advance();
                let body = self.parse_block().node;
                Some(Spanned::new(Stmt::Loop { body }, sp))
            }
            Token::Switch => Some(self.parse_switch()),
            Token::LBrace => {
                let block = self.parse_block();
                let stmt_span = block.span;
                Some(Spanned::new(Stmt::Block(block.node), stmt_span))
            }
            Token::Semicolon => { self.advance(); None }
            Token::Eof | Token::RBrace => None,
            _ => {
                // Try typed declaration first: `Type name [= expr];`
                if self.looks_like_local_decl() {
                    return self.parse_local_decl_stmt();
                }
                // Expression statement
                let expr = crate::expr::parse_expression(self);
                let span = expr.span;
                if self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(Stmt::Expr(expr), span))
            }
        }
    }

    // ─────────────────────────────────────────────────────────
    //  Local declarations
    // ─────────────────────────────────────────────────────────

    fn parse_local_decl_stmt(&mut self) -> Option<Spanned<york_ast::Stmt>> {
        self.parse_local_decl(true)
    }

    /// Parse a local declaration: `Type name [= expr];` or `let/var/const ...`
    /// If `consume_semi` is true, consume trailing semicolon.
    pub(crate) fn parse_local_decl(&mut self, consume_semi: bool) -> Option<Spanned<york_ast::Stmt>> {
        let sp = self.peek_span();

        match self.peek_tok().clone() {
            Token::Let => {
                self.advance();
                let Some(name) = self.expect_ident("variable name") else { return None };
                let mut ty = None;
                if self.eat(&Token::Colon) { ty = parse_type(self); }
                let mut value = None;
                if self.eat(&Token::Eq) { value = Some(crate::expr::parse_expression(self)); }
                if consume_semi && self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(york_ast::Stmt::Let { name, ty, value }, sp))
            }
            Token::Var => {
                self.advance();
                let Some(name) = self.expect_ident("variable name") else { return None };
                let mut ty = None;
                if self.eat(&Token::Colon) { ty = parse_type(self); }
                self.expect(&Token::Eq, "`=` in variable declaration");
                let value = crate::expr::parse_expression(self);
                if consume_semi && self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(york_ast::Stmt::Var { name, ty, value }, sp))
            }
            Token::Const => {
                self.advance();
                let Some(name) = self.expect_ident("variable name") else { return None };
                let mut ty = None;
                if self.eat(&Token::Colon) { ty = parse_type(self); }
                self.expect(&Token::Eq, "`=` in const declaration");
                let value = crate::expr::parse_expression(self);
                if consume_semi && self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(york_ast::Stmt::Let { name, ty, value: Some(value) }, sp))
            }
            _ if self.is_type_start() => {
                // Java-style: `Type name [= expr];`
                let Some(ty) = parse_type(self) else { return None };
                let Some(name) = self.expect_ident("variable name") else { return None };
                let value = if self.eat(&Token::Eq) {
                    Some(crate::expr::parse_expression(self))
                } else {
                    None
                };
                if consume_semi && self.at(&Token::Semicolon) { self.advance(); }
                Some(Spanned::new(york_ast::Stmt::Let { name, ty: Some(ty), value }, sp))
            }
            _ => None,
        }
    }

    // ─────────────────────────────────────────────────────────
    //  Structs, Enums, Impls, Traits
    // ─────────────────────────────────────────────────────────

    fn parse_struct_item(&mut self, visibility: Visibility) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance(); // consume `struct`

        let mut is_packed = false;
        if self.eat(&Token::Packed) { is_packed = true; }

        let Some(name) = self.expect_ident("struct name") else { return None };
        let generics = self.parse_generic_params();

        let mut fields = Vec::new();
        if self.expect(&Token::LBrace, "`{` for struct body").is_some() {
            while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
                let mut field_vis = Visibility::Private;
                if self.eat(&Token::Pub) { field_vis = Visibility::Public; }
                else if self.eat(&Token::Priv) { field_vis = Visibility::Private; }

                // Java-style: `Type name [= default];`
                if self.is_type_start() {
                    let Some(field_ty) = parse_type(self) else { self.advance(); continue; };
                    let Some(field_name) = self.expect_ident("field name") else { self.advance(); continue; };
                    // Optional default value
                    let _default = if self.eat(&Token::Eq) {
                        Some(crate::expr::parse_expression(self))
                    } else { None };
                    if self.at(&Token::Comma) || self.at(&Token::Semicolon) { self.advance(); }
                    fields.push(Spanned::new(
                        york_ast::FieldDef { visibility: field_vis, name: field_name, ty: field_ty },
                        sp,
                    ));
                }
                // Old Rust-style: `name: Type`
                else if let Token::Ident(_) = self.peek_tok().clone() {
                    let Some(field_name) = self.expect_ident("field name") else { self.advance(); continue; };
                    if self.eat(&Token::Colon) {
                        let Some(field_ty) = parse_type(self) else { self.advance(); continue; };
                        if self.at(&Token::Comma) || self.at(&Token::Semicolon) { self.advance(); }
                        fields.push(Spanned::new(
                            york_ast::FieldDef { visibility: field_vis, name: field_name, ty: field_ty },
                            sp,
                        ));
                    } else {
                        self.push_error("`:` after field name");
                    }
                }
                // `static` field
                else if self.eat(&Token::Static) {
                    if let Some(field_ty) = parse_type(self) {
                        if let Some(field_name) = self.expect_ident("field name") {
                            let _default = if self.eat(&Token::Eq) {
                                Some(crate::expr::parse_expression(self))
                            } else { None };
                            if self.at(&Token::Comma) || self.at(&Token::Semicolon) { self.advance(); }
                            fields.push(Spanned::new(
                                york_ast::FieldDef { visibility: field_vis, name: field_name, ty: field_ty },
                                sp,
                            ));
                        }
                    }
                }
                else {
                    self.push_error("field definition");
                    self.advance();
                }
            }
            self.expect(&Token::RBrace, "`}` to close struct body");
        }

        Some(Spanned::new(
            Item::Struct(StructDef { visibility, name, generics, fields, is_packed }),
            sp,
        ))
    }

    fn parse_enum_item(&mut self, visibility: Visibility) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance();
        let Some(name) = self.expect_ident("enum name") else { return None };
        let generics = self.parse_generic_params();

        let mut variants = Vec::new();
        if self.expect(&Token::LBrace, "`{` for enum body").is_some() {
            while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
                let vsp = self.peek_span();
                let Some(variant_name) = self.expect_ident("variant name") else { self.advance(); continue; };
                let fields = if self.eat(&Token::LParen) {
                    let mut f = Vec::new();
                    if !self.at(&Token::RParen) {
                        loop {
                            if let Some(ty) = parse_type(self) { f.push(ty); }
                            if !self.eat(&Token::Comma) { break; }
                        }
                    }
                    self.expect(&Token::RParen, "`)` for variant fields");
                    f
                } else { Vec::new() };
                if self.at(&Token::Comma) || self.at(&Token::Semicolon) { self.advance(); }
                variants.push(Spanned::new(york_ast::VariantDef { name: variant_name, fields }, vsp));
            }
            self.expect(&Token::RBrace, "`}` to close enum body");
        }

        Some(Spanned::new(
            Item::Enum(EnumDef { visibility, name, generics, variants }),
            sp,
        ))
    }

    fn parse_impl_item(&mut self) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance(); // consume `impl`
        let generics = self.parse_generic_params();
        let self_type = parse_type(self)?;
        let mut methods = Vec::new();

        if self.expect(&Token::LBrace, "`{` for impl body").is_some() {
            while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
                let mut method_vis = Visibility::Private;
                if self.eat(&Token::Pub) { method_vis = Visibility::Public; }
                let mut is_static = false;
                if self.at(&Token::Static) {
                    self.advance();
                    is_static = true;
                }

                if self.at(&Token::Fn) {
                    // Old-style fn method
                    let method = self.parse_fn_item(method_vis, is_static);
                    if let Some(Spanned { node: Item::Function(f), span }) = method {
                        methods.push(Spanned::new(f, span));
                    }
                } else if self.is_type_start() {
                    // Java-style: `RetType name(params) { body }`
                    let Some(ret_type) = parse_type(self) else { self.advance(); continue; };
                    let Some(method_name) = self.expect_ident("method name") else { self.advance(); continue; };
                    let params = self.parse_params();
                    let body = if self.at(&Token::LBrace) {
                        Some(self.parse_block().node)
                    } else if self.at(&Token::Semicolon) {
                        self.advance(); None
                    } else {
                        self.push_error("method body `{` or `;`"); None
                    };
                    let mspan = method_name.span;
                    methods.push(Spanned::new(FunctionDef {
                        visibility: method_vis, name: method_name, generics: Vec::new(),
                        params, return_type: Some(ret_type), body,
                        is_extern: false, is_thread: false, is_static,
                    }, mspan));
                } else if self.at(&Token::RBrace) {
                    break;
                } else {
                    self.push_error("method definition");
                    self.advance();
                }
            }
            self.expect(&Token::RBrace, "`}` to close impl body");
        }

        Some(Spanned::new(
            Item::Impl(york_ast::ImplBlock { self_type, generics, methods }),
            sp,
        ))
    }

    fn parse_trait_item(&mut self, visibility: Visibility) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance();
        let Some(name) = self.expect_ident("trait name") else { return None };
        let generics = self.parse_generic_params();

        let mut methods = Vec::new();
        if self.expect(&Token::LBrace, "`{` for trait body").is_some() {
            while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
                let msp = self.peek_span();
                let mut mvis = Visibility::Private;
                if self.eat(&Token::Pub) { mvis = Visibility::Public; }
                let _ = mvis;

                // Java-style: `RetType name(params);`
                if self.is_type_start() {
                    let ret_type = parse_type(self);
                    let Some(method_name) = self.expect_ident("trait method name") else { self.advance(); continue; };
                    let params = self.parse_params();
                    let has_body = self.at(&Token::LBrace);
                    if has_body { self.parse_block(); }
                    if self.at(&Token::Semicolon) { self.advance(); }
                    methods.push(Spanned::new(
                        york_ast::TraitMethod { name: method_name, params, return_type: ret_type, has_body },
                        msp,
                    ));
                }
                // Old-style: `fn name(params): RetType`
                else if self.eat(&Token::Fn) {
                    let Some(method_name) = self.expect_ident("trait method name") else { self.advance(); continue; };
                    let params = self.parse_params();
                    let return_type = self.parse_return_type();
                    let has_body = self.at(&Token::LBrace);
                    if has_body { self.parse_block(); }
                    methods.push(Spanned::new(
                        york_ast::TraitMethod { name: method_name, params, return_type, has_body },
                        msp,
                    ));
                } else {
                    self.push_error("trait method definition");
                    self.advance();
                }
            }
            self.expect(&Token::RBrace, "`}` to close trait body");
        }

        Some(Spanned::new(
            Item::Trait(TraitDef { visibility, name, generics, methods }),
            sp,
        ))
    }

    // ─────────────────────────────────────────────────────────
    //  Imports & Constants
    // ─────────────────────────────────────────────────────────

    fn parse_import_item(&mut self) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance();

        // Source-file import: `import "util/math.yk";`
        if let Token::StringLiteral(path) = self.peek_tok().clone() {
            self.advance();
            self.expect(&Token::Semicolon, "`;` to close file import");
            return Some(Spanned::new(
                Item::Import(ImportDecl { path: Vec::new(), aliases: Vec::new(), file: Some(path) }),
                sp,
            ));
        }

        let mut path = Vec::new();
        let Some(first) = self.expect_ident("module path") else { return None };
        path.push(first);
        while self.eat(&Token::Dot) || self.eat(&Token::ColonColon) {
            let Some(seg) = self.expect_ident("module path segment") else { break };
            path.push(seg);
        }

        let mut aliases = Vec::new();
        if self.eat(&Token::LBrace) {
            while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
                let Some(name) = self.expect_ident("import name") else { self.advance(); continue; };
                let alias = if self.eat(&Token::As) { self.expect_ident("import alias") } else { None };
                aliases.push(york_ast::ImportAlias { name, alias });
                if !self.eat(&Token::Comma) { break; }
            }
            self.expect(&Token::RBrace, "`}` to close import list");
        } else if self.eat(&Token::As) {
            let name = Spanned::new(path.last().unwrap().node.clone(), sp);
            let alias = self.expect_ident("import alias");
            if let Some(alias) = alias { aliases.push(york_ast::ImportAlias { name, alias: Some(alias) }); }
        } else {
            let name = Spanned::new(path.last().unwrap().node.clone(), sp);
            aliases.push(york_ast::ImportAlias { name, alias: None });
        }

        if self.at(&Token::Semicolon) { self.advance(); }
        Some(Spanned::new(Item::Import(ImportDecl { path, aliases, file: None }), sp))
    }

    fn parse_const_item(&mut self, visibility: Visibility) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance();
        let Some(name) = self.expect_ident("constant name") else { return None };
        // Java-style: `Type name = expr;` — type before name. But here we already consumed `const`.
        // So it's `const name [: Type] = expr;`
        let ty = if self.eat(&Token::Colon) { parse_type(self) } else { None };
        self.expect(&Token::Eq, "`=` in const declaration");
        let value = crate::expr::parse_expression(self);
        if self.at(&Token::Semicolon) { self.advance(); }
        Some(Spanned::new(
            Item::Const(york_ast::ConstDecl { visibility, name, ty, value }),
            sp,
        ))
    }

    fn parse_static_item(&mut self, visibility: Visibility) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance();
        // Java-style: `static Type name = expr;` or Rust-style: `static name: Type = expr;`
        if self.is_type_start() {
            let Some(ty) = parse_type(self) else { return None };
            let Some(name) = self.expect_ident("static name") else { return None };
            let value = if self.eat(&Token::Eq) { Some(crate::expr::parse_expression(self)) } else { None };
            if self.at(&Token::Semicolon) { self.advance(); }
            return Some(Spanned::new(Item::Static(StaticDecl { visibility, name, ty, value }), sp));
        }
        let Some(name) = self.expect_ident("static name") else { return None };
        self.expect(&Token::Colon, "`:` in static declaration");
        let ty = parse_type(self)?;
        let value = if self.eat(&Token::Eq) { Some(crate::expr::parse_expression(self)) } else { None };
        if self.at(&Token::Semicolon) { self.advance(); }
        Some(Spanned::new(Item::Static(StaticDecl { visibility, name, ty, value }), sp))
    }

    fn parse_type_alias_item(&mut self, visibility: Visibility) -> Option<Spanned<Item>> {
        let sp = self.peek_span();
        self.advance();
        let Some(name) = self.expect_ident("type alias name") else { return None };
        let generics = self.parse_generic_params();
        self.expect(&Token::Eq, "`=` in type alias");
        let ty = parse_type(self)?;
        if self.at(&Token::Semicolon) { self.advance(); }
        Some(Spanned::new(Item::TypeAlias(TypeAliasDecl { visibility, name, generics, ty }), sp))
    }
}
