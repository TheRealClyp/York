# York Release Notes

## 0.1.0 — 2026-09-19

### Windows (x64)
- `york-setup-x64.exe` — Inno Setup installer: live progress, installs to `%LOCALAPPDATA%\Programs\york`, adds `bin` to the user PATH, ships a self-verifying uninstaller.
- `york-x86_64-windows.zip` — portable archive, run `bin\york.exe`.
- `york.exe` — single-file executable, no install.

### Language
- Lexer, parser, semantics, code generation, and CLI — a York program compiles to a single native binary through C.
- Enums and `switch`, functions, variables, `if`/`loop`, `print` / `printline`.

### Infrastructure
- All artifacts verified against `downloads/SHASUMS256.txt` (SHA-256).
- `york run FILE` — compile and run in one step; `york new DIR` scaffolds a project.
- Git commit hashes are not part of this distribution; source is private.