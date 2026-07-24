<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**[jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) 的分支**，包含額外功能和 Linux 修復。

瀏覽、搜尋和分析來自 **Claude Code**、**Codex CLI**、**OpenCode**、**Kimi**（Kimi Code / Kimi CLI）等的對話記錄 — 100% 離線。

</div>

<div align="center">
<img src="docs/assets/screenshot.png" alt="Screenshot" width="100%" />
</div>

---

## 新增功能

- **OpenCode 按目錄分組** — 工作階段按工作樹分組，而非單一的「global」專案
- **Kimi 支援 (Kimi Code & Kimi CLI)** — 完整的提供者，支援工作階段瀏覽、搜尋和 Token 統計。徽章顯示「Kimi (Code)」或「Kimi (CLI)」
- **一致的專案名稱** — 所有提供者顯示 `~/path/to/project` 格式
- **按專案的模型分佈** — 單一專案統計頁面中的模型使用分析卡片
- **全域統計：可點擊的熱門專案** — 點擊熱門專案卡片中的專案即可導航到該專案
- **提供者彩色徽章** — 熱門專案清單中按提供者顯示不同顏色徽章（琥珀色=claude，綠色=codex，橙色=kimi，藍色=opencode）
- **字型縮放支援** — 所有文字遵循字型縮放滑桿設定（90%-130%）
<!-- - **子代理工作階段篩選** — 從所有統計中排除子代理工作階段
- **改進的活動熱力圖** — 更大的圖塊（20px），基於 Portal 的工具提示，將工具圖表移至熱力圖下方
-->

<!-- ## Linux / WebKitGTK 修復

- 移除了全域 `OverlayScrollbars`（與 WebKitGTK 事件處理衝突）
- 修復了可調整大小面板拖動後游標卡住的問題
- 延遲圖表渲染以避免點擊專案時 2-4 秒的凍結
- 使用共享工具提示系統替代每個元素的 Radix Tooltip 樹
- 修復了 Token 分佈圖中 100% 不可見的圓弧 -->

## 安裝

### Linux

從[最新發佈](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest)下載 `.AppImage`：

```bash
chmod +x Claude*.AppImage
./Claude*.AppImage
```

### Windows

從[最新發佈](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest)下載安裝程式（`.exe`）。

### macOS（從原始碼建置）

由於此應用程式未使用付費的 Apple Developer 憑證，下載預建的 `.dmg` 會被 macOS Gatekeeper 阻擋。要繞過此限制，只需幾個步驟即可在本機編譯應用程式。

**1. 安裝建置依賴（如果尚未安裝）**

需要安裝 `pnpm` 和 `rust`。可以透過 Homebrew 輕鬆安裝：

```bash
brew install node pnpm rust
```

**2. 下載原始碼**

```bash
git clone --depth 1 --branch v1.11.1 https://github.com/astroboylrx/claude-code-history-viewer.git
cd claude-code-history-viewer
```

**3. 安裝套件並建置應用程式**

```bash
pnpm install --frozen-lockfile
pnpm tauri build --no-sign
```

建置過程中可能會短暫出現 Finder 視窗 — 這是正常現象，幾秒後會自動關閉。

**4. 將應用程式移至應用程式資料夾**

```bash
cp -r "src-tauri/target/release/bundle/macos/Claude Code History Viewer.app" "/Applications/"
```

**5. 清理（選填）**

可以安全刪除下載的原始碼資料夾以釋放空間：

```bash
cd ..
rm -rf claude-code-history-viewer
```

## 注意事項

**macOS 隱私提示：** 首次啟動時，macOS 可能會顯示「存取其他 App 的資料」權限提示。這是正常的——應用需要讀取系統上安裝的 AI 程式設計工具的工作階段歷史記錄。點擊「允許」以啟用所有提供者。該提示只會出現一次。

## 上游專案

原始專案請見 [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)。
