//! End-to-end tests for array literals, explicit casts, and sizeof/alignof.

use std::process::Command;

fn compile_run(source: &str) -> Result<String, String> {
    let lexed = york_lexer::lex(source);
    if !lexed.errors.is_empty() {
        return Err(format!("lex errors: {:?}", lexed.errors));
    }
    let parsed = york_parser::parse(&lexed.tokens);
    if !parsed.errors.is_empty() {
        return Err(format!("parse errors: {:?}", parsed.errors));
    }
    let sema = york_sema::analyze(&parsed.program);
    if !sema.errors.is_empty() {
        return Err(format!("sema errors: {:?}", sema.errors));
    }
    let c_source = york_codegen_c::generate(&sema.program);

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("york_syn_{}_{count}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let exe = dir.join("out");
    york_codegen_c::compile_c(&c_source, exe.to_str().unwrap())
        .map_err(|e| format!("{e}\n---C SOURCE---\n{c_source}"))?;

    let exe_path = if cfg!(windows) {
        format!("{}.exe", exe.display())
    } else {
        exe.display().to_string()
    };
    let out = Command::new(&exe_path)
        .output()
        .map_err(|e| format!("failed to run binary: {e}"))?;
    let _ = std::fs::remove_dir_all(&dir);
    if !out.status.success() {
        return Err(format!(
            "binary exited with {}\nstdout: {}\nstderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[test]
fn array_literal_indexes_and_iterates() {
    let src = r#"
public static void main(String[] args) {
    int[] xs = [10, 20, 30, 40];
    println(xs[0]);
    println(xs[3]);

    int sum = 0;
    for (int i = 0; i < 4; i = i + 1) {
        sum += xs[i];
    }
    println(sum);

    xs[1] = 99;
    println(xs[1]);
}
"#;
    let out = compile_run(src).expect("array literal program should run");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["10", "40", "100", "99"]);
}

#[test]
fn array_repeat_expands_at_compile_time() {
    let src = r#"
public static void main(String[] args) {
    int[] pad = [7; 4];
    println(pad[0]);
    println(pad[3]);

    string[] names = ["a"; 2];
    println(names[1]);
}
"#;
    let out = compile_run(src).expect("array repeat program should run");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["7", "7", "a"]);
}

#[test]
fn array_literal_elements_adopt_declared_type() {
    // Integer literals infer as i64; the declared element type must win.
    let src = r#"
public static void main(String[] args) {
    long[] wide = [1, 2];
    println(wide[1]);
    println(sizeof(long));
}
"#;
    let out = compile_run(src).expect("array literal should adopt declared element type");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["2", "8"]);
}

#[test]
fn empty_array_literal_takes_declared_type() {
    let src = r#"
public static void main(String[] args) {
    int[] empty = [];
    println(sizeof(int));
}
"#;
    compile_run(src).expect("empty array literal should compile");
}

#[test]
fn sizeof_and_alignof_are_compile_time_constants() {
    let src = r#"
struct Pair { a: int, b: int }

public static void main(String[] args) {
    println(sizeof(int));
    println(alignof(int));
    println(sizeof(Pair));
    println(sizeof(double));
}
"#;
    let out = compile_run(src).expect("sizeof/alignof program should run");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["4", "4", "8", "8"]);
}

#[test]
fn explicit_casts_convert_numerics() {
    let src = r#"
public static void main(String[] args) {
    int truncated = (int) 3.99;
    println(truncated);

    double widened = (double) 7;
    println(widened);

    int back = (int) 2.5;
    println(back + 1);
}
"#;
    let out = compile_run(src).expect("cast program should run");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["3", "7", "3"]);
}

#[test]
fn casting_zero_produces_struct_zero_value() {
    let src = r#"
struct Point { x: int, y: int }

public static void main(String[] args) {
    Point p = (Point) 0;
    println(p.x);
    println(p.y);
}
"#;
    let out = compile_run(src).expect("zero cast program should run");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["0", "0"]);
}

#[test]
fn parentheses_are_still_grouping_expressions() {
    // A cast must not swallow ordinary parenthesised expressions.
    let src = r#"
fn add(a: int, b: int) -> int { return a + b; }

public static void main(String[] args) {
    println((1 + 2) * 3);
    println(add((2), (3)));
}
"#;
    let out = compile_run(src).expect("grouping program should run");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["9", "5"]);
}

#[test]
fn for_in_loop_body_is_not_a_struct_literal() {
    // Regression: `for x in xs { ... }` used to parse the `{` as a struct
    // literal on the iterable expression.
    let src = r#"
public static void main(String[] args) {
    Arena<int> xs = new Arena<int>();
    xs.push(1);
    xs.push(2);

    int total = 0;
    for x in xs {
        total += x;
    }
    println(total);
}
"#;
    let out = compile_run(src).expect("for-in over arena should run");
    assert_eq!(out.lines().collect::<Vec<_>>(), ["3"]);
}