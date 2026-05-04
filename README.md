<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**Fork of [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)** with extra features and Linux fixes.

Browse, search, and analyze conversations from **Claude Code**, **Codex CLI**, **OpenCode**, **Kimi CLI**, and more — 100% offline.

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

## macOS Install (via Homebrew)

This fork does not have an Apple Developer certificate, so pre-built `.dmg` files will be blocked by Gatekeeper. Instead, build from source using Homebrew — the app compiles locally and installs directly to `/Applications`:

```bash
brew install --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

To update after a new release:

```bash
brew reinstall --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

## Upstream

For the original project, see [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer).
