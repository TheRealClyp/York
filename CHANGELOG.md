# York Changelog

All notable changes to York are tracked here. The site and `release` folder mirror this file.

## 1.1.2 — Package Manager (`ypkg`) & Language Server (`york-lsp`)

**Ecosystem milestone.** Version 1.1.2 closes the two gaps from the architecture review that block a perfect **5.0** rating — a package manager and official LSP support:

- **Package Manager (`ypkg` / `york pkg`)**: Cargo-style `york.toml` manifests; `init`, `add`, `remove`, `install`, `list`, `publish`; local `york_modules/` dependency tree; walks *up* parent directories to find a manifest.
- **Language Server (`york-lsp` / `york lsp`)**: Standard LSP over stdio (`Content-Length` framed JSON-RPC 2.0). Real-time diagnostics from the **full** lexer → parser → sema pipeline with byte-accurate line/column ranges, autocomplete (keywords + every built-in), and hover. Compatible with Neovim, VS Code, Sublime, Emacs, and Helix.
- **Full toolchain binaries ship everywhere**: `york`, `ypkg`, `york-lsp`, and `yc` are now bundled into the Windows installer, the Windows zip, and both Linux `musl` tarballs (x86_64 + aarch64).
- **README 14.1 / 14.2**: Complete Package Manager and LSP reference documentation added.

---

## 1.1.1 — Multi-File Imports, HashMap, Result/Option & yc Compiler Driver

**Major feature release.** Version 1.1.1 eliminates remaining language hurdles with full multi-file module linking (`import "file.yk"`), built-in generic `HashMap<K, V>`, `Result<T, E>` & `Option<T>` safe error handling, the standalone `yc` GCC/Clang-compatible compiler driver, and the sleek futuristic **YORK** wordmark brand identity.

- **Multi-File Module Linking**: Split projects across multiple source files using `import "filename.yk";`.
- **Built-in Generic `HashMap<K, V>`**: High-performance key-value maps out of the box.
- **`Result<T, E>` & `Option<T>`**: Safe algebraic sum types for clean error handling.
- **Custom `yc` Compiler Driver**: GCC/Clang-compatible systems compiler driver (`yc main.yk -o app -O2`).
- **Sleek Wordmark Branding**: Upgraded all logos, icons, and wordmark assets.

---

## 0.5.0 — The Integrity & Usability Milestone

**Monumental release.** Version 0.5.0 brings a complete brand identity revamp with the official high-tech York logo, comprehensive security verification and false-positive resolution, first-class uninstallation across all platforms, built-in diagnostic tooling (`york doctor`), real Base64 encoding/decoding, native `assert` & `read_line` runtime implementations, and major ergonomics upgrades.

### Brand & Visual Identity
- **Official High-Tech York Logo**: Crafted a futuristic glowing geometric "Y" crest with electric cyan (`#38bdf8`) and violet (`#a855f7`) gradients on an obsidian shield.
- **Full Icon Asset Pipeline**: Shipped high-resolution vector SVG (`assets/logo.svg`), multi-size raster PNGs (`512x512` down to `16x16`), and multi-resolution Windows icons (`logo.ico`, `favicon.ico`).
- **PE Icon & Resource Integration**: Both `york.exe` and `york-setup.exe` now embed the official logo icon and Windows VersionInfo metadata.

### Security, Trust & False-Positive Resolution
- **Transparent Security Manifesto (`SECURITY.md`)**: Comprehensive documentation explaining why Microsoft Defender SmartScreen and antivirus heuristics flag newly released open-source compilers (lack of expensive commercial EV certs, native code generation heuristics, and Mark-of-the-Web).
- **Hardened Inno Setup Installer**: Replaced indirect `cmd.exe /c start` process spawning with native Windows ShellExec to eliminate dropper heuristic false positives in antivirus engines.
- **Automatic SmartScreen Unblocking**: `install.ps1` automatically clears the `Zone.Identifier` stream on downloaded binaries via `Unblock-File`, preventing SmartScreen blocking.
- **PE Manifest & Metadata**: `york_setup` now bundles complete Windows metadata, `asInvoker` execution level, and proper publisher attributes.

### Complete Uninstallation Suite
- **`york uninstall`**: Built-in CLI command for safe local uninstallation with interactive confirmation (`--yes` to skip).
- **`installers/uninstall.ps1`**: Standalone PowerShell uninstaller that cleans program files, purges User PATH environment variables in Windows registry, and wipes caches.
- **`installers/uninstall.sh`**: Standalone Unix uninstaller that removes `~/.york` and cleans PATH exports from `~/.bashrc`, `~/.zshrc`, and `~/.profile`.
- **`install.ps1 -Uninstall`**: Integrated uninstallation switch into the main installer.
- **Inno Setup Uninstaller**: Clean Windows Settings "Add or remove programs" integration with dedicated icon.

### Developer Experience & Diagnostics
- **`york doctor`**: Instant health-check command checking host platform, York executable path, PATH registration, native C compilers (`clang`, `gcc`, `cl.exe`, `tcc`, `zig`), and mobile toolchains (`javac`).
- **`york check` & `york build`**: Enhanced compiler error diagnostics.

