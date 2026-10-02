use york_ast::ast::{BinOp, Expr, MatchArm, Path, Pattern};
use york_ast::ast::UnOp;
use york_ast::span::{Spanned};
use york_ast::{CompoundOp, IfExpr, ElseBranch, Token};

use crate::Parser;

/// Java/JS-style expression parser.
/// Parse `match <scrutinee> { <pattern> [if <guard>] => <expr>, ... }`.
///
/// Also accepts `pattern: expr` as a synonym for `pattern => expr`.
fn parse_match_expr(p: &mut Parser) -> Spanned<Expr> {
    let sp = p.peek_span();
    p.advance(); // `match`

    // The scrutinee's `{` opens the arm list, never a struct literal.
    p.no_struct_lit = true;
    let scrutinee = parse_expr_bp(p, 4);
    p.no_struct_lit = false;
    p.expect(&Token::LBrace, "`{` to open match arms");

    let mut arms = Vec::new();
    while !p.at(&Token::RBrace) && !p.at(&Token::Eof) {
        let Some(pattern) = parse_pattern(p) else {
            p.advance();
            continue;
        };
        let guard = if p.eat(&Token::If) {
            Some(parse_expr_bp(p, 4))
        } else {
            None
        };
        if !p.eat(&Token::FatArrow) && !p.eat(&Token::Colon) {
            p.push_error("`=>` after match pattern");
            p.advance();
            continue;
        }
        let body = parse_expr_bp(p, 4);
        arms.push(MatchArm { pattern, guard, body });
        if !p.eat(&Token::Comma) {
            // Commas are optional before `}` and between block-bodied arms.
            if !p.at(&Token::RBrace) {
                p.push_error("`,` between match arms");
                p.advance();
            }
        }
    }
    p.expect(&Token::RBrace, "`}` to close match expression");
    let end = p.previous_span();
    Spanned::new(
        Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        },
        sp.to(end),
    )
}

/// Parse a single match pattern: literal, `_`, identifier binding, tuple,
/// enum variant, or struct pattern.
fn parse_pattern(p: &mut Parser) -> Option<Spanned<Pattern>> {
    let sp = p.peek_span();

    // Negative numeric literals.
    let literal = match p.peek_tok().clone() {
        Token::Minus => {
            p.advance();
            let operand = parse_expr_bp(p, 14);
            Some(Spanned::new(
                Expr::Unary {
                    op: Spanned::new(UnOp::Neg, sp),
                    operand: Box::new(operand),
                },
                sp,
            ))
        }
        Token::IntLiteral(_) | Token::FloatLiteral(_) | Token::StringLiteral(_)
        | Token::CharLiteral(_) | Token::BoolLiteral(_) => Some(parse_expr_bp(p, 14)),
        _ => None,
    };
    if let Some(lit) = literal {
        return Some(Spanned::new(Pattern::Literal(lit), sp));
    }

    if matches!(p.peek_tok(), Token::Ident(s) if s == "_") {
        p.advance();
        return Some(Spanned::new(Pattern::Wildcard, sp));
    }

    // Tuple pattern: `(a, b)`
    if p.at(&Token::LParen) {
        let save = p.pos();
        p.advance();
        if !p.at(&Token::RParen) {
            let mut items = Vec::new();
            while !p.at(&Token::RParen) && !p.at(&Token::Eof) {
                let Some(inner) = parse_pattern(p) else { break };
                items.push(inner);
                if !p.eat(&Token::Comma) { break; }
            }
            if p.eat(&Token::RParen) && items.len() > 1 {
                return Some(Spanned::new(Pattern::Tuple(items), sp));
            }
        }
        p.set_pos(save);
    }

    let segs = p.parse_path_segments();
    if segs.is_empty() {
        return None;
    }
    // Enum variant patterns use dotted access: `Color.Red`.
    let mut segs = segs;
    while p.at(&Token::Dot) {
        let dot = p.peek_span();
        p.advance();
        let Some(name) = p.expect_ident("variant name after `.`") else { break };
        segs.push(Spanned::new(name.node, dot));
    }

    // Struct pattern: `Point { x, y }` (or `Point { x: px, y: py }`)
    if p.at(&Token::LBrace) {
        p.advance();
        let mut fields = Vec::new();
        while !p.at(&Token::RBrace) && !p.at(&Token::Eof) {
            let Some(name) = p.expect_ident("struct pattern field") else { p.advance(); continue; };
            let value = if p.eat(&Token::Colon) {
                match parse_pattern(p) {
                    Some(inner) => inner,
                    None => Spanned::new(Pattern::Wildcard, name.span),
                }
            } else {
                Spanned::new(Pattern::Ident(name.clone()), name.span)
            };
            fields.push((name, value));
            if !p.eat(&Token::Comma) { break; }
        }
        p.expect(&Token::RBrace, "`}` to close struct pattern");
        return Some(Spanned::new(
            Pattern::Struct {
                path: Path { segments: segs },
                fields,
            },
            sp,
        ));
    }

    let path = Path { segments: segs };
    // Enum variant with positional payload: `Variant(a, b)`
    if p.at(&Token::LParen) {
        p.advance();
        let mut fields = Vec::new();
        while !p.at(&Token::RParen) && !p.at(&Token::Eof) {
            let Some(inner) = parse_pattern(p) else { break };
            fields.push(inner);
            if !p.eat(&Token::Comma) { break; }
        }
        p.expect(&Token::RParen, "`)` to close variant pattern");
        return Some(Spanned::new(Pattern::Variant { path, fields }, sp));
    }

    // A single-segment path with no payload is a binding, unless it names an
    // enum variant — the caller resolves that against the enum tables.
    let last = path.segments.last().map(|s| s.node.clone()).unwrap_or_default();
    let name = path.segments.last().cloned()?;
    Some(Spanned::new(
        if path.segments.len() == 1 && !is_uppercase_start(&last) {
            Pattern::Ident(name)
        } else {
            Pattern::Variant { path, fields: Vec::new() }
        },
        sp,
    ))
}

