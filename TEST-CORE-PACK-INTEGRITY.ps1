# StimTake Core Pack Integrity Check
$ErrorActionPreference = 'Stop'
$Root = Join-Path $PSScriptRoot 'StimTake-Rust-UI-Concept-Build\Core_Packs'
if (-not (Test-Path -LiteralPath $Root)) { throw "Core_Packs folder not found: $Root" }
$failed = @()
foreach ($folder in Get-ChildItem -LiteralPath $Root -Directory) {
  $manifestPath = Join-Path $folder.FullName 'deck.json'
  if (-not (Test-Path -LiteralPath $manifestPath)) { $failed += "$($folder.Name): missing deck.json"; continue }
  try { $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json } catch { $failed += "$($folder.Name): invalid deck.json"; continue }
  if (@($manifest.slots).Count -ne 20) { $failed += "$($folder.Name): expected 20 manifest slots, got $(@($manifest.slots).Count)"; continue }
  foreach ($slot in $manifest.slots) {
    if ([string]::IsNullOrWhiteSpace([string]$slot.file) -or -not (Test-Path -LiteralPath (Join-Path $folder.FullName $slot.file) -PathType Leaf)) { $failed += "$($folder.Name): missing slot $($slot.slot) file '$($slot.file)'" }
  }
  $htmlCount = @(Get-ChildItem -LiteralPath $folder.FullName -Filter '*.html' -File).Count
  if ($htmlCount -ne 20) { $failed += "$($folder.Name): expected 20 HTML files, got $htmlCount" }
}
if ($failed.Count) { $failed | ForEach-Object { Write-Error $_ }; exit 1 }
Write-Host 'PASS: every built-in pack has a valid deck.json and 20 matching HTML overlays.' -ForegroundColor Green
