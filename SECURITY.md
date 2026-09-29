# Security Policy & Trust Verification — York Programming Language

> **Summary**: York is **100% open-source, clean, MIT-licensed software**. It is NOT malware, hacking software, a trojan, or a virus. This document explains why Windows Defender / SmartScreen may show warnings, how antivirus heuristics work on new compilers, and how to verify binary integrity.

---

## 1. Why Did Windows / SmartScreen Show a Warning?

If you download York or run its installer on Windows, you may encounter a prompt:
```
Windows protected your PC
Microsoft Defender SmartScreen prevented an unrecognized app from starting.
Running this app might put your PC at risk.
```
Or an antivirus scanner may flag the binary as "Generic", "HackTool", or "Riskware".

### Why does this happen?
1. **No Paid EV Code Signing Certificate ($500/year)**:
   Microsoft SmartScreen operates on a commercial reputation database. Unless an application is signed with an Extended Validation (EV) hardware-token certificate costing hundreds of dollars per year, Microsoft Defender SmartScreen flags **every newly compiled executable** as "unrecognized" by default until millions of downloads build cloud telemetry.
2. **Compiler & Systems Tool Capabilities**:
   York is a native systems compiler. By design, a compiler:
   - Generates executable machine code (`.exe` / `.obj`).
   - Invokes system linkers and compilers (`clang`, `gcc`, `cl.exe`).
   - Interacts with operating system sockets (`ws2_32.lib`), native Win32 windows, and threads.
   Antivirus machine-learning algorithms often flag unsigned binaries that interact with sockets, processes, or write executables as "HackTool" or "Riskware" (false positives).
3. **Zone.Identifier Mark-of-the-Web**:
   When you download a file via a browser or PowerShell without unblocking, Windows attaches a hidden stream (`Zone.Identifier`) marking the file as untrusted from the internet.

---

## 2. Our Security Guarantee

- **100% Open Source**: Every single line of York’s lexer, parser, type checker, semantic analyzer, C code generator, and CLI is public and inspectable in this repository.
- **Zero Telemetry, Zero Tracking, Zero Bloat**: York does not transmit telemetry, track users, or phone home.
- **Inspectable Output**: When York compiles your code, it generates human-readable C11 code before invoking the system compiler. You can inspect every generated C line at any time.
- **Reproducible Builds**: Releases are built transparently using standard Rust tooling (`cargo build --release`).

---

## 3. How to Run York Safely

### Option A: Allow in SmartScreen (Fastest)
1. When the blue "Windows protected your PC" screen appears:
2. Click **More info**.
3. Click **Run anyway**.

### Option B: Unblock via PowerShell
If downloaded via browser or script, remove the Mark-of-the-Web:
```powershell
Unblock-File "$env:LOCALAPPDATA\Programs\york\bin\york.exe"
```
Or for the installer:
```powershell
Unblock-File ".\york-setup-x64.exe"
```

### Option C: Build Directly From Source
If you prefer not to run precompiled binaries, compile York yourself in seconds:
```bash
git clone https://github.com/TheRealClyp/York.git
cd York
cargo build --release
# Built binary located at target/release/york.exe
```

---

## 4. Cryptographic Checksum Verification

Every official release publishes SHA-256 checksums in `SHASUMS256.txt`. Verify your downloads before executing:

### On Windows (PowerShell):
```powershell
Get-FileHash -Algorithm SHA256 .\york-x86_64-windows.zip
Get-FileHash -Algorithm SHA256 .\york-setup-x64.exe
```

### On Linux / macOS:
```bash
sha256sum -c SHASUMS256.txt
```

---

## 5. Reporting a Security Vulnerability

If you discover an actual security issue or vulnerability within York, please open an issue on GitHub at:
https://github.com/TheRealClyp/York/issues

We review and address all community reports promptly.
