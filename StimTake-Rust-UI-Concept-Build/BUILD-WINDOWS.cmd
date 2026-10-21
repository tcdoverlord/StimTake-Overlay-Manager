@echo off
setlocal EnableExtensions
cd /d "%~dp0"
echo ============================================================
echo StimTake - Rust Release Build
echo ============================================================
where cargo >nul 2>nul || (echo [FAIL] Cargo not found. Install Rust from https://rustup.rs/ & exit /b 1)
where rustc >nul 2>nul || (echo [FAIL] rustc not found. Install Rust from https://rustup.rs/ & exit /b 1)
if not exist Cargo.toml (echo [FAIL] Cargo.toml missing & exit /b 1)
cargo check || (echo [FAIL] cargo check & exit /b 1)
cargo test || (echo [FAIL] cargo test & exit /b 1)
cargo build --release || (echo [FAIL] cargo build --release & exit /b 1)
if not exist "target\release\StimTake.exe" (echo [FAIL] StimTake.exe missing from target\release & exit /b 1)
echo [PASS] StimTake.exe produced: %CD%\target\release\StimTake.exe
exit /b 0
