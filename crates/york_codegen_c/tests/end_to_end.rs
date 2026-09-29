//! End-to-end: York source → lexer → parser → sema → C codegen → system CC → run.

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
    let dir = std::env::temp_dir().join(format!("york_e2e_{}_{count}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let exe = dir.join("out");
    york_codegen_c::compile_c(&c_source, exe.to_str().unwrap()).map_err(|e| {
        format!("{e}\n---C SOURCE---\n{c_source}")
    })?;

    let exe_path = if cfg!(windows) {
        format!("{}.exe", exe.display())
    } else {
        exe.display().to_string()
    };
    let out = Command::new(&exe_path)
        .output()
        .map_err(|e| format!("failed to run binary: {e}"))?;
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
fn hello_arena_end_to_end() {
    let src = r#"
struct Player {
    float pos;
    float hp;
    bool active;
}

impl Player {
    void damage(float amount) {
        this.hp -= amount;
        if (this.hp < 0.0) {
            this.hp = 0.0;
            this.active = false;
        }
    }
}

public static void main(String[] args) {
    Arena<Player> players = new Arena(4);
    players.push(Player { pos: 1.0, hp: 100.0, active: true });
    players.push(Player { pos: 2.0, hp: 100.0, active: true });
    players.push(Player { pos: 3.0, hp: 100.0, active: true });

    println(players.count());
    for (Player p : players) {
        p.damage(10.0);
        println(p.hp);
    }

    for (int i = 0; i < players.count(); i++) {
        println(players.get(i).hp);
    }
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            // 3 players, each 100 - 10 = 90.
            assert_eq!(stdout.lines().map(str::trim).collect::<Vec<_>>(), vec![
                "3", "90", "90", "90", "100", "100", "100",
            ]);
        }
        Err(e) => panic!("{e}"),
    }
}

/// York source -> C must include a real `assert`, multi-arg println, string
/// methods, and a deterministic rand path.
#[test]
fn test_features_strings_and_assert() {
    let src = r#"
public static void main(String[] args) {
    // Multi-arg print/println concatenation.
    println("n=", 7);
    println("pi=", 3.5, " bool=", true);

    // String methods.
    String s = "  York  ";
    println(s.trim().toUpper());
    println(s.trim().toLower());
    println(s.len());
    println("hello".len());
    println("Hello World".contains("World"));
    println("hello".startsWith("he"));
    println("hello".endsWith("lo"));
    println("abcdef".substring(1, 4));
    println("  a  b ".trim() == "a  b");

    // assert: passing flavor only.
    assert(true);
    assert(3 < 5, "math works");
    println("assert-ok");

    // Fixed: foreach snapshot (iterable side effect evaluated once).
    Arena<int> xs = new Arena(2);
    xs.push(1);
    xs.push(2);
    for (int v : xs) {
        println(v);
    }

    // Switch arms must not fall through.
    int day = 2;
    switch (day) {
        case 1:
            println("monday");
        case 2:
            println("tuesday");
        default:
            println("any");
    }

    // rand compiles and clamps to >= 0.
    srand(1);
    int r = rand();
    if (r >= 0) {
        println("rand-ok");
    }
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            assert_eq!(stdout.lines().map(str::trim).collect::<Vec<_>>(), vec![
                "n=7",          // println("n=", 7)
                "pi=3.5 bool=true", // println("pi=", 3.5, " bool=", true)
                "YORK",          // s.trim().toUpper()
                "york",          // s.trim().toLower()
                "8",             // s.len() ("  York  " is 8 chars)
                "5",             // "hello".len()
                "1",             // contains("World")
                "1",             // startsWith("he")
                "1",             // endsWith("lo")
                "bcd",           // substring(1, 4)
                "1",             // trim equality
                "assert-ok",
                "1",             // foreach
                "2",             // foreach
                "tuesday",       // switch: case 2 only, no fallthrough
                "rand-ok",
            ]);
        }
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn test_020_math_and_string_growth() {
    let src = r#"
public static void main(String[] args) {
    // Math helpers: min/max/clamp (int + float).
    println(min(3, 7));
    println(max(3, 7));
    println(min(2.5, 1.5));
    println(max(2.5, 1.5));
    println(clamp(100, 0, 10));
    println(clamp(-5, 0, 10));
    println(clamp(5, 0, 10));
    println(clamp(7.5, 1.0, 5.0));

    // String growth methods.
    String s = "Hello York";
    println(s.indexOf("York"));
    println(s.indexOf("zzz"));
    println(s.lastIndexOf("o"));
    println(s.charAt(1));
    println("ab".repeat(3));
    println("".repeat(3) == "");
    println("".isEmpty());
    println("x".isEmpty());
    println("Hello".indexOf("ell"));
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            assert_eq!(stdout.lines().map(str::trim).collect::<Vec<_>>(), vec![
                "3",            // min(3, 7)
                "7",            // max(3, 7)
                "1.5",          // min(2.5, 1.5)
                "2.5",          // max(2.5, 1.5)
                "10",           // clamp(100, 0, 10)
                "0",            // clamp(-5, 0, 10)
                "5",            // clamp(5, 0, 10)
                "5",            // clamp(7.5, 1.0, 5.0)
                "6",            // "Hello York".indexOf("York")
                "-1",           // "Hello York".indexOf("zzz")
                "7",            // "Hello York".lastIndexOf("o")
                "e",            // charAt(1)
                "ababab",       // "ab".repeat(3)
                "1",            // repeat(0) == ""
                "1",            // "".isEmpty()
                "0",            // "x".isEmpty()
                "1",            // "Hello".indexOf("ell")
            ]);
        }
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn test_021_string_transform_and_now() {
    let src = r#"
public static void main(String[] args) {
    println("a-b-c".replace("-b-", "x"));
    println("hello".replace("l", "L"));
    println("abc".reverse());
    println("   x  ".trimEnd() == "   x");
    println("   x  ".trimStart() == "x  ");
    print("go,");
    long t0 = now();
    sleep(50);
    long t1 = now();
    println(t1 >= t0);
    println(t1 - t0 >= 50);
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            assert_eq!(stdout.lines().map(str::trim).collect::<Vec<_>>(), vec![
                "axc",      // replace("-b-", "x") on "a-b-c"
                "heLLo",    // replace("l", "L")
                "cba",      // reverse
                "1",        // trimEnd() == "   x"
                "1",        // trimStart() == "x  "
                "go,1",     // print + now/sleep monotonic
                "1",        // elapsed ms >= 50
            ]);
        }
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn test_02x_type_conversions_and_padding() {
    let src = r#"
public static void main(String[] args) {
    println(to_int("42") + 1);
    println(to_int("-7"));
    println(to_float("3.5") + 0.25);
    println("n=", to_int("123") * 2);
    println("7".padLeft(3, "0"));
    println("bad".padLeft(6, "x"));
    println("7".padRight(3, "0"));
    println("ab".padRight(5, "-"));
    println("abc".padLeft(2, "0") == "abc");
    println("".isEmpty());
    println("n".repeat(0) == "");
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            assert_eq!(stdout.lines().map(str::trim).collect::<Vec<_>>(), vec![
                "43",       // to_int("42") + 1
                "-7",       // to_int("-7")
                "3.75",     // to_float("3.5") + 0.25
                "n=246",    // print + to_int in multi-arg
                "007",      // "7".padLeft(3, "0")
                "xxxbad",   // "bad".padLeft(6, "x")
                "700",      // "7".padRight(3, "0")
                "ab---",    // "ab".padRight(5, "-")
                "1",        // no-op pad when already long enough
                "1",        // "".isEmpty()
                "1",        // repeat(0) == ""
            ]);
        }
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn test_language_improvements() {
    let src = r#"
public static void main(String[] args) {
    // String equality with strcmp
    String s1 = "york_lang";
    String s2 = "york_lang";
    String s3 = "other";
    println(s1 == s2);
    println(s1 == s3);
    println(s1 != s3);

    // Arena improvements: push, set, last, pop, count
    Arena<int> nums = new Arena(4);
    nums.push(10);
    nums.push(20);
    nums.set(0, 99);
    println(nums.get(0));
    println(nums.last());
    println(nums.pop());
    println(nums.count());

    // File I/O: write_file, file_exists, read_file
    write_file("test_york_tmp.txt", "hello_york_systems");
    println(file_exists("test_york_tmp.txt"));
    String content = read_file("test_york_tmp.txt");
    println(content == "hello_york_systems");
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
            assert_eq!(lines, vec![
                "1",  // s1 == s2 (true)
                "0",  // s1 == s3 (false)
                "1",  // s1 != s3 (true)
                "99", // nums.get(0) after set
                "20", // nums.last()
                "20", // nums.pop()
                "1",  // nums.count()
                "1",  // file_exists (true)
                "1",  // content == "hello_york_systems" (true)
            ]);
            let _ = std::fs::remove_file("test_york_tmp.txt");
        }
        Err(e) => {
            let _ = std::fs::remove_file("test_york_tmp.txt");
            panic!("{e}");
        }
    }
}

#[test]
fn test_trig_log_env_platform() {
    let src = r#"
public static void main(String[] args) {
    println(sin(0.0));
    println(cos(0.0));
    println(tan(0.0));
    println(ln(1.0));
    println(ln(exp(2.0)));
    println(log10(1000.0));
    println(exp(0.0));
    println(round(sin(1.5707963267948966)));
    println(env("PATH").len() > 0);
    println(env("YORK_NO_SUCH_VAR_XYZ"));
    String p = platform_name();
    println(p == "windows" || p == "linux" || p == "macos" || p == "freebsd");
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
            assert_eq!(lines, vec![
                "0",              // sin(0)
                "1",              // cos(0)
                "0",              // tan(0)
                "0",              // ln(1)
                "2",              // ln(exp(2))
                "3",              // log10(1000)
                "1",              // exp(0)
                "1",              // round(sin(pi/2))
                "1",              // env("PATH") non-empty
                "",               // env(unknown)
                "1",              // platform_name is a known OS
            ]);
        }
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn test_random_range_and_str_count() {
    let src = r#"
public static void main(String[] args) {
    srand(42);
    int in_range = 1;
    for (int i = 0; i < 50; i++) {
        int r = random_range(5, 10);
        if (r < 5 || r >= 10) {
            in_range = 0;
        }
    }
    println(in_range);

    String a = "banana banana banana";
    println(a.count("banana"));
    println(a.count("a"));
    println(a.count(""));
    println(a.count("zzz"));
    println("aaa".count("aa"));
}
"#;

    let output = compile_run(src);
    match output {
        Ok(stdout) => {
            let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
            assert_eq!(lines, vec![
                "1", // all 50 random_range(5,10) within [5,10)
                "3", // "banana" x3
                "9", // count of "a"
                "0", // empty sub
                "0", // no "zzz"
                "1", // "aaa".count("aa") -> non-overlapping
            ]);
        }
        Err(e) => panic!("{e}"),
    }
}