# Build the York website with the York toolchain and serve it locally.
# Usage:  .\serve.ps1  [port]
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "site")

$port = if ($args.Count -gt 0) { $args[0] } else { 8000 }

& york run main.yk
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "Serving site at http://localhost:$port" -ForegroundColor Cyan
Write-Host "Press Ctrl+C to stop."
python -m http.server $port