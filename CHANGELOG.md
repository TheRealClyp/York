# York Release Notes

# York v0.3.4 â€” Make Mobile Apps

The big one. You can now build **mobile apps** (and Android APKs) entirely in York.

## New commands

- `york new-mobile <name>` â€” scaffold a mobile app project.
- `york mobile <file.yk> [--name "App Name"] [--out dir] [--icon icon.svg]` â€” build a mobile app.

A small York program generates your whole UI (dark-mode mobile web UI, counter, interactive habits list â€” see the template), and `york mobile` packages the result into:

- **An installable PWA** â€” `manifest.webmanifest`, offline service worker (`sw.js`), theme color, 192/512 icons. Tap "Add to Home Screen" on a phone.
- **An Android shell** â€” a complete Gradle project with a WebView `MainActivity.java`, app icon, and theme that you open in Android Studio and press **Run** to get a real APK.
- The web app itself is copied into the Android assets so the APK is fully offline.

## Marked improvements

- New CLI subcommand architecture (`Mobile`, `NewMobile`).
- York mobile UI generator template with JS interactivity (counter, list add, bottom tab bar).
- Output dir sanitized per build; custom icon support (`--icon`).
- Everything from v0.3.0â€“v0.3.3 carried over (threads, math/string builtins).

## Try it

```
york new-mobile myapp
cd myapp
york mobile src/app.yk --name "My App" --out build
```

Then open `build/android` in Android Studio and run, or host `build/` and install the PWA.

# York v0.3.3

New built-ins:

- `math_is_prime(int)` â€” `true` if the value is a prime number.
- `math_gcd(a, b)` â€” greatest common divisor.
- `math_lcm(a, b)` â€” least common multiple.
- `str_levenshtein("a", "b")` â€” Levenshtein edit distance between two strings.

Also includes everything from v0.3.0 (real multithreading), v0.3.1, and v0.3.2.

# York v0.3.2

New built-ins:

- `math_lerp(a, b, t)` â€” linear interpolation between `a` and `b` at factor `t`.
- `str_is_lower("...")` â€” `true` if the string contains at least one letter and no uppercase letters.
- `str_is_upper("...")` â€” `true` if the string contains at least one letter and no lowercase letters.
- `str_rev_words("word1 word2 ...")` â€” returns the words in reverse order ("a b c" -> "c b a").

Also includes everything from v0.3.0 (real multithreading) and v0.3.1.

# York v0.3.1

New built-ins:

- `math_is_even(int)` â€” `true` if the value is even.
- `math_is_odd(int)` â€” `true` if the value is odd.
- `str_is_numeric("...")` â€” `true` if the string contains only digits (and is non-empty).
- `str_is_alnum("...")` â€” `true` if the string contains only letters and digits (and is non-empty).

Also includes everything from v0.3.0: real multithreading (`thread_spawn`, `thread_join`, `thread_self`) plus the full v0.2.x built-in set.

# York v0.3.0 â€” Real Multithreading (BIG)

This is a big one: York now has **real native threads**.

## New built-ins

- `thread_spawn("functionName", arg)` â€” starts a new OS thread that executes a user-defined function by name, passing the integer argument. Returns a thread id (`long`), or `-1` if the function can't be found/zthread creation fails.
- `thread_join(id)` â€” waits for the thread to finish and releases its handle.
- `thread_self()` â€” returns the id of the current thread.

Any user function with a numeric first parameter (or no parameters) can be spawned as a thread.

## Platforms

- **Windows**: real threads via Win32 `CreateThread`, joined via `WaitForSingleObject`.
- **Linux**: real threads via `pthread_create`, joined via `pthread_join` (static musl builds included).

## Full v0.2.x feature set carried over

`math_pi`, `math_e`, `str_repeat`, `str_word_count`, `degrees_to_radians`, `radians_to_degrees`, `log2`, `fract`, `str_trim_left`, `str_trim_right`, `str_is_alpha`, `str_is_digit`, `random_float`, `math_sign`, `str_first`, `str_last`, string methods, collections, networking, crypto, native GUI, and more.

# York 0.2.9

## New Builtins

- `random_float()` â€” returns a random float in [0, 1).
- `math_sign(x)` â€” returns 1.0, 0.0, or -1.0 based on the sign of x.
- `str_first(s)` â€” first character as a string.
- `str_last(s)` â€” last character as a string.

