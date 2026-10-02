//! End-to-end tests for the built-in `HashMap<K, V>` collection.

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
    let dir = std::env::temp_dir().join(format!("york_hm_{}_{count}", std::process::id()));
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
fn hashmap_string_keys_end_to_end() {
    let src = r#"
public static void main(String[] args) {
    HashMap<string, int> ages = new HashMap<string, int>();
    ages.put("ana", 31);
    ages.put("bo", 24);
    ages.put("cy", 45);
    println(ages.count());
    println(ages.get("bo"));
    println(ages.get("cy"));
    println(ages.contains("ana"));
    println(ages.contains("zz"));
    println(ages.get_or("zz", -1));
    ages.put("bo", 25);
    println(ages.get("bo"));
    println(ages.count());
    println(ages.remove("ana"));
    println(ages.count());
    println(ages.remove("ana"));
}
"#;
    let out = compile_run(src).expect("hashmap program should compile and run");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines,
        vec!["3", "24", "45", "1", "0", "-1", "25", "3", "1", "2", "0"]
    );
}

#[test]
fn hashmap_int_keys_end_to_end() {
    let src = r#"
public static void main(String[] args) {
    HashMap<int, int> squares = new HashMap<int, int>();
    for (int i = 0; i < 200; i = i + 1) {
        squares.put(i, i * i);
    }
    println(squares.count());
    println(squares.get(199));
    println(squares.get(7));
    println(squares.contains(150));
    println(squares.contains(500));

    int ok = 0;
    for (int i = 0; i < 200; i = i + 1) {
        if (squares.get(i) == i * i) {
            ok = ok + 1;
        }
    }
    println(ok);
}
"#;
    let out = compile_run(src).expect("hashmap program should compile and run");
    let lines: Vec<&str> = out.lines().collect();
    // Every entry must survive repeated grow-and-rehash.
    assert_eq!(lines, vec!["200", "39601", "49", "1", "0", "200"]);
}

#[test]
fn hashmap_as_struct_field_end_to_end() {
    let src = r#"
struct Registry {
    HashMap<string, int> sessions;
}

public static void main(String[] args) {
    Registry r = new Registry();
    r.sessions.put("s-1", 100);
    r.sessions.put("s-2", 250);
    println(r.sessions.count());
    println(r.sessions.get("s-2"));
    println(r.sessions.get("s-3"));
    r.sessions.clear();
    println(r.sessions.count());
    println(r.sessions.is_empty());
}
"#;
    let out = compile_run(src).expect("hashmap program should compile and run");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, vec!["2", "250", "0", "0", "1"]);
}

#[test]
fn hashmap_string_values_round_trip() {
    let src = r#"
public static void main(String[] args) {
    HashMap<int, string> ports = new HashMap<int, string>();
    ports.put(80, "http");
    ports.put(443, "https");
    println(ports.get(443));
    println(ports.count());
    HashMap<string, string> env = new HashMap<string, string>();
    env.put("mode", "release");
    println(env.get("mode"));
    println(env.get("nope"));
}
"#;
    let out = compile_run(src).expect("hashmap program should compile and run");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, vec!["https", "2", "release", ""]);
}