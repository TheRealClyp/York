# York toolchain installer for Windows.
# Run:  irm https://raw.githubusercontent.com/TheRealClyp/York/main/installers/install.ps1 | iex
param(
    [string]$Version = $env:YORK_VERSION,
    [string]$Base = $env:YORK_INSTALL_BASE
)

$ErrorActionPreference = "Stop"

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
Write-Host "York installer" -ForegroundColor Cyan

# Defaults
if (-not $Base) { $Base = "https://raw.githubusercontent.com/TheRealClyp/York/main/downloads" }
if (-not $Version) { $Version = "latest" }

$arch = $env:PROCESSOR_ARCHITECTURE
$target = switch ($arch) {
    "AMD64"  { "x86_64" }
    "ARM64"  { "aarch64" }
    default  { throw "Unsupported architecture: $arch" }
}

# Install location
$ErrorActionPreference = "Stop"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\york"
$BinDir = Join-Path $InstallDir "bin"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$exe = Join-Path $BinDir "york.exe"

$Step = 1
# If already installed, print version
if (Test-Path $exe) {
    try { $existing = & $exe --version } catch { $existing = "an older build" }
    Write-Host "Found existing $existing · will update in place." -ForegroundColor Yellow
}

# Artifact: york-<target>-<os>.zip  e.g. york-x86_64-windows.zip
$zipName = "york-$target-windows.zip"
$url = "$Base/$zipName"
$tmp = Join-Path $env:TEMP $zipName
$tmpDir = Join-Path $env:TEMP ("york-extract-" + [System.IO.Path]::GetRandomFileName())

Write-Host ("[{0}/4] detected {1} {2}" -f $Step++, $target, "windows") -ForegroundColor Cyan
Write-Host "[$Step/4] downloading $url" -ForegroundColor Cyan; $Step++
$ProgressPreference = "SilentlyContinue"
Invoke-WebRequest -Uri $url -OutFile $tmp

Write-Host "[$Step/4] extracting $zipName" -ForegroundColor Cyan; $Step++
Expand-Archive -Path $tmp -DestinationPath $tmpDir -Force

# Find york.exe inside the archive (root or bin/)
$srcExe = Get-ChildItem -Path $tmpDir -Recurse -Filter "york.exe" | Select-Object -First 1
if (-not $srcExe) { throw "could not find york.exe in archive" }
Copy-Item $srcExe.FullName $exe -Force

# Cleanup
Remove-Item -Force $tmp -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue

# Add $BinDir to the user PATH if not present
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
if (-not ($userPath -split ';' | Where-Object { $_.Trim() -eq $BinDir })) {
    $newPath = (($userPath.TrimEnd(';')) + ";" + $BinDir).TrimStart(';')
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "[$Step/4] york added to your PATH (user)" -ForegroundColor Cyan; $Step++
    Write-Host "         Open a NEW terminal so PATH takes effect." -ForegroundColor Yellow
} else {
    Write-Host "[$Step/4] york already on your PATH" -ForegroundColor Cyan; $Step++
}

Write-Host ""
Write-Host "York" -ForegroundColor Cyan
Write-Host "WASAY" -ForegroundColor Magenta
Write-Host ""
Write-Host "york $Version installed." -ForegroundColor Green
Write-Host "Run 'york --help' to get started."