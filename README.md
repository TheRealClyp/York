# York

**The speed of C, the feel of Java & JS.**

![Version](https://img.shields.io/badge/version-0.1.0-67e8f9?style=flat-square)
![Platform](https://img.shields.io/badge/platform-Windows%20x64-blue?style=flat-square)
![Active](https://img.shields.io/badge/status-development-green?style=flat-square)
![License](https://img.shields.io/badge/license-MIT-8A9BD8?style=flat-square)

York is a small, fast systems language. Clean, familiar syntax that compiles to a
single native binary — no runtime, no garbage collector, no classes.

```
fn main() {
    let name = "York";
    printline("Hello, {}!", name);
}
```

## Features

- **Fast compiler** — source to native output in seconds, designed for edit-run-fix loops
- **Tiny output** — one self-contained executable, ~1.3 MB, no dependencies
- **Simple tooling** — `york new`, `york run`, `york build`
- **Enums & switch, functions, variables, `if` / `loop`**

## Download (Windows x64)

| Artifact | Notes |
| --- | --- |
| `downloads/york-setup-x64.exe` | Installer — installs to `%LOCALAPPDATA%\Programs\york`, adds PATH, self-verifying uninstaller |
| `downloads/york-x86_64-windows.zip` | Portable archive — unzip, run `bin\york.exe` |
| `downloads/york.exe` | Single-file executable — no install |

Every artifact is verified against `downloads/SHASUMS256.txt` (SHA-256):

```
certutil -hashfile downloads\york.exe SHA256
```

macOS and Linux builds are next in the pipeline.

## One-line install

Windows (PowerShell):

```
irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.ps1 | iex
```

macOS / Linux (fires when those builds ship):

```
curl -fsSL https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.sh | sh
```

## Quick start

```
york new hello
york run .\hello\main.yk
```

## Repository layout

```
downloads/    Official binaries + SHA-256 checksums
installers/   One-line install scripts
CHANGELOG.md  Release notes
LICENSE       MIT — binaries are free to use and redistribute
```

## License

MIT — see [LICENSE](LICENSE). The compiler source is not distributed publicly.