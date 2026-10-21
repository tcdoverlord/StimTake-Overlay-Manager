STIMTAKE WINDOWS RELEASE CANDIDATE

This is a source-and-build package, not a pre-verified executable.

BUILD
1. Install Rust via https://rustup.rs/ and Inno Setup 6.
2. Open this folder and run BUILD-WINDOWS-RELEASE.cmd.
3. The script uses cargo metadata to resolve Cargo's target directory, runs cargo check, cargo test, and cargo build --release, verifies the explicit StimTake binary target and non-empty StimTake.exe, validates the overlay packs, and compiles the installer.
4. A successful build does not establish server or OBS runtime success.

GATE 2 — SERVER
Launch StimTake.exe, then check /health, /obs.html, /api/state, and /api/memory at http://127.0.0.1:8790. Run TEST-STIMTAKE-RUNTIME.ps1 from this folder.

GATE 3 — OBS
Configure one OBS Browser Source to http://127.0.0.1:8790/obs.html. Verify multiple overlays, the five-instance cap, individual stop behavior, and deck switching. A blank transparent compositor is expected when no overlays are active.

STATUS
Source package assembled; ZIP and static checks must pass. Rust build, runtime HTTP, and live OBS tests must be performed on Windows before calling this a verified release.
