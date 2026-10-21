StimTake — Deck Switch Stop-All Fix Candidate

This full source package applies one targeted runtime change: changing deck IDs stops every currently active overlay, unregisters active-resource protection, clears expiration state, and loads the new deck with all cards OFF. The overlay card files and URLs still load normally, and new cards can be activated after the switch.

Build:
1. Extract this ZIP into a new folder. Do not overwrite your locked MAIN package.
2. Run BUILD-WINDOWS-RELEASE.cmd from the package root.
3. Start the rebuilt StimTake.exe.
4. Run TEST-STIMTAKE-RUNTIME.ps1.
5. In OBS, activate an overlay in deck A, switch to deck B, and confirm the old overlay disappears immediately and all deck B cards are initially off. Then activate a deck B card and confirm it appears.

The included existing EXE and installer are reference copies, not rebuilt artifacts. Promote this candidate only after a successful Windows build, passing smoke test, and live OBS verification.
