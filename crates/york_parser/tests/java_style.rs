use york_ast::{Item, Stmt, Expr, TypeAnnotation, PrimitiveType};
use york_lexer;

/// Parse a source string, failing the test if lexing/parsing produces errors.
fn parse_ok(src: &str) -> york_parser::ParseResult {
    let lex = york_lexer::lex(src);
    assert!(lex.errors.is_empty(), "lex errors: {:?}", lex.errors);
    let res = york_parser::parse(&lex.tokens);
    assert!(res.errors.is_empty(), "parse errors: {:?}", res.errors);
    res
}

#[test]
fn parses_java_style_function_decl() {
    let res = parse_ok("void main(String[] args) { print(\"hi\"); }");
    assert_eq!(res.program.items.len(), 1);
    let item = res.program.items[0].node.clone();
    match item {
        Item::Function(f) => {
            assert_eq!(f.name.node, "main");
            assert!(f.is_static == false);
            assert_eq!(f.params.len(), 1);
            let p = f.params[0].node.clone();
            assert_eq!(p.name.node, "args");
            match p.ty.node {
                TypeAnnotation::Slice(inner) => {
                    match inner.node {
                        TypeAnnotation::Primitive(PrimitiveType::String) => {}
                        other => panic!("expected String, got {:?}", other),
                    }
                }
                other => panic!("expected slice type, got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parses_public_static_void_main() {
    let res = parse_ok("public static void main(String[] args) { print(\"hello\"); }");
    let item = res.program.items[0].node.clone();
    match item {
        Item::Function(f) => {
            assert_eq!(f.name.node, "main");
            assert!(f.is_static, "expected static");
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parses_struct_with_java_fields() {
    let res = parse_ok("struct Player { Vec2 pos; Vec2 vel; bool active; }");
    let item = res.program.items[0].node.clone();
    match item {
        Item::Struct(s) => {
            assert_eq!(s.name.node, "Player");
            assert_eq!(s.fields.len(), 3);
            assert_eq!(s.fields[0].node.name.node, "pos");
            assert_eq!(s.fields[2].node.name.node, "active");
        }
        other => panic!("expected Struct, got {:?}", other),
    }
}

#[test]
fn parses_impl_block_java_methods() {
    let res = parse_ok(
        "struct Player { Vec2 pos; }
         impl Player {
             void update(float dt) { this.pos.x += dt; }
             static Player create(Vec2 pos) {
                 Player p;
                 p.pos = pos;
                 return p;
             }
         }",
    );
    let item = res.program.items[0].node.clone();
    match item {
        Item::Struct(_) => {
            let impl_item = res.program.items[1].node.clone();
            match impl_item {
                Item::Impl(imp) => {
                    assert_eq!(imp.methods.len(), 2);
                    let static_m = imp.methods.iter().find(|m| m.node.is_static);
                    assert!(static_m.is_some(), "expected a static method create");
                }
                other => panic!("expected Impl, got {:?}", other),
            }
        }
        other => panic!("expected Struct, got {:?}", other),
    }
}

#[test]
fn parses_classic_for_loop() {
    let res = parse_ok(
        "void main() {
            for (int i = 0; i < 1000; i++) {
                print(i);
            }
         }",
    );
    let item = res.program.items[0].node.clone();
    match item {
        Item::Function(f) => {
            let body = f.body.unwrap();
            assert_eq!(body.stmts.len(), 1);
            match &body.stmts[0].node {
                Stmt::ForC { init, condition, update, body } => {
                    assert!(init.is_some());
                    assert!(condition.is_some());
                    assert!(update.is_some());
                    match &body.stmts[0].node {
                        Stmt::Expr(_) => { /* nested print is fine */ }
                        other => panic!("unexpected body stmt: {:?}", other),
                    }
                    assert_eq!(body.stmts.len(), 1);
                }
                other => panic!("expected ForC, got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parses_enhanced_for_loop() {
    let res = parse_ok(
        "void main() {
            Arena<Player> players = new Arena(1000);
            for (Player p : players) {
                p.update(0.016);
            }
         }",
    );
    // Just verify no errors — detailed stmt check below.
    let item = res.program.items[0].node.clone();
    match item {
        Item::Function(f) => {
            let body = f.body.unwrap();
            // stmt0: typed decl `Arena<Player> players = new Arena(1000)`
            match &body.stmts[0].node {
                Stmt::Let { name, ty, value } => {
                    assert_eq!(name.node, "players");
                    match &ty.as_ref().unwrap().node {
                        TypeAnnotation::Generic { base, args } => {
                            assert_eq!(base.segments[0].node, "Arena");
                            assert_eq!(args.len(), 1);
                            match &args[0].node {
                                TypeAnnotation::Named(p) => assert_eq!(p.segments[0].node, "Player"),
                                other => panic!("expected Named Player, got {:?}", other),
                            }
                        }
                        other => panic!("expected Generic, got {:?}", other),
                    }
                    match &value.as_ref().unwrap().node {
                        Expr::New { args, .. } => {
                            assert_eq!(args.len(), 1);
                            if let Expr::Int(n) = args[0].node {
                                assert_eq!(n, 1000);
                            } else {
                                panic!("expected Int arg");
                            }
                        }
                        other => panic!("expected New, got {:?}", other),
                    }
                }
                other => panic!("expected Let, got {:?}", other),
            }
            // stmt1: enhanced for
            match &body.stmts[1].node {
                Stmt::For { variable, iterable, .. } => {
                    assert_eq!(variable.node, "p");
                    match &iterable.node {
                        Expr::Ident(path) => assert_eq!(path.segments[0].node, "players"),
                        other => panic!("expected Ident, got {:?}", other),
                    }
                }
                other => panic!("expected For, got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parses_struct_literal() {
    let res = parse_ok(
        "void main() {
            Vec2 at;
            Player p = Player { pos: at, active: true };
         }",
    );
    assert_eq!(res.program.items.len(), 1);
}

#[test]
fn parses_imports_with_dots() {
    let res = parse_ok("import york.io.print;\nimport york.mem.Arena;\nvoid main() {}");
    assert_eq!(res.program.items.len(), 3);
    match res.program.items[0].node.clone() {
        Item::Import(imp) => {
            let segs: Vec<&str> = imp.path.iter().map(|s| s.node.as_str()).collect();
            assert_eq!(segs, vec!["york", "io", "print"]);
        }
        other => panic!("expected Import, got {:?}", other),
    }
}

#[test]
fn parses_method_call_chain() {
    let res = parse_ok(
        "void main() {
            players.get(5).update(0.016);
         }",
    );
    let item = res.program.items[0].node.clone();
    match item {
        Item::Function(f) => {
            let body = f.body.unwrap();
            match &body.stmts[0].node {
                Stmt::Expr(e) => match &e.node {
                    Expr::MethodCall { .. } => {}
                    other => panic!("expected MethodCall, got {:?}", other),
                },
                other => panic!("expected Expr stmt, got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parses_untyped_local_with_java_syntax() {
    let res = parse_ok("void main() { int count = players.count(); }");
    assert_eq!(res.errors.len(), 0);
}

#[test]
fn parses_switch_statement() {
    let res = parse_ok(
        r#"enum Color { Red, Green, Blue }
        void main() {
            Color c = Color.Red;
            switch (c) {
                case Color.Red:
                    println("red");
                    break;
                case Color.Green:
                case Color.Blue:
                    println("other");
                    break;
                default:
                    println("unknown");
            }
        }"#,
    );
    assert_eq!(res.program.items.len(), 2);
    match res.program.items[0].node.clone() {
        Item::Enum(e) => {
            assert_eq!(e.name.node, "Color");
            assert_eq!(e.variants.len(), 3);
        }
        other => panic!("expected Enum, got {:?}", other),
    }
    match &res.program.items[1].node {
        Item::Function(f) => {
            let body = f.body.as_ref().unwrap();
            match &body.stmts[1].node {
                Stmt::Switch { scrutinee, arms } => {
                    match &scrutinee.node {
                        Expr::Ident(path) => assert_eq!(path.segments[0].node, "c"),
                        other => panic!("expected Ident scrutinee, got {:?}", other),
                    }
                    // 4 arms: Red, Green (empty, falls through), Blue, default
                    assert_eq!(arms.len(), 4);
                    // First arm label is `Color.Red` (field access).
                    match arms[0].label.as_ref().unwrap().node.clone() {
                        Expr::Field { object, field } => {
                            match object.node {
                                Expr::Ident(path) => assert_eq!(path.segments[0].node, "Color"),
                                other => panic!("expected Ident Color, got {:?}", other),
                            }
                            assert_eq!(field.node, "Red");
                        }
                        other => panic!("expected Field label, got {:?}", other),
                    }
                    // Green arm has an empty body (falls through into Blue).
                    assert!(arms[1].label.is_some());
                    assert!(arms[1].body.is_empty());
                    // Last arm is default.
                    assert!(arms[3].label.is_none());
                }
                other => panic!("expected Switch stmt, got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}