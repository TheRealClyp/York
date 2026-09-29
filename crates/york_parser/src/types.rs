use york_ast::span::{Spanned, Span};
use york_ast::{PrimitiveType, TypeAnnotation, Path};
use york_ast::Token;

use crate::Parser;

/// Parse a type annotation. Java/JS-style:
///   - primitives: int float double bool long char void
///   - named: Vec2, york.math.Vec2 (:: or . separators)
///   - generic: Arena<Player>, Map<String, int>
///   - arrays:  int[]  int[4]  String[]
///   - optional-postfix: int? Player?
pub fn parse_type(p: &mut Parser) -> Option<Spanned<TypeAnnotation>> {
    let start = p.peek_span();
    let base = parse_base_type(p)?;
    let ty = parse_type_postfix(p, base);
    Some(Spanned::new(ty, start.to(p.previous_span())))
}

fn parse_base_type(p: &mut Parser) -> Option<TypeAnnotation> {
    let tok = p.peek_tok().clone();
    match tok {
        // primitives
        Token::TyI8 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::I8)) }
        Token::TyI16 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::I16)) }
        Token::TyI32 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::I32)) }
        Token::TyI64 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::I64)) }
        Token::TyI128 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::I128)) }
        Token::TyU8 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::U8)) }
        Token::TyU16 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::U16)) }
        Token::TyU32 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::U32)) }
        Token::TyU64 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::U64)) }
        Token::TyU128 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::U128)) }
        Token::TyF16 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::F16)) }
        Token::TyF32 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::F32)) }
        Token::TyF64 => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::F64)) }
        Token::TyBool => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::Bool)) }
        Token::TyChar => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::Char)) }
        Token::TyString => { p.advance(); Some(TypeAnnotation::Primitive(PrimitiveType::String)) }
        Token::TyVoid => { p.advance(); Some(TypeAnnotation::Void) }
        Token::TyNever => { p.advance(); Some(TypeAnnotation::Never) }
        Token::SelfType => {
            p.advance();
            Some(TypeAnnotation::Named(p.single_path("Self")))
        }
        Token::Arena => {
            p.advance();
            Some(TypeAnnotation::Named(p.single_path("Arena")))
        }
        Token::Ident(_) => {
            let segs = p.parse_path_segments();
            Some(TypeAnnotation::Named(Path { segments: segs }))
        }
        Token::LParen => {
            p.advance();
            let mut elems = Vec::new();
            while !p.at(&Token::RParen) && !p.at(&Token::Eof) {
                let Some(t) = parse_type(p) else { p.advance(); continue };
                elems.push(t);
                if !p.eat(&Token::Comma) { break; }
            }
            p.expect(&Token::RParen, "`)` to close tuple type");
            if elems.is_empty() {
                Some(TypeAnnotation::Void)
            } else {
                Some(TypeAnnotation::Tuple(elems))
            }
        }
        _ => None,
    }
}

fn parse_type_postfix(p: &mut Parser, base: TypeAnnotation) -> TypeAnnotation {
    let mut current = base;
    // Generic args: Arena<Player>  (only after a Named path)
    if let TypeAnnotation::Named(_) = &current {
        if p.at(&Token::Lt) {
            let mut args = Vec::new();
            let save = p.pos();
            p.advance();
            let mut ok = true;
            loop {
                let Some(t) = parse_type(p) else { ok = false; break };
                args.push(t);
                if p.at(&Token::Comma) { p.advance(); continue; }
                if p.at(&Token::Gt) { p.advance(); break; }
                ok = false;
                break;
            }
            if ok && !args.is_empty() {
                let base_path = match &current {
                    TypeAnnotation::Named(path) => path.clone(),
                    _ => unreachable!(),
                };
                current = TypeAnnotation::Generic { base: base_path, args };
            } else {
                p.set_pos(save);
            }
        }
    }

    loop {
        if p.at(&Token::LBracket) {
            // int[] slice
            let before = p.peek_span().lo;
            p.advance();
            if p.at(&Token::RBracket) {
                p.advance();
                let after = p.previous_span().hi;
                current = TypeAnnotation::Slice(Box::new(Spanned::new(current, Span::new(before, after))));
                continue;
            }
        }
        if p.at(&Token::Question) {
            let before = p.peek_span().lo;
            p.advance();
            let after = p.previous_span().hi;
            current = TypeAnnotation::Optional(Box::new(Spanned::new(current, Span::new(before, after))));
            continue;
        }
        break;
    }

    current
}

/// Used by the parser to build a one-segment path (e.g. "Self").
#[allow(dead_code)]
pub(crate) fn make_path(name: &str) -> Path {
    let span = Span::new(york_ast::BytePos::ZERO, york_ast::BytePos::ZERO);
    Path { segments: vec![Spanned::new(name.to_string(), span)] }
}