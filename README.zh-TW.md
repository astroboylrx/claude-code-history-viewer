<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**[jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) 的分支**，包含額外功能和 Linux 修復。

瀏覽、搜尋和分析來自 **Claude Code**、**Codex CLI**、**OpenCode**、**Kimi CLI** 等的對話記錄 — 100% 離線。

</div>

---

## 新增功能

- **OpenCode 按目錄分組** — 工作階段按工作樹分組，而非單一的「global」專案
- **Kimi CLI 支援** — 完整的提供者，支援工作階段瀏覽、搜尋和 Token 統計
- **一致的專案名稱** — 所有提供者顯示 `~/path/to/project` 格式
- **按專案的模型分佈** — 單一專案統計頁面中的模型使用分析卡片
- **全域統計：可點擊的熱門專案** — 點擊熱門專案卡片中的專案即可導航到該專案
- **提供者彩色徽章** — 熱門專案清單中按提供者顯示不同顏色徽章（琥珀色=claude，綠色=codex，橙色=kimi，藍色=opencode）
- **字型縮放支援** — 所有文字遵循字型縮放滑桿設定（90%-130%）

<!-- ## Linux / WebKitGTK 修復

- 移除了全域 `OverlayScrollbars`（與 WebKitGTK 事件處理衝突）
- 修復了可調整大小面板拖動後游標卡住的問題
- 延遲圖表渲染以避免點擊專案時 2-4 秒的凍結
- 使用共享工具提示系統替代每個元素的 Radix Tooltip 樹
- 修復了 Token 分佈圖中 100% 不可見的圓弧 -->

## macOS 安裝（透過 Homebrew）

此分支沒有 Apple 開發者憑證，因此預建的 `.dmg` 檔案會被 Gatekeeper 阻擋。請使用 Homebrew 從原始碼建置 — 應用程式在本機編譯並直接安裝到 `/Applications`：

```bash
brew install --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

新版本發佈後更新：

```bash
brew reinstall --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

## 上游專案

原始專案請見 [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)。
