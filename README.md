<div align="center">

  <img src="assets/logo.svg" width="160" height="160" alt="York Programming Language Logo" />

  # York Programming Language

  **A fast, friendly, zero-overhead systems programming language.**  
  *Native Binaries · Win32 Desktop GUI · High-Throughput Sockets · PWA · Signed Android APK · iOS Xcode*

  [![GitHub Release](https://img.shields.io/github/v/release/TheRealClyp/York?include_prereleases&style=flat-square&color=blue)](https://github.com/TheRealClyp/York/releases)
  [![License](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet.svg?style=flat-square)]()
  [![Security](https://img.shields.io/badge/security-100%25%20Verified%20Safe-brightgreen.svg?style=flat-square)](SECURITY.md)
  [![Mobile](https://img.shields.io/badge/mobile-PWA%20%7C%20Android%20%7C%20iOS-cyan.svg?style=flat-square)]()

  **The definitive reference manual and technical specification for the York programming language (v1.1.0).**

</div>

York is a high-performance, statically-typed systems language engineered to eliminate runtime bloat while keeping modern ergonomics. From one source program you get a native desktop binary, a full native Win32 GUI app, an installable web app **and** a cross-platform mobile app — a real signed Android `.apk` and a complete iOS Xcode project — built with York's own toolchain, no Gradle, no Android Studio, no full SDK.

---

## Table of Contents

- [Quickstart & Installation](#quickstart--installation)
- [Security, Trust & False-Positive Notice ("We Are NOT Hacking")](#security-trust--false-positive-notice-we-are-not-hacking)
- [Uninstallation Guide](#uninstallation-guide)
1. [Introduction & Architectural Philosophy](#1-introduction--architectural-philosophy)
2. [Compiler Pipeline & Code Generation](#2-compiler-pipeline--code-generation)
3. [Lexical Structure & Grammar Specification](#3-lexical-structure--grammar-specification)
4. [Type System & Inference Engine](#4-type-system--inference-engine)
5. [Memory Management & Arena Allocators (`Arena<T>`)](#5-memory-management--arena-allocators-arenat)
6. [Control Flow & Branching Constructs](#6-control-flow--branching-constructs)
7. [Structs, Methods (`impl`), and Enums](#7-structs-methods-impl-and-enums)
8. [Built-in Functions Reference (Complete)](#8-built-in-functions-reference-complete)
9. [String Manipulation & Unicode Methods](#9-string-manipulation--unicode-methods)
10. [File System, I/O, and Operating System Integration](#10-file-system-io-and-operating-system-integration)
11. [Networking & Raw Sockets](#11-networking--raw-sockets)
12. [Threading & Concurrency](#12-threading--concurrency)
13. [Native Win32 Desktop GUI Subsystem](#13-native-win32-desktop-gui-subsystem)
14. [CLI Toolchain & Project Workflow](#14-cli-toolchain--project-workflow)
15. [Mobile Apps: PWA, Android APK & iOS (`york mobile`)](#15-mobile-apps-pwa-android-apk--ios-york-mobile)
16. [Cross-Compilation & Multi-Platform Packaging](#16-cross-compilation--multi-platform-packaging)
17. [Limits & Language Capabilities](#17-limits--language-capabilities)
18. [Exhaustive Code Examples & Recipes](#18-exhaustive-code-examples--recipes)

---

## Quickstart & Installation

Install the York toolchain on your system in seconds:

### Windows (Recommended)
**Option 1: Windows GUI Installer**
Download and run **[york-setup-x64.exe](https://github.com/TheRealClyp/York/releases/latest)**.

**Option 2: PowerShell One-Liner**
Open PowerShell and run:
```powershell
irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.ps1 | iex
```

### macOS & Linux
Open your terminal and run:
```bash
curl -fsSL https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.sh | sh
```

### From Source (Any Platform with Rust)
```bash
git clone https://github.com/TheRealClyp/York.git
cd York
cargo install --path tools/york_cli
```

Verify your installation:
```bash
york --version
york doctor
```

---

## Security, Trust & False-Positive Notice ("We Are NOT Hacking")

> **Important:** York is **100% clean, verified open-source software (MIT License)**. It is **NOT** malware, hacking software, a trojan, or spyware. See our official [SECURITY.md](SECURITY.md) for full verification details.

### Why does Windows Defender or SmartScreen show a warning?
When you download new or freshly compiled open-source tools on Windows, Microsoft Defender SmartScreen may display:
`"Windows protected your PC: Microsoft Defender SmartScreen prevented an unrecognized app from starting."`
Or an antivirus scanner may flag unsigned developer tools as generic riskware.

**The Explanation:**
1. **Lack of Expensive EV Certificates**: Commercial code-signing certificates with hardware tokens cost $400–$600+ per year. As an open-source project, binaries start without enterprise telemetry reputation. Microsoft's cloud defaults to showing an "unrecognized" warning for any unsigned executable until hundreds of thousands of users run it.
2. **Native Systems Compiler Behavior**: York compiles C code, invokes system linkers, binds network sockets (`ws2_32.lib`), and creates native executables. Generic heuristic algorithms often mistake these legitimate developer capabilities for suspicious activity.
3. **Mark-of-the-Web**: Windows tags internet downloads with `Zone.Identifier`. Our installer automatically unblocks files (`Unblock-File`) to prevent false alarms.

### How to run safely without warnings:
- **SmartScreen Popup**: Click **"More info"** → click **"Run anyway"**.
- **PowerShell**: Run `Unblock-File .\york.exe` or `Unblock-File .\york-setup-x64.exe`.
- **Verify Checksums**: Check SHA-256 hashes against `SHASUMS256.txt`.
- **Inspect Source**: All source code is completely public and auditable in this repository.

---

## Uninstallation Guide

We believe software should be as easy to remove as it is to install.

### Windows
- **Via York CLI**: Run `york uninstall` in your terminal.
- **Via PowerShell One-Liner**:
  ```powershell
  irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/uninstall.ps1 | iex
  ```
- **Via Windows Settings**: If installed via the Inno Setup installer, navigate to **Settings → Apps → Installed apps**, locate **York 1.1.0**, and click **Uninstall**.

### macOS & Linux
- **Via York CLI**: Run `york uninstall`.
- **Via Shell One-Liner**:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/TheRealClyp/York/main/installers/uninstall.sh | sh
  ```

---

## 1. Introduction & Architectural Philosophy

York compiles directly to optimized C11 code with **zero runtime overhead**: no garbage-collection pauses, no virtual machines, no heavyweight standard-library runtime. The generated C is fed into a native system compiler (Clang, GCC, TCC, Zig, or MSVC) producing standalone machine-code executables — or, with `york mobile`, packaged into mobile apps with York's own native bridge layer.

### Core Tenets
- **One source, every target.** The same `.yk` file -> a desktop binary, a Win32 GUI app, a server, an installable PWA, an Android `.apk`, and an iOS project.
- **Zero Runtime Overhead.** No VM, no GC, no interpreter.
- **Direct Native Integration.** First-class access to system APIs — Win32 windows and controls, BSD/Winsock sockets, real OS threads.
- **Predictable Performance.** Contiguous memory arenas (`Arena<T>`) give zero-fragmentation allocation for real-time simulations, games, and high-throughput servers.
- **Owned mobile toolchain.** `york mobile --apk` drives `aapt2 → javac → d8 → zipalign → apksigner` itself — the only dependency is a one-time ~120 MB fetch of Google's command-line compilers into `~/.york/android`.

### What York Does NOT Have (And What It Uses Instead)

1. **No Garbage Collector (GC)**
   - **What's missing**: There is no automatic background garbage collection to clean up memory (unlike Go, Java, or C#).
   - **The alternative**: It relies entirely on contiguous memory arenas (`Arena<T>`) that you must clear manually when you are done with them.

2. **No Heavy Runtime or Virtual Machine**
   - **What's missing**: It does not use a virtual machine (like JVM or .NET) or a heavyweight interpreter layer.
   - **The alternative**: It compiles directly into clean C11 source code, which is then bundled into a standalone native machine executable.

3. **No Browser-Based Desktop Wrappers**
   - **What's missing**: Its desktop GUI framework completely bypasses modern web-tech wrappers like Electron, Chromium, or Node.js (which often make simple apps consume hundreds of megabytes of RAM).
   - **The alternative**: It links straight to native, lightweight Win32 system libraries (`user32.lib` and `gdi32.lib`).

4. **Missing Common Modern Data Structures (Out of the Box)**
   - **What's missing**: The specification does not list standard language built-ins like dynamic HashMaps/Dictionaries, HashSets, or linked lists.
   - **The alternative**: Your primary data structure is the flat `Arena<T>`, meaning complex collections must be built manually using arrays, structs, and pointers.

5. **No Advanced Error Handling Syntax**
   - **What's missing**: There are no keywords or mechanisms for complex error handling like `try`/`catch` blocks or explicit `Result`/`Option` types.
   - **The alternative**: It utilizes a basic runtime `assert(cond, msg)` function to halt execution if a critical condition fails, shifting the burden of safety onto the developer.

---

## 2. Compiler Pipeline & Code Generation

Six distinct, deterministic phases managed by the `york_cli` driver:

1. **Lexical Analysis (`york_lexer`)** — Converts raw source into typed tokens with line/column spans.
2. **Syntactic Analysis (`york_parser`)** — Recursive-descent parser building an AST; supports Java/C#-style declarations, visibility modifiers, generic `Arena<T>`, and statement blocks.
3. **Semantic Analysis (`york_sema`)** — Type checking, scope resolution, symbol tables, call lowering, and built-in function mapping (`name → __york_*` C symbol).
4. **Intermediate Representation (`york_ir`)** — Typed IR: `Ty` (22 variants), `Stmt` (13 variants), `Expr` (25 variants including `Strcat`, `Streq`, `EnumRef`, `New`, `StructLiteral`, `MethodCall`).
5. **C Code Generation (`york_codegen_c`)** — Emits pristine C11 with arena helpers, string routines, platform wrappers, Win32 bindings, sockets, and pthread/`CreateThread` wrappers.
6. **Native Compilation (`compile_c`)** — Auto-detects the toolchain (`clang`, `gcc`, `cc`, `tcc`, `zig`, then MSVC `cl.exe`), linking `-lm` and, on Windows, `user32.lib gdi32.lib shell32.lib ws2_32.lib advapi32.lib`.

```
.yk source ─lex→ tokens ─parse→ AST ─sema→ typed IR ─codegen→ C11 ─CC→ native exe
                                                              └─ york mobile → PWA + Android + iOS
```

### The Custom `yc` Compiler Driver

Alongside the `york` CLI driver, York provides **`yc`**, a standalone GCC/Clang-compatible systems compiler driver.

```bash
yc main.yk -o myapp -O2 -v
```
- `-o <file>` — Output executable path.
- `-O0`, `-O1`, `-O2`, `-O3` — Optimization levels passed to backend compilers.
- `-c` — Compile only (emit temporary C11 source without linking).
- `-v` — Verbose pipeline reporting.

---

## 3. Lexical Structure & Grammar Specification

### Keywords
```
pub      static   fn       struct   impl     enum     switch   case
default  if       else     while    loop     for      in       break
continue return   true     false    null     void     new      this
let      var      import   packed   type     const
```

### Identifiers & Literals
- **Identifiers**: `[a-zA-Z_][a-zA-Z0-9_]*`
- **Integer Literals**: decimal (`42`, `1000`); sized per declared type, defaulting to 64-bit.
- **Float Literals**: `3.14159`, `0.0`; sized per declared type, defaulting to `float` (64-bit double).
- **String Literals**: double-quoted UTF-8, heap-allocated, null-terminated.
- **Character Literals**: single-quoted `'a'`.
- **Comments**: `// line` and `/* block */`.

---

## 4. Type System & Inference Engine

York is strictly typed at compile time with local type inference for variable bindings.

### Sized Primitive Types
| York type | Aliases | C equiv. | Size |
| :--- | :--- | :--- | :--- |
| `i8` | `byte` | `int8_t` | 8-bit signed |
| `i16` | `short` | `int16_t` | 16-bit signed |
| `i32` | `int` | `int32_t` | 32-bit signed |
| `i64` | `long` | `int64_t` | 64-bit signed |
| `i128` | — | `__int128` | 128-bit signed |
| `u8` | — | `uint8_t` | 8-bit unsigned |
| `u16` | — | `uint16_t` | 16-bit unsigned |
| `u32` | — | `uint32_t` | 32-bit unsigned |
| `u64` | — | `uint64_t` | 64-bit unsigned |
| `u128` | — | `unsigned __int128` | 128-bit unsigned |
| `f16` | — | promoted to `f32` | (stored 32-bit) |
| `f32` | `float` | `float` | 32-bit float |
| `f64` | `double` | `double` | 64-bit float |
| `bool` | `boolean` | `bool` | Boolean |
| `char` | — | `char` | Single character |
| `String` | `string` | `const char*` | Heap string |
| `void` | — | `void` | Absence of value |
| `!` | — | (never) | Bottom type |
| `_` | — | (inferred) | Inferred type |

### Composite & Declared Types
- **`struct`** — `struct Name { T field; ... }`; optional `packed` modifier. Forward-declared for recursive references and `Arena<T>` fields.
- **`enum`** — `enum Name { Variant, ... }` unit variants compiled to C enums; referenced as `Name.Variant`.
- **`impl`** — `impl Type { ... }` with implicit `self` / `this`; methods mangle to `Type_method`.
- **`Arena<T>`** — generic contiguous buffer (§5). Also accepts `arena(T)` annotation style.
- **Slices `T[]`** — lowered to `T*`; supports `for (x : slice)` and `slice[i]`.
- **Arrays / array literals** — *not yet supported* (§17).

### Statements
`let` / `var` bindings (optional type annotation) · expression statements · assignment and compound assignment (`=`, `+=`, `-=`, `*=`, `/=`, `%=`) · `return` · `if / else if / else` · `while` · `loop` (desugars to `while (true)`) · C-style `for` · `for (x : iterable)` foreach · `switch`/`case`/`default` (no fallthrough — implicit `break` after every arm) · `break` / `continue` · block expressions.

### Expressions
Int / float / string / char / bool / `null` literals · identifiers · string `+` concatenation · string `==` / `!=` comparison (via `strcmp`) · binary & unary operators · `++` / `--` (prefix and postfix) · ternary `cond ? a : b` · indexing `a[i]` · struct literal `Name { f: v }` · `new Name(args)` · if-as-expression (require both branches to be single expressions) · block expressions.

---

## 5. Memory Management & Arena Allocators (`Arena<T>`)

York ships built-in `Arena<T>` buffers for high-performance memory management without per-item `malloc`/`free`. Elements live in a contiguous, doubling buffer (initial capacity 16, `realloc`-grown), giving cache-friendly, zero-fragmentation allocation.

### Arena Operations
| Method | Semantics |
| :--- | :--- |
| `Arena<T> a = new Arena(cap);` | Allocates the initial buffer |
| `a.push(item)` | Appends an item, auto-growing capacity when full |
| `a.get(index)` | Reads the element at index |
| `a.set(index, item)` | Overwrites the element at index (bounds-guarded) |
| `a.pop()` | Removes and returns the last element |
| `a.last()` | Peeks at the last element without popping |
| `a.clear()` | Resets the element count to 0 instantly (keeps the buffer) |
| `a.count()` | Current element count (also `.len()` / `.length()`) |
| `a.is_empty()` | Returns whether the arena is empty (also `.isEmpty()`) |

`T` may be a struct, enum, or primitive. `for (T x : arena)` iterates (the iterable is snapshotted once to avoid double side-effects), and `arena[i]` indexing works too.

---

## 6. Control Flow & Branching Constructs

```java
if (score > 90) {
    println("A");
} else if (score > 80) {
    println("B");
} else {
    println("C");
}

int i = 0;
while (i < 5) { i = i + 1; }

for (int j = 0; j < 10; j++) {        // C-style
    if (j == 8) break;
    if (j == 5) continue;
}

loop { if (done) break; }             // infinite loop

switch (code) {                       // no fallthrough — break is implicit
    case 1: println("One");
    case 2: println("Two");
    default: println("Other");
}

for (String name : names) {           // foreach over Arena/slice
    println(name);
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

struct Player {                       // structs may hold arenas
    String name;
    Arena<int> scores;
}

enum Status {
    Pending,
    Active,
    Completed
}

public static void main(String[] args) {
    Vector2 v = new Vector2;          // `new` constructor
    v.x = 3.0; v.y = 4.0;
    println(to_string(v.length()));   // 5
    if (Status.Active == Status.Active) { /* enum reference via .Variant */ }
}
```

---

## 8. Built-in Functions Reference (Complete)

Every feature below is implemented in the compiler and emitted to real C. The list is grouped for reference.

### Core I/O & Console
| Function | Behavior |
| :--- | :--- |
| `print(...)` | Prints values (multi-arg values are stringified and concatenated) |
| `println(...)` | Prints with a newline |
| `read_int()` | Reads an integer from stdin |
| `rand()` / `srand(seed)` | C stdlib pseudo-random |
| `random_range(lo, hi)` | Random int in `[lo, hi)` |
| `random_float()` | Random float in `[0.0, 1.0]` |

### Math
| Function | Behavior |
| :--- | :--- |
| `min(a, b)` / `max(a, b)` | Int/float overloads chosen by inference |
| `clamp(v, lo, hi)` | Restrict value to `[lo, hi]` |
| `abs(v)` | `llabs` (int) or `fabs` (float) |
| `sqrt(v)`, `pow(b, e)`, `floor(v)`, `ceil(v)`, `round(v)` | Standard math |
| `sin(v)`, `cos(v)`, `tan(v)`, `ln(v)` (`log`), `log2(v)`, `log10(v)`, `exp(v)` | Trig / logarithmic |
| `fract(v)` | Fractional part (`modf`) |
| `math_pi()`, `math_e()` | Constants |
| `math_sign(v)` | Sign of value |
| `math_is_even(v)`, `math_is_odd(v)`, `math_is_prime(v)` | Number predicates |
| `math_gcd(a, b)`, `math_lcm(a, b)` | Greatest common divisor / least common multiple |
| `math_lerp(a, b, t)` | Linear interpolation |
| `degrees_to_radians(d)`, `radians_to_degrees(r)` | Angle conversion |

### Strings (free functions)
| Function | Behavior |
| :--- | :--- |
| `str_len(s)` / `strlen(s)` | String length |
| `to_string(v)` | Int/float/bool → string (identity for `String`) |
| `to_int(s)` | String → int |
| `to_float(s)` | String → float |
| `str_slugify(s)` · `str_capitalize(s)` | Slug / title-case transforms |
| `str_repeat(s, n)` | Repeat a string |
| `str_word_count(s)` | Number of words |
| `str_trim_left(s)` / `str_trim_right(s)` | Trim one edge |
| `str_is_alpha(s)`, `str_is_digit(s)`, `str_is_numeric(s)`, `str_is_alnum(s)`, `str_is_lower(s)`, `str_is_upper(s)` | Character-class checks |
| `str_levenshtein(a, b)` | Edit-distance |
| `str_first(s)`, `str_last(s)` | First / last character |
| `str_rev_words(s)` | Word-order reversal |

### System & OS
| Function | Behavior |
| :--- | :--- |
| `env(name)` | Environment variable (empty string if unset) |
| `platform_name()` | `"windows"`, `"macos"`, `"linux"`, `"freebsd"`, or `"unknown"` |
| `os_cpu_count()` | Logical CPU count |
| `os_total_memory()` | Total system memory |
| `os_pid()` | Current process id |
| `sys_username()` · `sys_hostname()` | User / machine identity |
| `sys_time_str()` | Local time `"YYYY-MM-DD HH:MM:SS"` |
| `now()` / `epoch_ms()` | Monotonic milliseconds |
| `sleep(ms)` / `sleep_ms(ms)` / `delay(ms)` | Pause execution |
| `exec(cmd)` / `system_cmd(cmd)` | Run a shell command, returns exit code |
| `exit(code)` | Terminate the program |

### Networking (raw BSD/Winsock)
| Function | Behavior |
| :--- | :--- |
| `net_listen(port)` | Open a listening socket, returns handle |
| `net_accept(server)` | Accept an incoming connection |
| `net_connect(host, port)` | Connect out to a host/port |
| `net_send(sock, data)` | Send data on a socket |
| `net_recv(sock)` | Receive (4096-byte buffer) |
| `net_close(sock)` | Close a socket |

### Threading
| Function | Behavior |
| :--- | :--- |
| `thread_spawn("fnName", arg)` | Start a real OS thread (pthread / `CreateThread`) dispatching by function name |
| `thread_join(id)` | Wait for a thread to finish |
| `thread_self()` | Current thread id |

### Binary & Hashing
| Function | Behavior |
| :--- | :--- |
| `bin_pack(path, data)` | Length-prefixed binary write |
| `crypto_hash(input)` | Fast hash (hex string) |

---

## 9. String Manipulation & Unicode Methods

Every York `String` supports chainable methods (each returns a `String`, `int`, or `bool`, so calls compose). Unknown methods produce a compile-time `NoMethod` error.

### Length & Query
- `.len()` / `.length()` / `.size()` — length
- `.isEmpty()` / `.is_empty()` — emptiness check
- `.contains(sub)` — substring search
- `.startsWith(pre)` / `.starts_with()` — prefix check
- `.endsWith(suf)` / `.ends_with()` — suffix check
- `.indexOf(sub)` / `.index_of()` / `.find()` — first index (`-1` if absent)
- `.lastIndexOf(sub)` / `.last_index_of()` / `.rfind()` — last index
- `.charAt(i)` / `.char_at()` — character at index (as a `String`)
- `.count(sub)` — non-overlapping occurrence count

### Transform
- `.toUpper()` / `.uppercase()` / `.upper()` — uppercase
- `.toLower()` / `.lowercase()` / `.lower()` — lowercase
- `.trim()` — strip both ends
- `.trimStart()` / `.trim_start()` — strip left
- `.trimEnd()` / `.trim_end()` — strip right
- `.substring(start, end)` / `.substr()` — range slice
- `.padLeft(width, pad)` / `.pad_left()` / `.ljust()` — left pad
- `.padRight(width, pad)` / `.pad_right()` / `.rjust()` — right pad
- `.repeat(n)` — repetition
- `.replace(from, to)` — replace all occurrences
- `.reverse()` — reverse the string

### Example
```java
String s = "  hello world  ";
String clean = s.trim().toUpper().substring(0, 5);   // "HELLO"
println(clean);                                       // HELLO
if (s.contains("world")) { println("found"); }
```

---

## 10. File System, I/O, and Operating System Integration

### File I/O
| Function | Behavior |
| :--- | :--- |
| `write_file(path, content)` | Writes string data to file |
| `append_file(path, content)` | Appends string data to file |
| `read_file(path)` | Reads an entire file into a heap string |
| `file_exists(path)` | Path existence check |
| `fs_file_size(path)` | Byte size |
| `fs_delete_file(path)` | Remove a file |
| `fs_copy_file(src, dst)` | Copy a file |
| `fopen` / `fclose` / `fprintf` / `fputs` | Direct C stdio passthrough |

### OS & Environment
As listed in §8 (System & OS): `env`, `platform_name`, `os_cpu_count`, `os_total_memory`, `os_pid`, `sys_username`, `sys_hostname`, `sys_time_str`, `now`/`epoch_ms`, `sleep`/`sleep_ms`/`delay`, `exec`/`system_cmd`, `exit`.

---

## 11. Networking & Raw Sockets

York exposes direct BSD/Winsock sockets so programs can speak TCP without a framework:

```java
public static void main(String[] args) {
    long srv = net_listen(8080);
    long client = net_accept(srv);
    net_send(client, "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
    net_close(client);
    net_close(srv);
}
```

`net_connect(host, port)` opens outbound connections; `net_recv` reads up to 4096 bytes at a time.

---

## 12. Threading & Concurrency

York can spawn real OS threads (pthread on Linux/macOS, `CreateThread` on Windows) dispatching by **function name**:

```java
public static void worker(int arg) {
    println("worker " + to_string(arg));
}

public static void main(String[] args) {
    long t = thread_spawn("worker", 42);
    thread_join(t);
}
```

The dispatcher wraps the target function in an OS thread entry point, so programs can parallelize CPU-bound work directly.

---

## 13. Native Win32 Desktop GUI Subsystem

York features a first-class native Windows GUI framework bridging directly to Win32 `user32` and `gdi32`. On non-Windows targets the GUI functions compile to no-op stubs, so the **same source compiles on Linux/macOS** (GUI calls simply do nothing there).

### Window API
| Function | Behavior |
| :--- | :--- |
| `window_create(title, w, h)` | Creates + shows a centered window; returns a handle |
| `window_show(win)` | Shows / updates the window |
| `window_hide(win)` | Hides the window |
| `window_close(win)` | Destroys the window |
| `window_is_open(win)` | Window liveness check |
| `window_poll_events(win)` | Non-blocking message pump (`PeekMessageA`) |
| `window_run_loop(win)` | Blocking message loop (`GetMessageA`) |
| `window_last_command()` | Returns (and resets) the last control `WM_COMMAND` id |

### Native Controls
| Function | Behavior |
| :--- | :--- |
| `control_button(win, label, x, y, w, h, id)` | Push button |
| `control_label(win, text, x, y, w, h, id)` | Static text |
| `control_textbox(win, text, x, y, w, h, id)` | Edit box |
| `control_set_text(ctrl, text)` | Update any control's text |

### System Dialogs
- `message_box(title, message)` — Native `MessageBoxA(MB_OK)`.

### Example: Note Taking App
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

---

## 14. CLI Toolchain & Project Workflow

### Entry Auto-Detection (npm-style)
`york run` (and `build`/`check`/`mobile`) auto-detect the entry file in this order:
explicit path → `resource.ypg` manifest (`entry`/`main`) → `main.yk` → `src/main.yk` → `src/index.yk` → the single `*.yk` in cwd → an interactive picker.

### CLI Commands
| Command | Description |
| :--- | :--- |
| `york run [file] [args...]` | Compile and run in one step. `york run dev` is a shorthand for watch mode |
| `york dev [file]` | **Watch mode** — rebuild and re-run automatically on every save (400 ms polling, debounced) |
| `york build [file] --out <name> --cc <compiler>` | Compile to a standalone native binary (`-O2 -std=c11 -lm`) |
| `york check [file]` | Lex + parse + type-check + codegen without invoking the C compiler |
| `york new [name]` | Scaffold a desktop project (`src/main.yk`) |
| `york new-mobile [name]` | Scaffold a mobile app project |
| `york mobile <app.yk> ...` | Build PWA / Android / iOS from a York program (§15) |
| `york doctor` | Inspect environment, C compiler health, PATH status, and toolchain readiness |
| `york uninstall [-y]` | Safely remove York from the local system |
| `york --version` / `york -c, --credits` | Version badge / ASCII banner |

### C Compiler Auto-Detection
`compile_c` tries, in order: `clang`, `gcc`, `cc`, `tcc`, `zig` (as `zig cc`), then MSVC via an auto-located `vcvars64.bat` under Visual Studio 2022. `--cc` overrides the choice explicitly.

Example:
```bash
york new hello
cd hello
york run main.yk      # compile -> Hello, York!
york dev              # keep editing; it rebuilds on save
york build --out hello-exe
```

---

## 15. Mobile Apps: PWA, Android APK & iOS (`york mobile`)

York ships a complete, **self-owned mobile toolchain**. A single York program that writes `index.html` / `app.js` becomes four things with one command:

- an **installable PWA** (manifest + service worker + icons) for any phone's home screen,
- an **Android project** you can open in Android Studio,
- a **real, signed `.apk`** compiled by York alone — no Gradle, no Android Studio, no full SDK (first build fetches Google's command-line compilers once, ~120 MB, into `~/.york/android`, then York drives `aapt2 → javac → d8 → zipalign → apksigner` itself), and
- a complete **iOS Xcode project** (WKWebView shell + Swift bridge) to open on a Mac.

```bash
york new-mobile myapp
cd myapp
york mobile src/app.yk --out build --name "My App" --platform both --apk
```

The program writes the UI with `write_file` / `append_file` (see the scaffolded template), then `york mobile`:
1. compiles and runs the York program with the output directory as cwd,
2. injects the `York.*` runtime (`york-mobile.js`) before `app.js`,
3. writes the PWA manifest, service worker, and icons,
4. generates the Android shell (`android/`) and/or iOS Xcode project (`ios/`),
5. with `--apk`, builds and signs `build/apk/app-debug.apk`.

### `--platform` and `--apk`
| Flag | Effect |
| :--- | :--- |
| `--platform android` | Generate only the Android shell |
| `--platform ios` (alias `apple`) | Generate only the iOS Xcode project |
| `--platform both` (alias `all`, default) | Generate both shells |
| `--apk` | Also compile a real, signed `.apk` with York's own toolchain |
| `--icon <file.svg>` | Custom web app icon |
| `--serve` | Serve the web app on `0.0.0.0:8787` for on-phone preview |

### The `York.*` Device Runtime — Core 12

Every mobile app automatically gets `window.York`, exposing the Core 12 device features through three interchangeable transports: the Android `YorkBridge` (Java), the iOS `WKScriptMessageHandler` (Swift), or pure-web fallbacks (localStorage, `navigator.*`, etc.). Same API everywhere — one app codebase, three platforms.

| # | Feature | API |
| :--- | :--- | :--- |
| 1 | Key/value storage | `York.storage.get/set/remove/clear` |
| 2 | Toast notifications | `York.toast(message)` |
| 3 | Alert / confirm dialogs | `York.dialog.alert(msg)`, `York.dialog.confirm(msg)` |
| 4 | Vibration | `York.vibrate(ms)` |
| 5 | Geolocation | `York.geolocation.get()` → `{latitude, longitude, accuracy, altitude, speed, heading}` |
| 6 | Accelerometer | `York.sensors.startAccelerometer(cb)` / `York.on('accel', cb)` / `York.sensors.stopAccelerometer()` |
| 7 | Battery | `York.battery.get()` → `{level, charging}` |
| 8 | Network | `York.network.get()` → `{online, type, downlink}` |
| 9 | Clipboard | `York.clipboard.read()`, `York.clipboard.write(text)` |
| 10 | Share sheet | `York.share(title, text, url)` |
| 11 | Open URL | `York.openUrl(url, external?)` |
| 12 | Camera | `York.camera.back()`, `York.camera.front()`, `York.camera.pick()` → `{name, type, dataUrl, width, height}` |

Helpers: `York.ready(cb)`, `York.on(event, fn)` (e.g. `"accel"`, `"network"`), and `York.platform` (`"android" | "ios" | "web"`). Calls are async (JSON-RPC with a 15 s timeout); results arrive via `York._resolve`, events via `York._emit`.

```java
// inside your York program, generating app.js
js("var taps = 0;");
js("York.ready(function(){");
js("  York.storage.get('taps').then(function(v){ taps = Number(v) || 0; draw(taps); });");
js("  function tap(){ taps++; York.vibrate(20); York.storage.set('taps', taps); }");
js("});");
```

### Native Coverage
- **Android** (`YorkBridge.java` + `YorkCallable.java`, lambda-free, compiled with `javac --release 11` + D8 9.0.3): SharedPreferences storage, toast, `AlertDialog`, `Vibrator`, one-shot geolocation with runtime permission, battery via `ACTION_BATTERY_CHANGED`, network via `NetworkCapabilities` (with API-29 + legacy fallback), clipboard, `ACTION_SEND` share chooser, `ACTION_VIEW` openUrl, settings panel, accelerometer via `SensorManager`.
- **iOS** (`YorkBridge.swift`): `UserDefaults` storage, `UIAlertController` toast/alert/confirm, `UIImpactFeedbackGenerator`, `CLLocationManager` one-shot geolocation (8 s timeout), `UIDevice` battery, `NWPathMonitor` network, `UIPasteboard`, `UIActivityViewController` share, `UIApplication.open` URL, `CMMotionManager` accelerometer, settings.
- **Web / PWA**: `localStorage`, toast element, `window.alert`/`confirm`, `navigator.vibrate`, `navigator.geolocation`, `devicemotion` (with the iOS `requestPermission()` flow), `navigator.getBattery`, `navigator.connection`, `navigator.clipboard`, `navigator.share` (clipboard-copy fallback), `<input type=file accept=image/* capture>` for camera.

Manifest permissions generated for Android: `INTERNET`, `ACCESS_NETWORK_STATE`, `VIBRATE`, `ACCESS_FINE_LOCATION`, `ACCESS_COARSE_LOCATION`. iOS `Info.plist` includes location, motion, and camera usage strings.

---

## 16. Cross-Compilation & Multi-Platform Packaging

York ships prebuilt toolchain bundles and cross-builds through WSL:

- **Windows x64**: native `.exe`, `.zip`, and an Inno Setup installer (`york-setup-x64.exe`).
- **Linux x86_64 / AArch64**: fully static musl builds in `.tar.gz` archives.
- Each release publishes **SHA-256 verified** checksums (`SHASUMS256.txt`) and a self-installing `install.ps1` / `install.sh`.

The GUI functions compile everywhere (no-op stubs off Windows); networking and threading map to Winsock/pthread transparently, so largely the same program builds for any supported platform.

---

## 17. Limits & Language Capabilities

### Newly Supported in v0.5.0:
- **`assert(cond, [msg])`** — Fully implemented and wired in C codegen with panic formatting.
- **`read_line()` / `readLine()` / `input()`** — Full stdin line reading with automatic newline stripping.
- **Real RFC 4648 Base64** — `str_base64_encode` and `str_base64_decode` with full padding support.
- **Diagnostic Tooling** — `york doctor` for instant toolchain environment auditing.
- **Automated Uninstallation** — First-class `york uninstall` across Windows, macOS, and Linux.

### Remaining Boundaries & Work-in-Progress:
- **Array literals** (`[a, b, c]`) — use `Arena<T>` or slices.
- **Tuples**, **`match` expressions**, **closures/lambdas**.
- **Explicit casts** and **`sizeof`/`alignof`**.
- **Tagged/payload enum variants** — only unit variants compile to plain C enums.
- **Multi-File Module Linking (`import "file.yk"`)** — Fully supported in v1.1.1; organize and modularize larger codebases across multiple source files effortlessly.
- **Built-in Generic `HashMap<K, V>` & Sum Types** — Out-of-the-box key-value dictionaries and `Result<T, E>` / `Option<T>` error handling primitives.
- **Custom `yc` Systems Compiler Driver** — Standalone GCC/Clang-compatible compiler driver with full optimization flags (`-O2`, `-O3`).
- **Wasm backend** — `york_codegen_wasm` / `york_web` are stubs; the web path today is `york mobile` producing a PWA from native code, not a compiled-to-Wasm toolchain.

---

## 18. Exhaustive Code Examples & Recipes

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

### Recipe B: String Pipeline
```java
public static void main(String[] args) {
    String raw = "  york programming LANGUAGE  ";
    println(raw.trim().toLower().replace("programming", "systems"));  // york systems language
    println(to_string("hello".startsWith("he")));                     // true
    println(to_string("abc-abc-abc".count("abc")));                   // 3
    println("York".repeat(3));                                        // YorkYorkYork
}
```

### Recipe C: Tiny TCP Server
```java
public static void main(String[] args) {
    long srv = net_listen(8080);
    println("listening on 8080");
    while (true) {
        long c = net_accept(srv);
        net_send(c, "HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nYork!");
        net_close(c);
    }
}
```

### Recipe D: Mobile App (habits counter)
```java
public static void main(String[] args) {
    write_file("index.html", "<!DOCTYPE html><html><head><title>York Habits</title>"
        + "<style>body{font-family:system-ui;padding:24px;background:#04070d;color:#e2e8f0}</style>"
        + "</head><body><h1>York Habits</h1><div id='c' style='font-size:64px'>0</div>"
        + "<button onclick='add()'>+1</button><script src='app.js'></script></body></html>");
    append_file("app.js", "var n=0;function draw(){document.getElementById('c').innerText=n;}"
        + "York.ready(function(){York.storage.get('n').then(function(v){n=Number(v)||0;draw();});});"
        + "function add(){n++;York.vibrate(15);York.storage.set('n',n);draw();}");
    // then:  york mobile main.yk --out build --apk --platform both
}
```

---

## License

MIT — see [LICENSE](LICENSE).

## Version History

See [CHANGELOG.md](CHANGELOG.md) for the full release log. v1.1.0 is the major stable milestone release: the complete mobile framework (`york mobile`, real signed APKs, iOS Xcode projects, the Core 12 `York.*` runtime) plus the expanded typed-IR compiler with sized primitives, networking, threads, and the full built-in/string method surface documented here.

### Architectural Guarantees & Constraints in v1.1.0
1. **No Garbage Collector (GC)**: Relies entirely on contiguous memory arenas (`Arena<T>`) for zero-overhead performance without background pauses.
2. **No Heavy Runtime or Virtual Machine**: Compiles directly into clean C11 source code bundled into standalone native machine executables.
3. **No Browser-Based Desktop Wrappers**: Bypasses Electron/Chromium entirely, linking straight to native lightweight Win32 system libraries (`user32.lib`, `gdi32.lib`).
4. **Arena-Centric Data Structures**: Focuses on flat `Arena<T>` and raw struct/array compositions for deterministic high-throughput allocation.
5. **Streamlined Error Handling**: Utilizes direct `assert(cond, msg)` runtime checks for robust execution flow without heavyweight try/catch overhead.