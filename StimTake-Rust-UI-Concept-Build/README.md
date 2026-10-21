# StimTake Rust UI Concept Build

This package brings together the current StimTake holiday-deck HTML UI concept and the existing Rust/egui manual overlay backend source as the next integration workspace.

## Included
- `ui/index.html` — latest StimTake UI concept: protected built-in holiday decks, custom decks, backup/restore controls, Click-to-Run mode, slot editing, overlay assignment clearing, Help & OBS URLs, and collapsible sections.
- `src/main.rs` — Rust native app and loopback HTTP server from the current Rust deck prototype.
- `Cargo.toml` and `BUILD-WINDOWS.cmd` — Windows build setup.

## Build on Windows
1. Install Rust with rustup: https://rustup.rs/
2. Open this folder in PowerShell.
3. Run `BUILD-WINDOWS.cmd` or `cargo build --release`.
4. The executable should be created at `target\release\stimtake-rust-deck.exe`.

## OBS browser source
When the Rust app is running, the existing backend serves:
- Overlay canvas: `http://127.0.0.1:8790/obs.html`
- Health/root endpoint: `http://127.0.0.1:8790/`

Add the overlay canvas URL as an OBS Browser Source and set its width/height to your canvas, commonly 1920x1080.

## Important integration status
This is a **source/build workspace, not a verified compiled release**. The current Rust native backend and the HTML UI concept are both included, but the HTML UI is not yet wired to every Rust backend action. In particular, browser-side deck management still uses browser storage, and HTML file selection in that UI is not yet the native file-copy/import pipeline. The existing Rust/egui window currently provides the working backend controls for importing files and driving the OBS canvas.

I could not compile or run Rust in this environment because `cargo`/`rustc` are not installed here. Please build on a Windows machine with Rust installed before relying on it in a live stream. Do not expose port 8790 outside localhost.
