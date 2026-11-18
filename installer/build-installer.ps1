# Builds installer\NetPulse-<version>.msi from the release executable.
#
# Requirements:
#   * cargo build --release has been run (or this script does it)
#   * WiX 3.14 binaries: candle.exe and light.exe
#     Download wix314-binaries.zip from https://github.com/wixtoolset/wix3/releases
#     and pass the extracted folder with -WixBin, or set WIX_BIN.
param(
    [string]$WixBin = $env:WIX_BIN,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $here

if (-not $SkipBuild) {
    Push-Location $root
    cargo build --release
    Pop-Location
}

if (-not $WixBin) {
    throw "Set WIX_BIN (or pass -WixBin) to the folder containing candle.exe and light.exe."
}

$candle = Join-Path $WixBin "candle.exe"
$light = Join-Path $WixBin "light.exe"
if (-not (Test-Path $candle) -or -not (Test-Path $light)) {
    throw "candle.exe / light.exe not found in $WixBin"
}

$version = (Select-String -Path (Join-Path $root "Cargo.toml") -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$output = Join-Path $here "NetPulse-$version.msi"

Push-Location $here
& $candle -ext WixUIExtension -nologo NetPulse.wxs
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "candle failed with exit code $LASTEXITCODE" }
& $light -ext WixUIExtension -cultures:en-us -nologo -out $output NetPulse.wixobj
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "light failed with exit code $LASTEXITCODE" }
Pop-Location

Remove-Item (Join-Path $here "NetPulse.wixobj") -ErrorAction SilentlyContinue
Write-Output "Built $output"
