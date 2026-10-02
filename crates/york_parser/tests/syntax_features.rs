//! Parser tests for array literals, explicit casts, sizeof/alignof, and match.

use york_ast::ast::{Expr, Stmt};
use york_ast::span::Spanned;
use york_ast::Item;

fn parse_ok(src: &str) -> york_parser::ParseResult {
    let lex = york_lexer::lex(src);
    assert!(lex.errors.is_empty(), "lex errors: {:?}", lex.errors);
    let res = york_parser::parse(&lex.tokens);
    assert!(res.errors.is_empty(), "parse errors: {:?}", res.errors);
    res
}

/// Body statements of the first function in `src`.
fn first_fn_body(src: &str) -> Vec<Spanned<Stmt>> {
    let res = parse_ok(src);
    fn_body(&res, 0)
}

fn fn_body(res: &york_parser::ParseResult, index: usize) -> Vec<Spanned<Stmt>> {
    match &res.program.items[index].node {
        Item::Function(f) => f
            .body
            .as_ref()
            .expect("function should have a body")
            .stmts
            .clone(),
        other => panic!("expected Function, got {other:?}"),
    }
}

#[test]
fn parses_array_literal() {
    let body = first_fn_body("fn main() { int[] xs = [1, 2, 3]; }");
    let Stmt::Let { value: Some(value), .. } = &body[0].node else {
        panic!("expected Var");
    };
    assert!(matches!(value.node, Expr::Array(_)));
}

#[test]
fn parses_empty_array_literal() {
    parse_ok("fn main() { int[] xs = []; }");
}

#[test]
fn parses_array_repeat() {
    let res = parse_ok("fn main() { int[] xs = [7; 6]; }");
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 0)[0].node else {
        panic!("expected Var");
    };
    assert!(matches!(value.node, Expr::ArrayRepeat { .. }));
}

#[test]
fn parses_explicit_cast_expression() {
    let res = parse_ok("fn main() { int x = (int) 3.99; }");
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 0)[0].node else {
        panic!("expected Var");
    };
    assert!(matches!(value.node, Expr::Cast { .. }));
}

#[test]
fn parses_cast_of_struct_type() {
    let res = parse_ok("struct P { x: int }\nfn main() { P p = (P) 0; }");
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 1)[0].node else {
        panic!("expected Var");
    };
    assert!(matches!(value.node, Expr::Cast { .. }));
}

#[test]
fn parses_sizeof_and_alignof() {
    let res = parse_ok("fn main() { int a = sizeof(int); int b = alignof(double); }");
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 0)[0].node else {
        panic!("expected Var");
    };
    assert!(matches!(value.node, Expr::Sizeof(_)));
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 0)[1].node else {
        panic!("expected Var");
    };
    assert!(matches!(value.node, Expr::Alignof(_)));
}

#[test]
fn size_takes_struct_type_argument() {
    parse_ok("struct P { x: int }\nfn main() { int a = sizeof(P); }");
}

#[test]
fn parses_match_with_literal_and_wildcard_arms() {
    let res = parse_ok("fn main() { int x = 1; int y = match x { 1 => 10, _ => 20 }; }");
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 0)[1].node else {
        panic!("expected Var");
    };
    let Expr::Match { arms, .. } = &value.node else {
        panic!("expected Match, got {:?}", value.node);
    };
    assert_eq!(arms.len(), 2);
}

#[test]
fn parses_match_with_guard_and_binding() {
    let res = parse_ok("fn main() { int y = match 3 { n if n > 2 => 1, _ => 0 }; }");
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 0)[0].node else {
        panic!("expected Var");
    };
    let Expr::Match { arms, .. } = &value.node else {
        panic!("expected Match");
    };
    assert!(arms[0].guard.is_some());
}

#[test]
fn parses_match_with_enum_and_tuple_patterns() {
    parse_ok(
        "enum E { A, B }\nfn main() { int y = match 1 { E.A => 1, (1, 2) => 2, _ => 3 }; }",
    );
}

#[test]
fn parenthesised_expression_is_not_a_cast() {
    let res = parse_ok("fn main() { int x = (1 + 2) * 3; }");
    let Stmt::Let { value: Some(value), .. } = &fn_body(&res, 0)[0].node else {
        panic!("expected Var");
    };
    assert!(!matches!(value.node, Expr::Cast { .. }));
}

#[test]
fn for_in_body_is_not_parsed_as_struct_literal() {
    let body = first_fn_body(
        "fn main() { Arena<int> xs = new Arena<int>(); for x in xs { println(x); } }",
    );
    assert!(body
        .iter()
        .any(|s| matches!(s.node, Stmt::For { .. })));
}