param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "StimTake"),
    [switch]$BuildFromSource
)
$ErrorActionPreference = "Stop"
$SourceRoot = $PSScriptRoot
$ProjectRoot = Join-Path $SourceRoot "StimTake-Rust-UI-Concept-Build"
if (-not (Test-Path (Join-Path $ProjectRoot "Cargo.toml"))) { throw "Rust project missing: $ProjectRoot" }

$Parent = Split-Path -Parent $InstallDir
if (-not (Test-Path $Parent)) { New-Item -ItemType Directory -Path $Parent -Force | Out-Null }
if (Test-Path $InstallDir) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $backupDir = "$InstallDir-backup-$stamp"
    Write-Host "Backing up existing install to $backupDir" -ForegroundColor Yellow
    Copy-Item -LiteralPath $InstallDir -Destination $backupDir -Recurse
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
# Preserve user assets from any existing installation by copying first; shipped defaults are replaced cleanly.
$existingAssets = Join-Path $InstallDir "obs_assets"
if (-not (Test-Path $existingAssets)) { New-Item -ItemType Directory -Path $existingAssets -Force | Out-Null }
$packBackup = Join-Path $existingAssets "backups\packs"
$configBackup = Join-Path $existingAssets "backups\configs"
$custom = Join-Path $existingAssets "custom_overlays"
New-Item -ItemType Directory -Path $packBackup,$configBackup,$custom -Force | Out-Null

# Copy the project as the installed application folder, keeping user data separate.
$destProject = Join-Path $InstallDir "StimTake-Rust-UI-Concept-Build"
if (Test-Path $destProject) { Remove-Item -LiteralPath $destProject -Recurse -Force }
Copy-Item -LiteralPath $ProjectRoot -Destination $destProject -Recurse

# Seed obs_assets only if absent, never overwrite existing user backups/custom assets.
$sourceAssets = Join-Path $ProjectRoot "obs_assets"
foreach ($sub in @("README.txt","backups","custom_overlays")) {
    $from = Join-Path $sourceAssets $sub
    $to = Join-Path $InstallDir "obs_assets\$sub"
    if ((Test-Path $from) -and -not (Test-Path $to)) { Copy-Item -LiteralPath $from -Destination $to -Recurse }
}

if ($BuildFromSource) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Cargo was not found. Install Rust from https://rustup.rs, reopen PowerShell, then run this script again with -BuildFromSource."
    }
    Push-Location $destProject
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
        $exe = Join-Path $destProject "target\release\stimtake-rust-deck.exe"
        if (-not (Test-Path $exe)) {
            $exe = Get-ChildItem (Join-Path $destProject "target\release") -Filter "*.exe" | Select-Object -First 1 -ExpandProperty FullName
        }
        if ($exe) {
            Copy-Item -LiteralPath $exe -Destination (Join-Path $InstallDir "StimTake-Rust-Deck.exe") -Force
        }
    } finally { Pop-Location }
}

# Create launcher and open-data-folder helpers.
$launcher = @'
@echo off
setlocal
cd /d "%~dp0"
if exist "StimTake-Rust-Deck.exe" (
  start "" "%~dp0StimTake-Rust-Deck.exe"
) else (
  echo StimTake has not been compiled yet.
  echo Run BUILD-STIMTAKE.ps1 -BuildFromSource after installing Rust.
  pause
)
'@
Set-Content -LiteralPath (Join-Path $InstallDir "START-STIMTAKE.cmd") -Value $launcher -Encoding ASCII

$openAssets = @'
@echo off
explorer "%~dp0obs_assets"
'@
Set-Content -LiteralPath (Join-Path $InstallDir "OPEN-OBS-ASSETS.cmd") -Value $openAssets -Encoding ASCII

$readme = @"
StimTake Rust Deck — install layout
===================================
Install folder: $InstallDir

Core_Packs:
  Shipped default pack library. Treat this as the clean default set.

obs_assets:
  backups\packs       User backups of packs before edits.
  backups\configs     Config backups.
  custom_overlays     User custom overlay assets.

Build / launch:
  1. Install Rust (rustup.rs) if building from source.
  2. Open PowerShell in this extracted setup folder.
  3. Run: .\BUILD-STIMTAKE.ps1 -BuildFromSource
  4. Launch: $InstallDir\START-STIMTAKE.cmd

OBS Browser Source:
  http://127.0.0.1:8790/obs.html

Important:
  The current Rust UI source is a manual 20-slot overlay controller.
  The pack folders are installed as a clean library; automatic pack browsing,
  backup/restore UI, and pack-to-slot loading still require integration in Rust.
"@
Set-Content -LiteralPath (Join-Path $InstallDir "INSTALL-README.txt") -Value $readme -Encoding UTF8
Write-Host ""
Write-Host "StimTake files installed to: $InstallDir" -ForegroundColor Green
if ($BuildFromSource) { Write-Host "Build requested. Check output above; launch with START-STIMTAKE.cmd." }
else { Write-Host "Files staged. To compile the app, run this script again with -BuildFromSource." -ForegroundColor Yellow }