## Notes

- Windows x64, Linux x86_64, and Linux aarch64 binaries available.
- Windows installer (york-setup-x64.exe) included.
- SHA-256 checksums verified for every artifact.

# York 0.2.8

## New Builtins

- `str_trim_left(s)` â€” strips leading whitespace.
- `str_trim_right(s)` â€” strips trailing whitespace.
- `str_is_alpha(s)` â€” true if every char is a letter.
- `str_is_digit(s)` â€” true if every char is a digit.

## Notes

- Windows x64, Linux x86_64, and Linux aarch64 binaries available.
- Windows installer (york-setup-x64.exe) included.
- SHA-256 checksums verified for every artifact.

# York 0.2.7

## New Builtins

- `degrees_to_radians(x)` â€” converts degrees to radians.
- `radians_to_degrees(x)` â€” converts radians to degrees.
- `log2(x)` â€” base-2 logarithm.
- `fract(x)` â€” fractional part of a float (positive for negatives: `fract(-1.25) == -0.25`).

## Notes

- Windows x64, Linux x86_64, and Linux aarch64 binaries available.
- Windows installer (york-setup-x64.exe) included.
- SHA-256 checksums verified for every artifact.

# York 0.2.6

## New Builtins

- `math_pi()` â€” returns the constant Ï€ (3.14159â€¦) as a float.
- `math_e()` â€” returns Euler's number e (2.71828â€¦) as a float.
- `str_repeat(s, n)` â€” repeats a string `n` times and returns the result.
- `str_word_count(s)` â€” counts whitespace-separated words in a string.

## Notes

- Windows x64, Linux x86_64, and Linux aarch64 binaries available.
- Windows installer (york-setup-x64.exe) included.
- SHA-256 checksums verified for every artifact.

## 0.2.55 - 2026-09-21

Massive new command suite release adding robust system inspection, file manipulation, and advanced string utilities.

### Added
- **OS & Hardware Inspection**: `os_cpu_count()`, `os_total_memory()`, `os_pid()`
- **System & User Environment**: `sys_username()`, `sys_hostname()`, `sys_time_str()`
- **Advanced File System Commands**: `fs_file_size(path)`, `fs_delete_file(path)`, `fs_copy_file(src, dst)`
- **Powerful String Utilities**: `str_slugify(s)`, `str_capitalize(s)`, `str_base64_encode(s)`

### Verified
- Tested end-to-end via `examples/commands_test.yk` compiling and running successfully to native binaries.
- Full release pipeline packaged, tagged (`v0.2.55`), and published to GitHub with all 11 binary/source assets and SHA-256 sums.

## 0.2.3 - 2026-09-20

Core compiler release with robust native Win32 GUI capabilities and clean project setup tooling.

Added
- Native Win32 window management, control creation (`control_button`, `control_label`, `control_textbox`), event polling, and message loops.
- Automatic console suppression for GUI apps (`--windows-gui` and linker configurations).
- Interactive `.yk` file selector and manifest discovery when running without explicit targets.

Fixed
- Resolved MSVC and GNU linker flags for Win32 GUI APIs (`user32.lib`, `gdi32.lib`, `shell32.lib`).
- Cleaned up auxiliary project files to maintain a pristine programming language repository.

Verified
- End-to-end smoke test, native executable builds, version bump across all platforms, and full GitHub release deployment.

## 0.2.2 - 2026-09-20

Native Windows Desktop GUI release.

Added
- Native Win32 window creation, event polling, message loops, and controls (`window_create`, `window_show`, `control_button`, `control_label`, `control_set_text`, `message_box`).
- Standalone `.exe` compilation entirely in York without external runtimes, HTML, JS, or Node.js.

Verified
- End-to-end Windows GUI example runs and responds to button clicks natively.
- Full release pipeline packaged and deployed for v0.2.2 across Windows and Linux.

## 0.2.1 - 2026-09-20

New randomness and string counting, following the 0.2.0 math/system batch.

Added
- `random_range(lo, hi)` â€” uniform integer in `[lo, hi)` using `srand()`-seeded C `rand()`, bias-corrected across the span.
- `String.count(needle)` â€” count of non-overlapping occurrences (e.g. `"banana banana".count("banana")` is `2`). Empty needles return `0`.

