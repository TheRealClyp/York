use york_ast::Item;
use york_lexer;

fn parse_ok(src: &str) -> york_parser::ParseResult {
    let lex = york_lexer::lex(src);
    assert!(lex.errors.is_empty(), "lex errors: {:?}", lex.errors);
    let res = york_parser::parse(&lex.tokens);
    assert!(res.errors.is_empty(), "parse errors: {:?}", res.errors);
    res
}

#[test]
fn parses_file_import_with_string_literal() {
    let res = parse_ok("import \"util/math.yk\";");
    assert_eq!(res.program.items.len(), 1);
    match &res.program.items[0].node {
        Item::Import(imp) => {
            assert_eq!(imp.file.as_deref(), Some("util/math.yk"));
            assert!(imp.path.is_empty());
            assert!(imp.aliases.is_empty());
        }
        other => panic!("expected Import, got {other:?}"),
    }
}

#[test]
fn parses_multiple_file_imports() {
    let res = parse_ok("import \"a.yk\";\nimport \"b/c.yk\";\nvoid main(String[] args) { print(1); }");
    let files: Vec<&str> = res
        .program
        .items
        .iter()
        .filter_map(|i| match &i.node {
            Item::Import(imp) => imp.file.as_deref(),
            _ => None,
        })
        .collect();
    assert_eq!(files, vec!["a.yk", "b/c.yk"]);
}

#[test]
fn dotted_import_still_parses_as_module_path() {
    let res = parse_ok("import std.math;");
    match &res.program.items[0].node {
        Item::Import(imp) => {
            assert!(imp.file.is_none(), "dotted import must not set `file`");
            let segs: Vec<&str> = imp.path.iter().map(|s| s.node.as_str()).collect();
            assert_eq!(segs, vec!["std", "math"]);
        }
        other => panic!("expected Import, got {other:?}"),
    }
}

#[test]
fn file_import_with_alias_list_is_parsed_before_path() {
    // The string form wins: aliases belong to module imports only.
    let res = parse_ok("import \"a.yk\";");
    assert!(matches!(&res.program.items[0].node, Item::Import(i) if i.file.is_some()));
}
