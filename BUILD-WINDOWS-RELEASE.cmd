@echo off
setlocal EnableExtensions EnableDelayedExpansion
cd /d "%~dp0"
set "APP_NAME=StimTake"
set "PROJECT=%~dp0StimTake-Rust-UI-Concept-Build"
set "OBS_URL=http://127.0.0.1:8790/obs.html"
set "METADATA_FILE=%TEMP%\stimtake-cargo-metadata-%RANDOM%.json"
set "TARGET_DIR_FILE=%TEMP%\stimtake-target-dir-%RANDOM%.txt"
set "FAIL=0"
echo ============================================================
echo   StimTake - Full Windows Release Build Gate
echo ============================================================
echo.
where cargo >nul 2>nul || goto :NO_CARGO
where rustc >nul 2>nul || goto :NO_RUSTC
if not exist "%PROJECT%\Cargo.toml" goto :NO_MANIFEST
if not exist "%PROJECT%\src\main.rs" goto :NO_SOURCE
if not exist "%PROJECT%\assets\StimTake.ico" goto :NO_ICON
if not exist "%PROJECT%\StimTake.rc" goto :NO_ICON_RESOURCE
pushd "%PROJECT%"
cargo metadata --no-deps --format-version 1 > "%METADATA_FILE%"
if errorlevel 1 (popd & goto :METADATA_FAILED)
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "$m=Get-Content -Raw -LiteralPath $env:METADATA_FILE | ConvertFrom-Json; $p=@($m.packages | Where-Object { $_.targets.name -contains 'StimTake' }); if($p.Count -lt 1){exit 2}; if([string]::IsNullOrWhiteSpace($m.target_directory)){exit 3}; [IO.File]::WriteAllText($env:TARGET_DIR_FILE,[string]$m.target_directory)"
if errorlevel 1 (popd & goto :BAD_METADATA)
set /p "CARGO_TARGET_DIR="<"%TARGET_DIR_FILE%"
if not defined CARGO_TARGET_DIR (popd & goto :BAD_TARGET_DIR)
echo [PASS] Cargo metadata confirms binary target StimTake.
echo [PASS] Cargo target directory: !CARGO_TARGET_DIR!
echo.
echo [1/8] cargo check
cargo check
if errorlevel 1 (popd & goto :CHECK_FAILED)
echo [PASS] cargo check
echo.
echo [2/8] cargo test
cargo test
if errorlevel 1 (popd & goto :TEST_FAILED)
echo [PASS] cargo test
echo.
echo [3/8] cargo build --release
cargo build --release
if errorlevel 1 (popd & goto :BUILD_FAILED)
popd
echo [PASS] cargo build --release
echo.
set "EXE=!CARGO_TARGET_DIR!\release\StimTake.exe"
if not exist "!CARGO_TARGET_DIR!\release" goto :NO_RELEASE_DIR
if not exist "!EXE!" goto :NO_EXE
for %%F in ("!EXE!") do if %%~zF LEQ 0 goto :EMPTY_EXE
copy /Y "!EXE!" "%~dp0StimTake.exe" >nul
if errorlevel 1 goto :STAGE_FAILED
echo [PASS] StimTake.exe built and staged beside the installer script.
echo.
echo [4/8] Checking release package inputs
if not exist "%PROJECT%\ui\index.html" goto :NO_UI
if not exist "%PROJECT%\Core_Packs" goto :NO_PACKS
if not exist "%~dp0TEST-STIMTAKE-RUNTIME.ps1" goto :NO_SMOKE_TEST
echo [PASS] UI, Core_Packs, and runtime smoke test are present.
echo.
echo [5/8] Checking overlay pack structure
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "$root=Join-Path $env:PROJECT 'Core_Packs'; $dirs=@(Get-ChildItem -LiteralPath $root -Directory); if($dirs.Count -eq 0){exit 2}; $bad=@(); foreach($d in $dirs){$n=@(Get-ChildItem -LiteralPath $d.FullName -File -Filter '*.html').Count; if($n -ne 20){$bad += ($d.Name+': '+$n+' root HTML overlays')}}; if($bad.Count){$bad | ForEach-Object {Write-Host ('[FAIL] '+$_)}; exit 3}; Write-Host ('[PASS] '+$dirs.Count+' packs each contain 20 root HTML overlays')"
if errorlevel 1 goto :PACKS_FAILED
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0TEST-CORE-PACK-INTEGRITY.ps1"
if errorlevel 1 goto :PACKS_FAILED
echo.
echo [6/8] Compiling installer
set "ISCC="
if exist "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if not defined ISCC if exist "%ProgramFiles%\Inno Setup 6\ISCC.exe" set "ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe"
for /f "delims=" %%I in ('where ISCC.exe 2^>nul') do if not defined ISCC set "ISCC=%%I"
if not defined ISCC goto :NO_ISCC
"!ISCC!" "%~dp0StimTake-Setup.iss"
if errorlevel 1 goto :INSTALLER_FAILED
echo [PASS] Installer compiled.
echo.
echo [7/8] Verifying staged release executable
if not exist "%~dp0StimTake.exe" goto :NO_STAGED_EXE
for %%F in ("%~dp0StimTake.exe") do if %%~zF LEQ 0 goto :EMPTY_STAGED_EXE
echo [PASS] StimTake.exe exists and is non-empty.
echo.
echo [8/8] Verifying installer output
if not exist "%~dp0StimTake-Setup-1.0.0.exe" goto :NO_INSTALLER
for %%F in ("%~dp0StimTake-Setup-1.0.0.exe") do if %%~zF LEQ 0 goto :EMPTY_INSTALLER
echo [PASS] Installer exists and is non-empty.
echo.
echo ============================================================
echo BUILD GATE PASSED
echo ============================================================
echo EXE: %~dp0StimTake.exe
echo Installer: %~dp0StimTake-Setup-1.0.0.exe
echo Next: run TEST-STIMTAKE-RUNTIME.ps1, then verify live OBS rendering.
call :CLEANUP
pause
exit /b 0
:NO_CARGO
echo [FAIL] cargo was not found in PATH.
goto :FAILED
:NO_RUSTC
echo [FAIL] rustc was not found in PATH.
goto :FAILED
:NO_MANIFEST
echo [FAIL] Cargo.toml missing from %PROJECT%.
goto :FAILED
:NO_SOURCE
echo [FAIL] src\main.rs missing.
goto :FAILED
:METADATA_FAILED
echo [FAIL] cargo metadata failed.
goto :FAILED
:BAD_METADATA
echo [FAIL] Cargo metadata did not contain the StimTake binary target and target_directory.
goto :FAILED
:BAD_TARGET_DIR
echo [FAIL] Cargo target_directory is empty.
goto :FAILED
:CHECK_FAILED
echo [FAIL] cargo check failed.
goto :FAILED
:TEST_FAILED
echo [FAIL] cargo test failed.
goto :FAILED
:BUILD_FAILED
echo [FAIL] cargo build --release failed.
goto :FAILED
:NO_RELEASE_DIR
echo [FAIL] Cargo release directory missing: !CARGO_TARGET_DIR!\release
goto :FAILED
:NO_EXE
echo [FAIL] StimTake.exe was not produced in Cargo's release directory.
goto :FAILED
:EMPTY_EXE
echo [FAIL] StimTake.exe exists but is empty.
goto :FAILED
:STAGE_FAILED
echo [FAIL] Could not copy StimTake.exe into the release package.
goto :FAILED
:NO_UI
echo [FAIL] UI index.html missing.
goto :FAILED
:NO_PACKS
echo [FAIL] Core_Packs missing.
goto :FAILED
:NO_SMOKE_TEST
echo [FAIL] TEST-STIMTAKE-RUNTIME.ps1 missing.
goto :FAILED
:PACKS_FAILED
echo [FAIL] Overlay pack structure check failed.
goto :FAILED
:NO_ISCC
echo [FAIL] Inno Setup 6 ISCC.exe not found. Install Inno Setup 6 and retry.
goto :FAILED
:INSTALLER_FAILED
echo [FAIL] Installer compilation failed.
goto :FAILED
:NO_STAGED_EXE
echo [FAIL] Packaged StimTake.exe missing.
goto :FAILED
:EMPTY_STAGED_EXE
echo [FAIL] Packaged StimTake.exe is empty.
goto :FAILED
:NO_INSTALLER
echo [FAIL] StimTake-Setup-1.0.0.exe missing.
goto :FAILED
:EMPTY_INSTALLER
echo [FAIL] Installer exists but is empty.
goto :FAILED
:FAILED
echo.
echo ============================================================
echo BUILD GATE FAILED - fix the first [FAIL] above and retry.
echo ============================================================
call :CLEANUP
pause
exit /b 1
:CLEANUP
if exist "%METADATA_FILE%" del /q "%METADATA_FILE%" >nul 2>nul
if exist "%TARGET_DIR_FILE%" del /q "%TARGET_DIR_FILE%" >nul 2>nul
exit /b 0
