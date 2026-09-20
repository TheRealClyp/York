# York Release Notes

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
