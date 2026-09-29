# York toolchain uninstaller for Windows.
# Run:  irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/uninstall.ps1 | iex
param(
    [switch]$Force,
    [switch]$Silent
)

$ErrorActionPreference = "Stop"

if (-not $Silent) {
    $York = @(
        "   ██╗  ██╗ ██████╗ ██████╗ ██╗  ██╗",
        "   ██║ ██╔╝██╔═══██╗██╔══██╗██║ ██╔╝",
        "   █████╔╝ ██║   ██║██████╔╝█████╔╝ ",
        "   ██╔═██╗ ██║   ██║██╔══██╗██╔═██╗ ",
        "   ██║  ██╗╚██████╔╝██║  ██║██║  ██╗",
        "   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝"
    )
    Write-Host ""
    foreach ($line in $York) { Write-Host $line -ForegroundColor Cyan }
    Write-Host ""
    Write-Host "York Uninstaller" -ForegroundColor Yellow
    Write-Host "────────────────────────────────────────────────────────────" -ForegroundColor DarkGray
}

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\york"
$BinDir = Join-Path $InstallDir "bin"
$HomeYorkDir = Join-Path $env:USERPROFILE ".york"

if (-not (Test-Path $InstallDir) -and -not (Test-Path $HomeYorkDir)) {
    if (-not $Silent) {
        Write-Host "York is not installed in the standard directories ($InstallDir)." -ForegroundColor Yellow
    }
}

if (-not $Force -and -not $Silent) {
    $confirm = Read-Host "Are you sure you want to uninstall York? (y/N)"
    if ($confirm -notmatch "^[Yy]$") {
        Write-Host "Uninstallation cancelled." -ForegroundColor Green
        exit 0
    }
}

# 1. Clean from PATH
if (-not $Silent) { Write-Host "[1/3] Removing York from User PATH..." -ForegroundColor Cyan }
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath) {
    $entries = $userPath -split ';' | Where-Object { 
        $trimmed = $_.Trim()
        $trimmed.Length -gt 0 -and $trimmed -ne $BinDir -and $trimmed -ne $InstallDir
    }
    $newPath = $entries -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    if (-not $Silent) { Write-Host "      Cleaned PATH environment variable." -ForegroundColor Green }
}

# 2. Remove program directory
if (-not $Silent) { Write-Host "[2/3] Removing York program files..." -ForegroundColor Cyan }
if (Test-Path $InstallDir) {
    try {
        Remove-Item -Recurse -Force $InstallDir -ErrorAction Stop
        if (-not $Silent) { Write-Host "      Removed $InstallDir" -ForegroundColor Green }
    } catch {
        if (-not $Silent) { Write-Host "      Warning: Could not remove all files in $InstallDir (in use?)" -ForegroundColor Yellow }
    }
} else {
    if (-not $Silent) { Write-Host "      $InstallDir not found (already removed)." -ForegroundColor DarkGray }
}

# 3. Clean optional cache/toolchain data in ~/.york
if (-not $Silent) { Write-Host "[3/3] Removing cache and runtime data..." -ForegroundColor Cyan }
if (Test-Path $HomeYorkDir) {
    try {
        Remove-Item -Recurse -Force $HomeYorkDir -ErrorAction SilentlyContinue
        if (-not $Silent) { Write-Host "      Removed $HomeYorkDir" -ForegroundColor Green }
    } catch {
        # ignore cache removal errors
    }
}

if (-not $Silent) {
    Write-Host ""
    Write-Host "York has been successfully uninstalled from your system." -ForegroundColor Green
    Write-Host "Restart your terminal to apply PATH changes." -ForegroundColor Yellow
    Write-Host ""
}