fn is_uppercase_start(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

pub(crate) fn parse_expression(p: &mut Parser) -> Spanned<Expr> {
    parse_expr_bp(p, 0)
}

/// Parse an `if` expression or statement.
pub fn parse_if_expr(p: &mut Parser) -> Spanned<Expr> {
    let sp = p.peek_span();
    p.advance(); // consume `if`

    let condition = if p.eat(&Token::LParen) {
        let c = parse_expr_bp(p, 0);
        p.expect(&Token::RParen, "`)` after if condition");
        c
    } else {
        parse_expr_bp(p, 0)
    };

    let then_branch = p.parse_block().node;

    let else_branch = if p.eat(&Token::Else) {
        if p.at(&Token::If) {
            let else_if = parse_if_expr(p);
            if let Expr::If(inner) = else_if.node {
                Some(ElseBranch::If(inner))
            } else {
                None
            }
        } else {
            Some(ElseBranch::Block(p.parse_block().node))
        }
    } else {
        None
    };

    let if_expr = IfExpr {
        condition,
        then_branch,
        else_branch,
    };
    Spanned::new(Expr::If(Box::new(if_expr)), sp)
}

fn bin_bp(tok: &Token) -> Option<(u8, u8)> {
    let (bp, right) = match tok {
        Token::Or => (4, false),
        Token::And => (5, false),
        Token::Pipe => (6, false),
        Token::Caret => (7, false),
        Token::Amp => (8, false),
        Token::EqEq | Token::BangEq => (9, false),
        Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => (10, false),
        Token::LtLt | Token::GtGt => (11, false),
        Token::Plus | Token::Minus => (12, false),
        Token::Star | Token::Slash | Token::Percent => (13, false),
        _ => return None,
    };
    Some((bp, if right { bp } else { bp + 1 }))
}

fn assign_bp(tok: &Token) -> Option<(u8, u8)> {
    match tok {
        Token::Eq
        | Token::PlusEq | Token::MinusEq | Token::StarEq | Token::SlashEq | Token::PercentEq
        | Token::AmpEq | Token::PipeEq | Token::CaretEq => Some((2, 2)),
        _ => None,
    }
}

fn parse_expr_bp(p: &mut Parser, min_bp: u8) -> Spanned<Expr> {
    let mut lhs = parse_prefix(p);

    loop {
        // Ternary
        if p.at(&Token::Question) && 3 >= min_bp {
            p.advance();
            let then_branch = parse_expr_bp(p, 0);
            p.expect(&Token::Colon, "`:` in ternary expression");
            let else_branch = parse_expr_bp(p, 3);
            let span = lhs.span.to(else_branch.span);
            lhs = Spanned::new(
                Expr::Ternary {
                    condition: Box::new(lhs),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                },
                span,
            );
            continue;
        }

        // Assignment
        if let Some((bp, _)) = assign_bp(p.peek_tok()) {
            if bp < min_bp {
                break;
            }
            let op = p.peek_tok().clone();
            let op_span = p.peek_span();
            p.advance();
            let rhs = parse_expr_bp(p, bp);
            let span = lhs.span.to(rhs.span);
            if matches!(op, Token::Eq) {
                lhs = Spanned::new(
                    Expr::Assign {
                        target: Box::new(lhs),
                        value: Box::new(rhs),
                    },
                    span,
                );
            } else {
                let c = match op {
                    Token::PlusEq => CompoundOp::Add,
                    Token::MinusEq => CompoundOp::Sub,
                    Token::StarEq => CompoundOp::Mul,
                    Token::SlashEq => CompoundOp::Div,
                    Token::PercentEq => CompoundOp::Mod,
                    Token::AmpEq => CompoundOp::BitAnd,
                    Token::PipeEq => CompoundOp::BitOr,
                    Token::CaretEq => CompoundOp::BitXor,
                    _ => break,
                };
                lhs = Spanned::new(
                    Expr::CompoundAssign {
                        op: Spanned::new(c, op_span),
                        target: Box::new(lhs),
                        value: Box::new(rhs),
                    },
                    span,
                );
            }
            continue;
        }

        // Binary operators
        if let Some((bp, rhs_bp)) = bin_bp(p.peek_tok()) {
            if bp < min_bp {
                break;
            }
            let op = p.peek_tok().clone();
            let op_span = p.peek_span();
            p.advance();
            let rhs = parse_expr_bp(p, rhs_bp);
            let binop = BinOp::from_str(&op.to_string()).unwrap();
            let span = lhs.span.to(rhs.span);
            lhs = Spanned::new(
                Expr::Binary {
                    op: Spanned::new(binop, op_span),
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                span,
            );
            continue;
        }

        break;
    }

    lhs
}

fn parse_prefix(p: &mut Parser) -> Spanned<Expr> {
    // Prefix increment/decrement
    if p.at(&Token::Plus) && matches!(p.peek_n(1).node, Token::Plus) {
        p.advance();
        p.advance();
        let operand = parse_prefix(p);
        let span = operand.span;
        return Spanned::new(Expr::Incr { operand: Box::new(operand), positive: true, is_postfix: false }, span);
    }
    if p.at(&Token::Minus) && matches!(p.peek_n(1).node, Token::Minus) {
        p.advance();
        p.advance();
        let operand = parse_prefix(p);
        let span = operand.span;
        return Spanned::new(Expr::Incr { operand: Box::new(operand), positive: false, is_postfix: false }, span);
    }

    let expr = match p.peek_tok().clone() {
        Token::IntLiteral(v) => {
            let sp = p.peek_span();
            p.advance();
            Spanned::new(Expr::Int(v), sp)
        }
        Token::FloatLiteral(v) => {
            let sp = p.peek_span();
            p.advance();
            Spanned::new(Expr::Float(v), sp)
        }
        Token::StringLiteral(v) => {
            let sp = p.peek_span();
            p.advance();
            Spanned::new(Expr::String(v), sp)
        }
        Token::CharLiteral(v) => {
            let sp = p.peek_span();
            p.advance();
            Spanned::new(Expr::Char(v), sp)
        }
        Token::BoolLiteral(v) => {
            let sp = p.peek_span();
            p.advance();
            Spanned::new(Expr::Bool(v), sp)
        }
        Token::NullLiteral => {
            let sp = p.peek_span();
            p.advance();
            Spanned::new(Expr::Null, sp)
        }
        Token::This => {
            let sp = p.peek_span();
            p.advance();
            Spanned::new(Expr::This, sp)
        }
        Token::Minus => {
            let sp = p.peek_span();
            p.advance();
            let operand = parse_expr_bp(p, 14);
            let e = Spanned::new(Expr::Unary { op: Spanned::new(UnOp::Neg, sp), operand: Box::new(operand) }, sp);
            e
        }
        Token::Bang => {
            let sp = p.peek_span();
            p.advance();
            let operand = parse_expr_bp(p, 14);
            Spanned::new(Expr::Unary { op: Spanned::new(UnOp::Not, sp), operand: Box::new(operand) }, sp)
        }
        Token::Tilde => {
            let sp = p.peek_span();
            p.advance();
            let operand = parse_expr_bp(p, 14);
            Spanned::new(Expr::Unary { op: Spanned::new(UnOp::BitNot, sp), operand: Box::new(operand) }, sp)
        }
        Token::Amp => {
            let sp = p.peek_span();
            p.advance();
            let operand = parse_expr_bp(p, 14);
            Spanned::new(Expr::Unary { op: Spanned::new(UnOp::Ref, sp), operand: Box::new(operand) }, sp)
        }
        Token::Star => {
            let sp = p.peek_span();
            p.advance();
            let operand = parse_expr_bp(p, 14);
            Spanned::new(Expr::Unary { op: Spanned::new(UnOp::Deref, sp), operand: Box::new(operand) }, sp)
        }
        Token::New => {
            let sp = p.peek_span();
            p.advance();
            let Some(ty) = crate::types::parse_type(p) else {
                p.push_error("type after `new`");
                return Spanned::new(Expr::Null, sp);
            };
            let mut args = Vec::new();
            if p.eat(&Token::LParen) {
                if !p.at(&Token::RParen) {
                    loop {
                        args.push(parse_expression(p));
                        if !p.eat(&Token::Comma) { break; }
                    }
                }
                p.expect(&Token::RParen, "`)` to close constructor call");
            }
            let end = p.previous_span();
            Spanned::new(Expr::New { ty, args }, sp.to(end))
        }
        Token::Sizeof | Token::Alignof => {
            let sp = p.peek_span();
            let is_sizeof = p.at(&Token::Sizeof);
            p.advance();
            p.expect(&Token::LParen, "`(` after `sizeof`/`alignof`");
            let Some(ty) = crate::types::parse_type(p) else {
                p.push_error("type inside `sizeof`/`alignof`");
                return Spanned::new(Expr::Null, sp);
            };
            p.expect(&Token::RParen, "`)` to close `sizeof`/`alignof`");
            let end = p.previous_span();
            let e = if is_sizeof {
                Expr::Sizeof(ty)
            } else {
                Expr::Alignof(ty)
            };
            Spanned::new(e, sp.to(end))
        }
        Token::Match => parse_match_expr(p),
        Token::LBracket => {
            // Array literal: `[a, b, c]` or repeat form `[value; count]`.
            let sp = p.peek_span();
            p.advance();
            if p.at(&Token::RBracket) {
                p.advance();
                return Spanned::new(Expr::Array(Vec::new()), sp);
            }
            let first = parse_expression(p);
            if p.eat(&Token::Semicolon) {
                let count = parse_expression(p);
                p.expect(&Token::RBracket, "`]` to close array repeat");
                let end = p.previous_span();
                return Spanned::new(
                    Expr::ArrayRepeat {
                        value: Box::new(first),
                        count: Box::new(count),
                    },
                    sp.to(end),
                );
            }
            let mut elems = vec![first];
            while p.eat(&Token::Comma) {
                if p.at(&Token::RBracket) {
                    break;
                }
                elems.push(parse_expression(p));
            }
            p.expect(&Token::RBracket, "`]` to close array literal");
            let end = p.previous_span();
            Spanned::new(Expr::Array(elems), sp.to(end))
        }
        Token::LParen => {
            let sp = p.peek_span();

            // Explicit cast: `(i32) x`, `(float) y`, `(Player) p`.
            // Only when a type is followed by `)` and then a value — otherwise
            // this is an ordinary parenthesised expression.
            if p.is_type_start_at(1) {
                let save = p.pos();
                p.advance(); // `(`
                if let Some(ty) = crate::types::parse_type(p) {
                    if p.eat(&Token::RParen) && p.can_start_expression_at(0) {
                        let operand = parse_expr_bp(p, 14);
                        let end = p.previous_span();
                        return Spanned::new(
                            Expr::Cast {
                                expr: Box::new(operand),
                                ty,
                            },
                            sp.to(end),
                        );
                    }
                }
                p.set_pos(save);
            }

            p.advance();
            if p.at(&Token::RParen) {
                p.advance();
                Spanned::new(Expr::Tuple(vec![]), sp)
            } else {
                let first = parse_expression(p);
                if p.eat(&Token::Comma) {
                    let mut elems = vec![first];
                    while !p.at(&Token::RParen) && !p.at(&Token::Eof) {
                        elems.push(parse_expression(p));
                        if !p.eat(&Token::Comma) { break; }
                    }
                    p.expect(&Token::RParen, "`)` to close tuple");
                    let end = p.previous_span();
                    Spanned::new(Expr::Tuple(elems), sp.to(end))
                } else {
                    p.expect(&Token::RParen, "`)` to close parenthesized expression");
                    first
                }
            }
        }
        Token::LBrace => {
            let block = p.parse_block();
            Spanned::new(Expr::Block(block.node), block.span)
        }
        Token::Ident(_) | Token::SelfType => {
            let sp = p.peek_span();
            let segs = p.parse_path_segments();
            let path = Path { segments: segs };

            // Struct literal: `Player { pos: v, hp: 100 }`
            if p.at(&Token::LBrace) && !p.no_struct_lit {
                p.advance();
                let mut fields = Vec::new();
                while !p.at(&Token::RBrace) && !p.at(&Token::Eof) {
                    let Some(name) = p.expect_ident("struct literal field") else {
                        p.advance();
                        continue;
                    };
                    let value = if p.eat(&Token::Colon) {
                        parse_expression(p)
                    } else {
                        let sp2 = name.span;
                        Spanned::new(Expr::Ident(Path { segments: vec![name.clone()] }), sp2)
                    };
                    fields.push((name, value));
                    if !p.eat(&Token::Comma) { break; }
                }
                p.expect(&Token::RBrace, "`}` to close struct literal");
                let end = p.previous_span();
                Spanned::new(Expr::StructLiteral { path, fields }, sp.to(end))
            } else {
                Spanned::new(Expr::Ident(path), sp)
            }
        }
        _ => {
            let sp = p.peek_span();
            p.push_error("expression");
            p.advance();
            Spanned::new(Expr::Null, sp)
        }
    };

    parse_postfix(p, expr)
}

fn parse_postfix(p: &mut Parser, mut lhs: Spanned<Expr>) -> Spanned<Expr> {
    loop {
        match p.peek_tok().clone() {
            Token::Dot => {
                p.advance();
                let Some(name) = p.expect_ident("field or method name") else { break };
                if p.at(&Token::LParen) {
                    p.advance();
                    let mut args = Vec::new();
                    if !p.at(&Token::RParen) {
                        loop {
                            args.push(parse_expression(p));
                            if !p.eat(&Token::Comma) { break; }
                        }
                    }
                    p.expect(&Token::RParen, "`)` to close method call");
                    let end = p.previous_span();
                    let lhs_span = lhs.span;
                    lhs = Spanned::new(Expr::MethodCall { receiver: Box::new(lhs), method: name, args }, lhs_span.to(end));
                } else {
                    let end = name.span;
                    let lhs_span = lhs.span;
                    lhs = Spanned::new(Expr::Field { object: Box::new(lhs), field: name }, lhs_span.to(end));
                }
            }
            Token::ColonColon => {
                // Namespace access in expressions: ray.GetType(), etc.
                p.advance();
                let Some(name) = p.expect_ident("path segment") else { break };
                if p.at(&Token::LParen) {
                    p.advance();
                    let mut args = Vec::new();
                    if !p.at(&Token::RParen) {
                        loop {
                            args.push(parse_expression(p));
                            if !p.eat(&Token::Comma) { break; }
                        }
                    }
                    p.expect(&Token::RParen, "`)` to close call");
                    let end = p.previous_span();
                    let lhs_span = lhs.span;
                    let name_span = name.span;
                    let path = match lhs.node.clone() {
                        Expr::Ident(mut path) => {
                            path.segments.push(name);
                            path
                        }
                        _ => Path { segments: vec![Spanned::new(name.node.clone(), name_span)] },
                    };
                    lhs = Spanned::new(Expr::Call { callee: Box::new(Spanned::new(Expr::Ident(path), lhs_span)), args }, lhs_span.to(end));
                } else {
                    let lhs_span = lhs.span;
                    let name_span = name.span;
                    let path = match lhs.node.clone() {
                        Expr::Ident(mut path) => {
                            path.segments.push(name);
                            path
                        }
                        _ => Path { segments: vec![Spanned::new(name.node.clone(), name_span)] },
                    };
                    lhs = Spanned::new(Expr::Ident(path), lhs_span.to(name_span));
                }
            }
            Token::LParen => {
                p.advance();
                let mut args = Vec::new();
                if !p.at(&Token::RParen) {
                    loop {
                        args.push(parse_expression(p));
                        if !p.eat(&Token::Comma) { break; }
                    }
                }
                p.expect(&Token::RParen, "`)` to close call");
                let end = p.previous_span();
                let lhs_span = lhs.span;
                lhs = Spanned::new(Expr::Call { callee: Box::new(lhs), args }, lhs_span.to(end));
            }
            Token::LBracket => {
                p.advance();
                let index = parse_expression(p);
                p.expect(&Token::RBracket, "`]` to close index");
                let end = p.previous_span();
                let lhs_span = lhs.span;
                lhs = Spanned::new(
                    Expr::Index { object: Box::new(lhs), index: Box::new(index) },
                    lhs_span.to(end),
                );
            }
            Token::Plus if matches!(p.peek_n(1).node, Token::Plus) => {
                p.advance();
                p.advance();
                let end = p.previous_span();
                let lhs_span = lhs.span;
                lhs = Spanned::new(Expr::Incr { operand: Box::new(lhs), positive: true, is_postfix: true }, lhs_span.to(end));
            }
            Token::Minus if matches!(p.peek_n(1).node, Token::Minus) => {
                p.advance();
                p.advance();
                let end = p.previous_span();
                let lhs_span = lhs.span;
                lhs = Spanned::new(Expr::Incr { operand: Box::new(lhs), positive: false, is_postfix: true }, lhs_span.to(end));
            }
            _ => break,
        }
    }
    lhs
}

/// The parsed `for` header: either enhanced `(Type name : iterable)`
/// or classic `(init; cond; update)`.
pub enum ForRest {
    Enhanced {
        variable: Spanned<String>,
        iterable: Spanned<Expr>,
    },
    Classic {
        init: Option<Box<Spanned<york_ast::Stmt>>>,
        condition: Option<Spanned<Expr>>,
        update: Option<Box<Spanned<york_ast::Stmt>>>,
    },
}

/// Parse a classic or enhanced `for` header after the `for` keyword.
pub fn parse_for_rest(p: &mut Parser) -> Option<ForRest> {
    p.expect(&Token::LParen, "`(` after `for`")?;

    // Enhanced for: `(Type name : expr)` — detect via backtrack
    {
        let save = p.pos();
        // try: [type] ident ':'
        if let Some((name, _)) = try_parse_type_then_ident(p) {
            if p.at(&Token::Colon) {
                p.advance();
                let iterable = parse_expression(p);
                p.expect(&Token::RParen, "`)` to close enhanced-for");
                return Some(ForRest::Enhanced { variable: name, iterable });
            }
        }
        p.set_pos(save);
    }

    // Classic: `(init ; cond ; update)`
    let init = if p.at(&Token::Semicolon) {
        p.advance();
        None
    } else {
        let stmt = parse_for_init(p);
        p.expect(&Token::Semicolon, "`;` after for-init");
        Some(Box::new(stmt))
    };

    let condition = if p.at(&Token::Semicolon) {
        None
    } else {
        let c = parse_expression(p);
        p.expect(&Token::Semicolon, "`;` after for-condition")?;
        Some(c)
    };

    let update = if p.at(&Token::RParen) {
        None
    } else {
        let u = parse_expression(p);
        let span = u.span;
        let stmt = Spanned::new(york_ast::Stmt::Expr(u), span);
        Some(Box::new(stmt))
    };

    p.expect(&Token::RParen, "`)` to close for-header")?;

    Some(ForRest::Classic { init, condition, update })
}

fn try_parse_type_then_ident(p: &mut Parser) -> Option<(Spanned<String>, Spanned<york_ast::TypeAnnotation>)> {
    let save = p.pos();
    let ty = crate::types::parse_type(p)?;
    let name = p.expect_ident("loop variable")?;
    if p.at(&Token::Colon) {
        Some((name, Spanned::new(ty.node, ty.span)))
    } else {
        p.set_pos(save);
        None
    }
}

fn parse_for_init(p: &mut Parser) -> Spanned<york_ast::Stmt> {
    if let Some(stmt) = p.parse_local_decl(false) {
        return stmt;
    }
    let expr = parse_expression(p);
    let span = expr.span;
    Spanned::new(york_ast::Stmt::Expr(expr), span)
}