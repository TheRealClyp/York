# ============================================================================
#  York Android Addon  (Windows)
#  Downloads the Android build compiler tools into ~\.york\android so that
#  `york mobile app.yk --apk` can produce a real signed .apk.
#  This is an OPTIONAL addon — like installing Android Studio — it is NOT
#  bundled with the York language installer.
#
#  Requires: curl.exe (comes with Windows 10+), and a JDK on PATH for signing.
#  Size: ~120 MB one-time download from Google's repository.
# ============================================================================
$ErrorActionPreference = "Stop"

$Root = Join-Path $HOME ".york\android"
New-Item -ItemType Directory -Force -Path $Root | Out-Null

$BtZip  = Join-Path $Root "build-tools.zip"
$PlZip  = Join-Path $Root "platform.zip"
$Tools  = Join-Path $Root "build-tools-36"
$JarDir = Join-Path $Root "platforms-34"
$Jar    = Join-Path $JarDir "android.jar"

$BuildToolsUrl = "https://dl.google.com/android/repository/build-tools_r36.1_windows.zip"
$PlatformUrl   = "https://dl.google.com/android/repository/platform-34-ext7_r03.zip"

Write-Host "York Android Addon" -ForegroundColor Cyan -NoNewline
Write-Host " — installing aapt2 / d8 / zipalign / apksigner + android.jar (~120MB, one-time)."

if (-not (Test-Path (Join-Path $Tools "aapt2.exe"))) {
    if (-not (Test-Path $BtZip)) {
        Write-Host "  downloading Android build tools ..."
        curl.exe -L -C - -o $BtZip $BuildToolsUrl
        if ($LASTEXITCODE -ne 0) { throw "build tools download failed" }
    }
    $tmp = Join-Path $Root "bt_extract"
    if (Test-Path $tmp) { Remove-Item $tmp -Recurse -Force }
    New-Item -ItemType Directory -Path $tmp | Out-Null
    tar.exe -xf $BtZip -C $tmp
    if ($LASTEXITCODE -ne 0) { throw "build tools extract failed" }
    $inner = Get-ChildItem $tmp -Directory | Select-Object -First 1
    if (-not $inner) { throw "build tools archive was empty" }
    Move-Item $inner.FullName $Tools
    Remove-Item $tmp -Recurse -Force
}

if (-not (Test-Path $Jar)) {
    if (-not (Test-Path $PlZip)) {
        Write-Host "  downloading Android platform android.jar ..."
        curl.exe -L -C - -o $PlZip $PlatformUrl
        if ($LASTEXITCODE -ne 0) { throw "platform download failed" }
    }
    $tmp = Join-Path $Root "pl_extract"
    if (Test-Path $tmp) { Remove-Item $tmp -Recurse -Force }
    New-Item -ItemType Directory -Path $tmp | Out-Null
    tar.exe -xf $PlZip -C $tmp
    if ($LASTEXITCODE -ne 0) { throw "platform extract failed" }
    $inner = Get-ChildItem $tmp -Directory | Select-Object -First 1
    if (-not $inner) { throw "platform archive was empty" }
    if (Test-Path $JarDir) { Remove-Item $JarDir -Recurse -Force }
    Move-Item $inner.FullName $JarDir
    Remove-Item $tmp -Recurse -Force
}

$KeyStore = Join-Path $Root "debug.keystore"
if (-not (Test-Path $KeyStore)) {
    $keytool = Get-Command keytool.exe -ErrorAction SilentlyContinue
    if ($keytool) {
        Write-Host "  creating local debug signing key ..."
        & $keytool.Source -genkeypair -v -keystore $KeyStore -storepass android -alias androiddebugkey -keypass android -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=York,C=US" 2>$null | Out-Null
    } else {
        Write-Host "  (no JDK on PATH — signing key will be created automatically by York when needed)" -ForegroundColor Yellow
    }
}

Remove-Item $BtZip, $PlZip -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Done: " -ForegroundColor Green -NoNewline
Write-Host "York Android Addon installed in $Root"
Write-Host "  Usage: `n    cd your-app`n    york mobile src/app.yk --apk"
Write-Host ""