# York Releases

Static distribution site for the York compiler — binaries and installers only.

Served on GitHub Pages. The compiler **source is not part of this repository**.

## Contents

| Path | What it is |
| --- | --- |
| `index.html` / `site.js` | The download website (generated, do not edit) |
| `downloads/york-setup-x64.exe` | Windows installer (Inno Setup, x64) |
| `downloads/york-x86_64-windows.zip` | Portable archive — `bin\york.exe` |
| `downloads/york.exe` | Single-file executable — no install |
| `downloads/SHASUMS256.txt` | SHA-256 checksums for every artifact |
| `installers/install.ps1` | Windows one-line installer |
| `installers/install.sh` | POSIX one-line installer (fires when macOS/Linux builds ship) |
| `CHANGELOG.md` | Release notes |
| `LICENSE` | Binary-use license |

## Hosting

1. Push the contents of this directory to a public repo (e.g. `york-lang/york-releases`, branch `main`).
2. Repo → Settings → Pages → **Deploy from a branch** → branch `main`, folder `/`.
3. Site appears at `https://{owner}.github.io/{repo}`.

Everything is referenced with relative paths, so it works under any URL.