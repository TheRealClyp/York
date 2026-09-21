# York Release Notes

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





