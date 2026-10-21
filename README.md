<div align="center">

  # StimTake — Chaturbate Overlay Manager for OBS

  **A Windows toolkit for interactive OBS overlays, tip tracking, and customizable action packs.**

  ![Platform](https://img.shields.io/badge/Platform-Windows%2010%2F11-0078d4?style=for-the-badge)
  ![OBS](https://img.shields.io/badge/Integration-OBS%20Studio-7b61ff?style=for-the-badge)
  ![Actions](https://img.shields.io/badge/Interactive%20Actions-20-00b8d9?style=for-the-badge)

  **[🌐 Project Website](https://tcdoverlord.github.io/StimTake-Overlay-Manager/)** · **[📦 GitHub Repository](https://github.com/tcdoverlord/StimTake-Overlay-Manager)** · **[🐛 Report an Issue](https://github.com/tcdoverlord/StimTake-Overlay-Manager/issues)**

</div>

---

<p align="center">
  <a href="https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker/blob/main/assets/daily-pack-banner.png">
    <img src="./assets/daily-pack-banner.png" alt="StimTake Daily Pack — 20 everyday interactive actions for OBS" width="100%">
  </a>
</p>

---

## Table of Contents

- [Project Overview](#project-overview)
- [Action Packs](#action-packs)
- [Features](#features)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [Using StimTake with OBS](#using-stimtake-with-obs)
- [Project Images](#project-images)
- [Troubleshooting](#troubleshooting)
- [Security and Privacy](#security-and-privacy)
- [Development and Contributions](#development-and-contributions)
- [Project Links](#project-links)
- [License and Attribution](#license-and-attribution)

---

## Project Overview

StimTake is a Windows toolkit for Chaturbate creators who want to add interactive actions and customizable visual experiences to OBS Studio. The project includes a desktop interface for managing action slots and decks, plus supporting tools and overlays.

The repository's current project direction includes a 20-slot action manager, customizable decks, overlay editing, and integration workflows intended to make interactive show elements easier to organize.

StimTake is actively evolving. Features and packaging can vary by build. Check the current source, release notes, and included documentation when confirming whether a specific capability is available in a given version.

## Action Packs

### Halloween 2026

<p align="center">
  <a href="https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker/blob/main/assets/stimtake-promo-banner.png">
    <img src="./assets/stimtake-promo-banner.png" alt="StimTake Halloween 2026 action pack" width="100%">
  </a>
</p>

A seasonal set of spooky themed interactions. The available actions and artwork depend on the installed Halloween pack version.

A set of everyday interactive actions, including wheel spins, dice, cards, mystery prizes, races, and other game-style overlays. The included pack and installed build determine the available actions.

> **Image setup:** Keep `daily-pack-banner.png` and `halloween-2026-banner.png` in the repository's `assets/` folder. If the Halloween banner has a different filename in your current project, update both the relative `src` and GitHub `href` above to match the committed filename.

## Features

- **20 action slots** for organizing interactive show actions.
- **Deck switching** to keep different sets of actions organized.
- **Custom decks** for personal layouts and show themes.
- **Overlay editing and assignment** for supported slots.
- **OBS integration workflows** for using supported overlays as browser sources.
- **Pack-based customization** for seasonal and everyday themes.

Feature availability depends on the current build and installed content. The README does not imply that every overlay, event source, or integration is automatically configured on every system.

## Requirements

Before using StimTake, prepare:

- A supported Windows 10 or Windows 11 system.
- The StimTake build or installer appropriate to your release.
- OBS Studio for streaming and displaying supported overlays.
- Any required browser-source URLs, local services, or pack files described by the build's documentation.
- Internet access when downloading the repository, releases, or optional content.

System requirements vary with the specific release and with the OBS scenes, overlays, and other applications running at the same time.

## Quick Start

### 1. Get StimTake

Download the latest available release from the repository's [Releases page](https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker/releases), or clone the repository:

```powershell
git clone https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker.git
cd Chaturbate-OBS-Tip-Tracker
```

### 2. Launch the application

If using a packaged Windows release, run the supplied installer or executable and follow the instructions included with that release.

If working from source, follow the build instructions and scripts included in the repository. Build prerequisites can differ between versions.

### 3. Choose an action pack

Open the Decks or action-pack interface and select the deck you want to use, such as **Daily Pack** or **Halloween 2026**, if that pack is installed in your build.

### 4. Configure and test an overlay

Select an action slot, review its overlay assignment and settings, and test it before using it live. Enable live/click mode only when you're ready to run actions.

### 5. Add supported overlays to OBS

Use the URL or integration method documented by your installed build. Test the source in OBS before going live. A standalone HTML preview may not start the local application service by itself.

## Using StimTake with OBS

1. Open OBS Studio and select the scene where the overlay should appear.
2. Add a **Browser Source** when the selected overlay uses a browser-source workflow.
3. Use the local URL or file path provided by the running StimTake build and its documentation.
4. Set the source dimensions as appropriate for your scene.
5. Test the overlay and triggering workflow before streaming.

Exact URLs, ports, and source settings can vary by build. Use the values displayed by the running application instead of assuming a default endpoint.

## Project Images

The website's image pack includes screenshots and promotional assets for documenting the project. The website HTML uses relative image paths so the page and its assets can be hosted together.

For images displayed in this README, commit the relevant files into the repository. Relative paths such as `./assets/daily-pack-banner.png` work when the image exists at that location in the repository.

## Troubleshooting

### The application does not launch

- Check the release's included requirements and instructions.
- Confirm that you extracted or installed the full package.
- Run the executable from its expected location.
- Review any console output or error message before reinstalling.

### An action or overlay does not appear

- Confirm the correct deck and slot are selected.
- Check that an overlay is assigned to the slot.
- Confirm the relevant pack is installed and supported by the current build.
- Test the overlay before using it in a live scene.

### OBS Browser Source is blank

- Verify that the StimTake service or application required by the overlay is running.
- Confirm the URL or file path matches the instructions from your installed build.
- Refresh the Browser Source and review application output for errors.
- Remember that opening a standalone HTML preview does not necessarily start any local backend service.

### A pack or image is missing

- Confirm the pack was extracted or imported as described by its instructions.
- Check file paths and capitalization.
- For README and website images, confirm that the asset is committed to GitHub at the same path used in the Markdown or HTML.

## Security and Privacy

- Download releases from the official repository whenever possible.
- Review scripts before running them, especially when using development builds.
- Do not commit API keys, credentials, private configuration files, or personal data.
- Test integrations and event-triggered actions before relying on them during a live show.
- Keep OBS, Windows, and project dependencies updated.

## Development and Contributions

Bug reports, documentation improvements, reproducible tests, and focused code contributions are welcome.

When reporting an issue, include the StimTake version, Windows version, relevant error output, and steps to reproduce the behavior. Remove personal information, tokens, and other secrets from logs before posting.

- [Open an issue](https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker/issues)
- [View pull requests](https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker/pulls)

## Project Links

- **Repository:** https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker
- **Releases:** https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker/releases
- **Issues:** https://github.com/tcdoverlord/Chaturbate-OBS-Tip-Tracker/issues
- **Project website:** https://tcdoverlord.github.io/Chaturbate-OBS-Tip-Tracker/
- **Maintainer:** https://github.com/tcdoverlord

## License and Attribution

Review the license files and notices in the repository and any included dependencies before redistributing modified builds or bundled software. Do not assume that every component uses the same license.

---

<div align="center">

**Built by [TCDOVERLORD](https://github.com/tcdoverlord)**

*Interactive actions. Custom decks. OBS-ready workflows.*

</div>
