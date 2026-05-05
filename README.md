<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**Fork of [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)** with extra features and Linux fixes.

Browse, search, and analyze conversations from **Claude Code**, **Codex CLI**, **OpenCode**, **Kimi CLI**, and more — 100% offline.

</div>

<div align="center">
<img src="docs/assets/screenshot.png" alt="Screenshot" width="100%" />
</div>

---

## Added Features

- **OpenCode directory-based grouping** — sessions grouped by worktree instead of a single "global" project
- **Kimi CLI support** — full provider with session browsing, search, and token stats
- **Consistent project names** — all providers show `~/path/to/project` format
- **Per-project model distribution** — model usage breakdown card on individual project stats
- **Global stats: clickable top projects** — click a project in the Top Projects card to navigate to it
- **Provider-colored badges** — per-provider badges (amber=claude, green=codex, orange=kimi, blue=opencode) in top projects list
- **Font-scale support** — all text respects the font scale slider (90%-130%)
<!-- - **Subagent session filtering** — subagent sessions excluded from all stats
- **Improved activity heatmap** — larger tiles (20px), portal-based tooltips, moved tools chart below heatmap
-->

<!-- ## Linux / WebKitGTK Fixes

- Removed global `OverlayScrollbars` (conflicted with WebKitGTK event handling)
- Fixed resizable panel cursor stuck after drag
- Deferred chart rendering to avoid 2-4s freeze on project click
- Shared tooltip system replacing per-element Radix Tooltip trees
- Fixed 100% invisible arc in token distribution chart -->

## Installation

### Linux

Download the `.AppImage` from the [latest release](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest):

```bash
chmod +x Claude*.AppImage
./Claude*.AppImage
```

### Windows

Download the installer (`.exe`) from the [latest release](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest).

### macOS (Build from Source)

Because this app does not use a paid Apple Developer certificate, downloading a pre-compiled `.dmg` will cause macOS Gatekeeper to block it. To bypass this, you can compile the app locally on your machine in just a few steps.

**1. Install build dependencies (if you don't have them)**

You will need `pnpm` and `rust` installed. You can easily get them via Homebrew:

```bash
brew install node pnpm rust
```

**2. Download the source code**

```bash
git clone --depth 1 --branch v1.11.1 https://github.com/astroboylrx/claude-code-history-viewer.git
cd claude-code-history-viewer
```

**3. Install packages & build the app**

```bash
pnpm install --frozen-lockfile
pnpm tauri build --no-sign
```

A Finder window may briefly appear during the build — this is normal and will close on its own after a few seconds.

**4. Move the app to your Applications folder**

```bash
cp -r "src-tauri/target/release/bundle/macos/Claude Code History Viewer.app" "/Applications/"
```

**5. Clean up (optional)**

You can now safely delete the downloaded source code folder to free up space:

```bash
cd ..
rm -rf claude-code-history-viewer
```

## Upstream

For the original project, see [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer).
