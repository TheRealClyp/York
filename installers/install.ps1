# York toolchain installer for Windows.
# Run:  irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.ps1 | iex
param(
    [string]$Version = $env:YORK_VERSION,
    [string]$Base = $env:YORK_INSTALL_BASE,
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

if ($Uninstall) {
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $uninstaller = Join-Path $scriptDir "uninstall.ps1"
    if (Test-Path $uninstaller) {
        & $uninstaller
        exit $LASTEXITCODE
    } else {
        $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\york"
        $BinDir = Join-Path $InstallDir "bin"
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath) {
            $entries = $userPath -split ';' | Where-Object { $_.Trim() -ne $BinDir }
            [Environment]::SetEnvironmentVariable("Path", ($entries -join ';'), "User")
        }
        if (Test-Path $InstallDir) { Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue }
        Write-Host "York uninstalled successfully." -ForegroundColor Green
        exit 0
    }
}

# ── YORK banner ──────────────────────────────────────────────
$York = @(
    "   ██╗  ██╗ ██████╗ ██████╗ ██╗  ██╗",
    "   ██║ ██╔╝██╔═══██╗██╔══██╗██║ ██╔╝",
    "   █████╔╝ ██║   ██║██████╔╝█████╔╝ ",
    "   ██╔═██╗ ██║   ██║██╔══██╗██╔═██╗ ",
    "   ██║  ██╗╚██████╔╝██║  ██║██║  ██╗",
    "   ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝"
)
$Wasay = @(
    "██╗    ██╗  █████╗  ███████╗  █████╗  ██╗   ██╗",
    "██║    ██║  ██╔══██╗  ██╔════╝  ██╔══██╗  ╚██╗ ██╔╝",
    "██║ █╗ ██║  ███████║  ███████╗  ███████║   ╚████╔╝ ",
    "██║███╗██║  ██╔══██║  ╚════██║  ██╔══██║    ╚██╔╝  ",
    "╚███╔███╔╝  ██║  ██║  ███████║  ██║  ██║     ██║   ",
    " ╚══╝╚══╝  ╚═╝  ╚═╝  ╚══════╝  ╚═╝  ╚═╝     ╚═╝   "
)
Write-Host ""
foreach ($line in $York) { Write-Host $line -ForegroundColor Cyan }
foreach ($line in $Wasay) { Write-Host $line -ForegroundColor Magenta }
Write-Host ""
Write-Host "York v0.5.0 Installer (Verified Open Source Systems Language)" -ForegroundColor Cyan
Write-Host "────────────────────────────────────────────────────────────" -ForegroundColor DarkGray

# Defaults
if (-not $Base) { $Base = "https://raw.githubusercontent.com/TheRealClyp/York/main/downloads" }
if (-not $Version) { $Version = "0.5.0" }

$arch = $env:PROCESSOR_ARCHITECTURE
$target = switch ($arch) {
    "AMD64"  { "x86_64" }
    "ARM64"  { "aarch64" }
    default  { throw "Unsupported architecture: $arch" }
}

if ($target -eq "aarch64") {
    throw "York ARM64 Windows builds ship with the next release. Please use the x64 installer for now."
}

# Install location
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\york"
$BinDir = Join-Path $InstallDir "bin"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$exe = Join-Path $BinDir "york.exe"

$Step = 1
# If already installed, print version
if (Test-Path $exe) {
    try { $existing = & $exe --version } catch { $existing = "an older build" }
    Write-Host "Found existing $existing · upgrading in place." -ForegroundColor Yellow
}

# Artifact: york-<target>-windows.zip e.g. york-x86_64-windows.zip
$zipName = "york-$target-windows.zip"
$url = "$Base/$zipName"
$tmp = Join-Path $env:TEMP $zipName
$tmpDir = Join-Path $env:TEMP ("york-extract-" + [System.IO.Path]::GetRandomFileName())

Write-Host ("[{0}/4] Detected platform: {1} {2}" -f $Step++, $target, "windows") -ForegroundColor Cyan
Write-Host "[$Step/4] Downloading release package from $url..." -ForegroundColor Cyan; $Step++
$ProgressPreference = "SilentlyContinue"

# Download with custom User-Agent to avoid web filter / AV heuristic false flags
try {
    Invoke-WebRequest -Uri $url -OutFile $tmp -UserAgent "York-Installer/0.5.0 (Windows NT; x64)"
} catch {
    # Fallback to local copy if running from repo
    $localZip = Join-Path $PSScriptRoot $zipName
    if (Test-Path $localZip) {
        Copy-Item $localZip $tmp -Force
    } else {
        throw "Failed to download $url : $_"
    }
}

# Unblock downloaded file to clear Windows SmartScreen Mark-of-the-Web
Unblock-File -Path $tmp -ErrorAction SilentlyContinue

Write-Host "[$Step/4] Extracting $zipName..." -ForegroundColor Cyan; $Step++
Expand-Archive -Path $tmp -DestinationPath $tmpDir -Force

# Find york.exe inside the archive (root or bin/)
$srcExe = Get-ChildItem -Path $tmpDir -Recurse -Filter "york.exe" | Select-Object -First 1
if (-not $srcExe) { throw "Could not find york.exe in archive" }
Copy-Item $srcExe.FullName $exe -Force

# Unblock installed executable to prevent SmartScreen blocking
Unblock-File -Path $exe -ErrorAction SilentlyContinue

# Also copy uninstaller to install dir for offline uninstall
$uninstallerSrc = Join-Path $PSScriptRoot "uninstall.ps1"
if (Test-Path $uninstallerSrc) {
    Copy-Item $uninstallerSrc (Join-Path $InstallDir "uninstall.ps1") -Force
}

# Cleanup
Remove-Item -Force $tmp -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue

# Add $BinDir to the user PATH if not present
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
if (-not ($userPath -split ';' | Where-Object { $_.Trim() -eq $BinDir })) {
    $newPath = (($userPath.TrimEnd(';')) + ";" + $BinDir).TrimStart(';')
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "[$Step/4] York added to your PATH (User environment)" -ForegroundColor Cyan; $Step++
    Write-Host "         Please open a NEW terminal window so the PATH takes effect." -ForegroundColor Yellow
} else {
    Write-Host "[$Step/4] York is already configured in your PATH." -ForegroundColor Cyan; $Step++
}

Write-Host ""
Write-Host "✓ York v$Version installed successfully!" -ForegroundColor Green
Write-Host "Run 'york --help' or 'york --credits' to get started." -ForegroundColor White
Write-Host "To uninstall anytime, run: irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/uninstall.ps1 | iex" -ForegroundColor DarkGray
Write-Host ""