# York Programming Language — Complete Technical Specification & Reference Manual

<div align="center">

[![GitHub Release](https://img.shields.io/github/v/release/TheRealClyp/York?include_prereleases&style=flat-square&color=blue)](https://github.com/TheRealClyp/York/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet.svg?style=flat-square)]()

**The definitive, exhaustive reference manual and technical specification for the York programming language (v0.2.3).**

</div>

---

## Table of Contents

1. [Introduction & Architectural Philosophy](#1-introduction--architectural-philosophy)
2. [Compiler Pipeline & Code Generation](#2-compiler-pipeline--code-generation)
3. [Lexical Structure & Grammar Specification](#3-lexical-structure--grammar-specification)
4. [Type System & Inference Engine](#4-type-system--inference-engine)
5. [Memory Management & Arena Allocators (`Arena<T>`)](#5-memory-management--arena-allocators-arenat)
6. [Control Flow & Branching Constructs](#6-control-flow--branching-constructs)
7. [Structs, Methods (`impl`), and Enums](#7-structs-methods-impl-and-enums)
8. [Standard Library & Built-in Functions Reference](#8-standard-library--built-in-functions-reference)
9. [String Manipulation & Unicode Methods](#9-string-manipulation--unicode-methods)
10. [File System, I/O, and Operating System Integration](#10-file-system-io-and-operating-system-integration)
11. [Native Win32 Desktop GUI Subsystem](#11-native-win32-desktop-gui-subsystem)
12. [CLI Toolchain & Project Configuration (`york.york`, `resource.ypg`)](#12-cli-toolchain--project-configuration-yorkyork-resourceypg)
13. [Cross-Compilation & Multi-Platform Packaging](#13-cross-compilation--multi-platform-packaging)
14. [Exhaustive Code Examples & Recipes](#14-exhaustive-code-examples--recipes)

---

## 1. Introduction & Architectural Philosophy

York is a high-performance, statically-typed systems programming language engineered to eliminate runtime bloat while providing modern ergonomics. Unlike interpreted languages or VM-based runtimes (Node.js, Java, Python), York compiles directly to highly optimized C11 code. This C code is subsequently fed into native system compilers (GCC, Clang, TCC, Zig, or MSVC `cl.exe`), producing standalone machine-code executables.

### Core Tenets
- **Zero Runtime Overhead**: No garbage collection pauses, no virtual machines, and no mandatory heavyweight standard library runtime.
- **Direct Native Integration**: First-class access to system APIs, including raw Win32 desktop window handles and GUI control management without browser wrappers.
- **Predictable Performance**: Contiguous memory arenas (`Arena<T>`) guarantee zero-fragmentation allocation patterns ideal for real-time simulations, games, and high-throughput servers.

---

## 2. Compiler Pipeline & Code Generation

The York compilation pipeline consists of six distinct, deterministic phases managed by the `york_cli` driver:

1. **Lexical Analysis (`york_lexer`)**: Converts raw source strings into a vector of typed tokens, tracking line and column spans for error reporting.
2. **Syntactic Analysis (`york_parser`)**: Employs a recursive descent parser to build an Abstract Syntax Tree (AST), supporting Java/C#-style declarations, visibility modifiers, generics, and statement blocks.
3. **Semantic Analysis (`york_sema`)**: Performs type checking, scope resolution, symbol table construction, function call lowering, and built-in function mapping.
4. **Intermediate Representation (`york_ir`)**: Transforms the verified AST into high-level intermediate representations for optimization passes.
5. **C Code Generation (`york_codegen_c`)**: Emits pristine C11 code complete with custom memory arena helpers, string routines, platform environment wrappers, and Win32 bindings.
6. **Native Compilation (`compile_c`)**: Automatically detects and invokes available system toolchains (`clang`, `gcc`, `cc`, `tcc`, `zig`, or `cl.exe`), linking against math (`-lm`) and Windows GUI libraries (`user32.lib`, `gdi32.lib`, `shell32.lib`) automatically.

---

## 3. Lexical Structure & Grammar Specification

### Keywords
```
pub      static   fn       struct   impl     enum     switch   case
default  if       else     while    for      in       break    continue
return   true     false    null     void     int      float    bool
char     String   import   use      new      this
```

### Literals & Identifiers
- **Identifiers**: `[a-zA-Z_][a-zA-Z0-9_]*`
- **Integer Literals**: Decimal integers (`42`, `1000`) mapped to 64-bit signed C `long long`.
- **Float Literals**: Floating-point numbers (`3.14159`, `0.0`) mapped to C `double`.
- **String Literals**: Double-quoted UTF-8 sequences (`"Hello York"`) heap-allocated with null termination.
- **Character Literals**: Single-quoted characters (`'a'`).

---

## 4. Type System & Inference Engine

York is strictly typed at compile time with local type inference for variable bindings.

### Primitive Types
| York Type | C Equiv. | Size / Description |
| :--- | :--- | :--- |
| `int` | `long long` | 64-bit signed integer |
| `float` | `double` | 64-bit double-precision float |
| `bool` | `bool` | Boolean (`true` / `false`) |
| `char` | `char` | Single character |
| `String` | `const char*` | Heap string reference |
| `void` | `void` | Absence of value |

---

## 5. Memory Management & Arena Allocators (`Arena<T>`)

York provides built-in `Arena<T>` types for high-performance memory management without manual `malloc`/`free` per item.

### Arena Operations
- `Arena<T> a = new Arena(capacity);` — Allocates initial buffer.
- `a.push(item);` — Appends element, auto-resizing capacity when full.
- `a.get(index);` — Fetches element at index.
- `a.set(index, item);` — Overwrites element at index.
- `a.pop();` — Removes and returns last element.
- `a.last();` — Peeks at last element.
- `a.clear();` — Resets element count to 0 instantly without reallocating buffer memory.
- `a.count();` — Returns current item count (`size_t`).
- `a.is_empty();` — Returns boolean check.

---

## 6. Control Flow & Branching Constructs

### Conditionals
```java
if (score > 90) {
    println("A");
} else if (score > 80) {
    println("B");
} else {
    println("C");
}
```

### Loops
```java
int i = 0;
while (i < 5) {
    i = i + 1;
}

for (int j = 0; j < 10; j++) {
    if (j == 5) { continue; }
    if (j == 8) { break; }
}
```

### Switch Branching
```java
int code = 2;
switch (code) {
    case 1:
        println("One");
        break;
    case 2:
        println("Two");
        break;
    default:
        println("Other");
        break;
}
```

---

## 7. Structs, Methods (`impl`), and Enums

```java
struct Vector2 {
    float x;
    float y;
}

impl Vector2 {
    float length() {
        return sqrt(this.x * this.x + this.y * this.y);
    }
}

enum Status {
    Pending,
    Active,
    Completed
}
```

---

## 8. Standard Library & Built-in Functions Reference

### Mathematical Functions
- `min(a, b)` / `max(a, b)` — Returns minimum or maximum of two ints or floats.
- `clamp(v, lo, hi)` — Restricts value within bounds.
- `abs(v)` — Absolute value (`fabs` for float, `llabs` for int).
- `sqrt(v)`, `pow(b, e)`, `floor(v)`, `ceil(v)`, `round(v)` — Standard math operations.
- `sin(v)`, `cos(v)`, `tan(v)`, `ln(v)`, `log10(v)`, `exp(v)` — Trigonometric and logarithmic evaluations.
- `srand(seed)`, `rand()`, `random_range(lo, hi)` — Pseudo-random number generation.

### System & Time Functions
- `now()` — Epoch timestamp in milliseconds.
- `sleep(secs)`, `sleep_ms(ms)` — Execution pause.
- `env(name)` — Retrieves environment variable (returns empty string if unset).
- `platform_name()` — Returns `"windows"`, `"linux"`, `"macos"`, `"freebsd"`, or `"unknown"`.
- `exec(cmd)` — Executes shell command and returns exit code.
- `exit(code)` — Terminates program execution.
- `assert(cond, msg)` — Evaluates condition and aborts if false.

---

## 9. String Manipulation & Unicode Methods

Every York `String` instance supports chainable methods:
- `.len()`, `.length()`, `.size()` — Returns string length.
- `.contains(sub)` — Substring search (`bool`).
- `.startsWith(pre)`, `.endsWith(suf)` — Prefix and suffix checks.
- `.indexOf(sub)`, `.lastIndexOf(sub)` — Index location (`int`).
- `.count(sub)` — Non-overlapping occurrence count.
- `.toUpper()`, `.toLower()` — Case conversion.
- `.trim()`, `.trimStart()`, `.trimEnd()` — Whitespace stripping.
- `.substring(start, end)` — Range slicing.
- `.padLeft(width, pad)`, `.padRight(width, pad)` — String alignment and padding.
- `.repeat(n)` — String multiplication.
- `.replace(from, to)` — Substring replacement.
- `.reverse()` — Character inversion.
- `to_string(v)`, `to_int(s)`, `to_float(s)` — Type conversions.

---

## 10. File System, I/O, and Operating System Integration

- `print(...)`, `println(...)` — Formatted standard output.
- `read_line()`, `read_int()` — Standard input reading.
- `file_exists(path)` — Checks path existence (`bool`).
- `read_file(path)` — Reads entire file into a heap string.
- `write_file(path, content)` — Writes string data to file (`bool`).
- `append_file(path, content)` — Appends string data to file (`bool`).

---

## 11. Native Win32 Desktop GUI Subsystem

York features a first-class native Windows GUI framework bridging directly to Win32 `user32` and `gdi32`:

### Window API
- `long win = window_create(title, width, height);`
- `window_show(win);`
- `window_hide(win);`
- `window_close(win);`
- `bool isOpen = window_is_open(win);`
- `window_poll_events(win);`
- `window_run_loop(win);`
- `long cmdId = window_last_command();`

### Native Controls
- `long btn = control_button(win, label, x, y, width, height, id);`
- `long lbl = control_label(win, text, x, y, width, height, id);`
- `long txt = control_textbox(win, text, x, y, width, height, id);`
- `control_set_text(controlHandle, newText);`

### System Dialogs
- `message_box(title, message);`

---

## 12. CLI Toolchain & Project Configuration (`resource.ypg`)

York uses `resource.ypg` for project manifests:
```toml
name = "my_york_app"
entry = "src/main.yk"
version = "0.2.3"
```

### CLI Commands
- `york run [file.yk]` — Auto-detects entry or `resource.ypg`, compiles, and runs.
- `york build [file.yk]` — Produces optimized standalone binary.
- `york check [file.yk]` — Static analysis and type checking.
- `york new [name]` — Scaffolds a new project.
- `york --credits` — Displays toolchain banner.

---

## 13. Cross-Compilation & Multi-Platform Packaging

York supports cross-compilation target bundles:
- **Windows x64**: Native `.exe`, `.zip`, and Inno Setup installer (`york-setup-x64.exe`).
- **Linux x86_64 / AArch64**: Musl static targets packaged in `.tar.gz` archives with SHA-256 verification sums (`SHASUMS256.txt`).

---

## 14. Exhaustive Code Examples & Recipes

### Recipe A: High-Performance Data Processing
```java
public static void main(String[] args) {
    Arena<float> readings = new Arena(100);
    readings.push(23.5);
    readings.push(24.1);
    readings.push(22.8);

    float sum = 0.0;
    for (int i = 0; i < readings.count(); i++) {
        sum = sum + readings.get(i);
    }
    println("Average Reading: " + to_string(sum / (float)readings.count()));
}
```

### Recipe B: Native Win32 Note Taking App
```java
public static void main(String[] args) {
    long win = window_create("York Notes", 800, 600);
    long txt = control_textbox(win, "Type notes here...", 20, 20, 740, 450, 101);
    long save = control_button(win, "Save", 20, 490, 100, 35, 102);
    long lbl = control_label(win, "Ready", 140, 495, 400, 30, 103);
    window_show(win);

    while (window_is_open(win)) {
        window_poll_events(win);
        long cmd = window_last_command();
        if (cmd == 102) {
            control_set_text(lbl, "Notes saved successfully!");
            message_box("York Notes", "Your note was persisted.");
        }
        sleep_ms(15);
    }
    window_close(win);
}
```