### Language & Runtime Additions
- **Real Base64**: Fully compliant RFC 4648 Base64 encoding (`str_base64_encode`) and decoding (`str_base64_decode`) natively implemented in the C codegen runtime.
- **Native `assert()`**: Shipped `__york_assert` in C codegen runtime supporting single-arg `assert(cond)` and dual-arg `assert(cond, message)`.
- **Native `read_line()`**: Fully implemented `__york_read_line` in C codegen runtime for reading user input with automatic trailing newline stripping.
- **100% Test Suite Coverage**: All end-to-end and parser test suites passing cleanly.

---

## 0.4.0 — One Source, Every Target

**Major release.** The complete mobile framework shipped in 0.3.6 is now the centerpiece of the language story, the README is rewritten as an exhaustive reference manual (v0.4.0), and the toolchain lands its biggest surface area yet.

### Headline
- **`york mobile`** — one York program → installable PWA + Android project + real signed APK (York's own `aapt2 → javac → d8 → zipalign → apksigner` toolchain, no Gradle/Android Studio/SDK) + complete iOS Xcode project.
- **Core 12 `York.*` runtime** — storage, toast, dialogs, vibrate, geolocation, accelerometer, battery, network, clipboard, share, openUrl, camera. Three transports (Android Java bridge, iOS `WKScriptMessageHandler`, web fallbacks), one API.
- **Rewrite of README.md** — full technical spec & reference manual documenting the verified type system (sized primitives `i8`–`i128`, `u8`–`u128`, `f16`–`f64`), the complete built-in function list, chainable string methods, networking, threading, Win32 GUI, `Arena<T>`, mobile framework, and honest "not-yet-supported" section.

### CLI
- `york mobile --platform android|ios|both` (default `both`, aliases `apple`/`all`), `--apk`, `--icon`, `--serve`, `--name`, `--out`.
- `york new-mobile NAME` — scaffolds a mobile app project with a demo template.
- npm-style entry auto-detection for `run`/`build`/`check`/`mobile`.

### Language surface
- Sized primitives + 128-bit integers, `packed` structs, slices `T[]`, foreach + `arena[i]` indexing, if-as-expression, implicit `break` after switch arms.
- Built-ins grouped: math, strings, files, OS/system, raw sockets (`net_listen`/`net_accept`/`net_connect`/`net_send`/`net_recv`/`net_close`), threads (`thread_spawn`/`thread_join`/`thread_self`), binary/hash (`bin_pack`, `crypto_hash`).
- Chainable String methods with camelCase + snake_case aliases.

### Docs & site
- Site cheat-sheet updated for `york new-mobile` and `york mobile … --apk`.
- README mirrored to the site repository.

---

## 0.3.6 — Mobile Apps: PWA + Android APK + iOS (`york mobile`)

**Headline:** York apps now ship to phones. One York program produces an installable PWA, an Android project, a real signed APK compiled entirely by York's own toolchain, and a complete iOS Xcode project.

### New: `york mobile` with `--platform` and `--apk`

- `york mobile <app.yk> --out build` wraps the app in an installable **PWA** (manifest, service worker, icons).
- `--platform android` / `--platform ios` / `--platform both` (default) selects the native shells to generate.
- `--apk` compiles a **real signed APK** with York's own build pipeline — aapt2 → javac → d8 → zipalign → apksigner — no Gradle, no Android Studio, no full Android SDK. The first build fetches Google's command-line compilers once (~120 MB) into `~/.york/android`; everything after is offline.
- `york new-mobile NAME` scaffolds a mobile app project.

### New: `York.*` device runtime (Core 12)

Every York mobile app automatically gets `window.York` with 12 device features, working across Android bridges, iOS `WKScriptMessageHandler`, and plain-WebView/PWA fallbacks:

storage (KV), toast, alert/confirm dialogs, vibration, geolocation, accelerometer, battery, network type, clipboard read/write, share sheet, open URL, and camera (front/back/pick via the file chooser path).

- Android: `YorkBridge.java` + `YorkCallable.java` (lambda-free, compiled with `--release 11` and D8 9.0.3).
- iOS: full `YorkApp.xcodeproj` with SwiftUI wrapper, `YorkBridge.swift`, Info.plist location/motion/camera usage strings, adaptive launch background.
- New manifest permissions: `ACCESS_FINE_LOCATION`, `ACCESS_COARSE_LOCATION`, `VIBRATE`, `ACCESS_NETWORK_STATE`, `INTERNET`.
- SwiftUI shell loads the app from a bundled `www` folder (offline-first, same code as the PWA).

### Docs

- README: new section 14 "Mobile Apps: PWA, Android APK & iOS (`york mobile`)" with the full `York.*` API table.
- Site quickstart cheat-sheet updated: `york new-mobile` + `york mobile … --apk`.

### Toolchain

- `android_build.rs`: deterministic D8/@`--release 11` build recipe validated end-to-end; signed APK verified via `apksigner` (v2 scheme passing).
- Templates confirmed working with JDK 26 (no lambdas; D8 9.0.3 required).

---

## 0.3.5 — Git is an Island

Initial public release line. (Tracked from 0.3.5 onward; earlier development history is preserved in the git log.)