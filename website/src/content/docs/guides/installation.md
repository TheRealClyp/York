---
title: Installation
description: Install the York programming language toolchain on Windows, Linux, or macOS.
---

Get up and running with the York programming language toolchain (`v0.2.55`) in seconds.

## Windows (PowerShell)

Run the automated installer script in PowerShell:

```powershell
irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.ps1 | iex
```

Or download `york-setup-x64.exe` or `york-x86_64-windows.zip` directly from the [GitHub Releases](https://github.com/TheRealClyp/York/releases).

## Linux / macOS (Bash)

Run the installer script in your terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.sh | bash
```

## Verifying Installation

Verify that the York CLI toolchain is correctly installed:

```bash
york --version
```