Changed
- `String.count` joins `indexOf`/`lastIndexOf`/`rfind` as integer-returning string methods.

Fixed
- Nothing regressed in the language; the release re-stages every binary, installer, source tarball, and checksum from a single pass.

Verified
- `random_range` range invariant and `Str.count` semantics pass end-to-end tests on Windows and Linux.
- Smoke suite passes identically on x86_64 Windows, x86_64 Linux, and aarch64 Linux (qemu).

## 0.2.0 - 2026-09-20

New math and system builtins, plus faster, cleaner builds.

Added
- Math: `sin()`, `cos()`, `tan()`, `ln()`, `log10()`, `exp()` â€” all resolve to C `math.h` functions and are inferred as floats.
- System: `env("NAME")` reads an environment variable (empty string when unset), and `platform_name()` returns `windows`, `linux`, `macos`, `freebsd`, or `unknown` at runtime.

Changed
- Release cross-build now compiles both x86_64 and arm64 (aarch64) Linux binaries via musl-gcc / rust-lld, verified on-device under qemu before packaging.
- Source tarballs and installers are re-staged each release; SHA-256 sums are generated for every artifact in `downloads` and `installers`.

Fixed
- WSL build sync now copies the complete `tools` workspace so release builds no longer fail to resolve workspace members.
- Version number is now carried through every artifact, installer, and the website in one pass.

Verified
- Trig, log, env, and platform calls pass end-to-end tests (York source -> C -> native binary) on Windows and Linux.
- Smoke suite passes identically on x86_64 Windows, x86_64 Linux, and aarch64 Linux (qemu).

## 0.1.1 â€” 2026-09-20

York's first tagged GitHub release. Everything below is verified against `downloads/SHASUMS256.txt` (SHA-256) and covered by the Windows and Linux installers.

### Language
- New builtin functions:
  - `min(a,b)`, `max(a,b)`, `clamp(x,lo,hi)` for int and float.
  - `to_int(str)`/`to_float(str)` string-to-number conversion.
  - `sleep(ms)` with aliases `sleep_ms`/`delay`, and `now()` returning millisecond epoch time.
- New String methods: `indexOf`, `lastIndexOf`, `charAt`, `repeat`, `isEmpty`, `replace`, `reverse`, `trimStart`, `trimEnd`, `padLeft`, `padRight` (with snake_case and `find`/`rfind`/`ljust`/`rjust` aliases where natural).
- String values compare with `==`, plus the existing lexer/parser/sema/codegen chain, enums + `switch`, functions, `if`/`for`, `println`, structs, and file I/O.

### Platforms
- Windows x64: `york-setup-x64.exe` (Inno Setup installer), `york-x86_64-windows.zip` (portable), `york.exe` (single file).
- Linux: fully-static musl builds for x86_64 and aarch64 (`york-x86_64-linux.tar.gz`, `york-aarch64-linux.tar.gz`), verified under QEMU/binfmt.
- `installers/install.sh` and `installers/install.ps1` install from the raw GitHub `downloads` bucket with explicit ARM64-Windows/macOS guards.

### Infrastructure & site
- Redesigned site on MVP.css (fast, system fonts) with current artifact links and SHA-256 verification.
- `york run FILE` compiles and runs in one step; `york check`, `york build`, `york new`; watch mode via `york dev`.
- Git commit hashes are not part of this distribution; source is private.

## 0.1.0 â€” 2026-09-19

### Windows (x64)
- `york-setup-x64.exe` â€” Inno Setup installer: live progress, installs to `%LOCALAPPDATA%\Programs\york`, adds `bin` to the user PATH, ships a self-verifying uninstaller.
- `york-x86_64-windows.zip` â€” portable archive, run `bin\york.exe`.
- `york.exe` â€” single-file executable, no install.

### Language
- Lexer, parser, semantics, code generation, and CLI â€” a York program compiles to a single native binary.
- Enums and `switch`, functions, variables, `if`/`for`, `println`.

### Examples
- `examples/` â€” five short, commented lessons (hello, variables, functions, enums, structs) with a Learn York guide in `examples/README.md`.

### Infrastructure
- All artifacts verified against `downloads/SHASUMS256.txt` (SHA-256).
- `york run FILE` â€” compile and run in one step; `york new DIR` scaffolds a project.
- Git commit hashes are not part of this distribution; source is private.














