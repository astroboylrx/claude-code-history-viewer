<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**[jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) 的分支**，包含额外功能和 Linux 修复。

浏览、搜索和分析来自 **Claude Code**、**Codex CLI**、**OpenCode**、**Kimi CLI** 等的对话记录 — 100% 离线。

</div>

---

## 新增功能

- **OpenCode 按目录分组** — 会话按工作树分组，而非单一的"global"项目
- **Kimi CLI 支持** — 完整的提供商，支持会话浏览、搜索和 Token 统计
- **一致的项目名称** — 所有提供商显示 `~/path/to/project` 格式
- **按项目的模型分布** — 单个项目统计页面中的模型使用分析卡片
- **全局统计：可点击的热门项目** — 点击热门项目卡片中的项目即可导航到该项目
- **提供商彩色徽章** — 热门项目列表中按提供商显示不同颜色徽章（琥珀色=claude，绿色=codex，橙色=kimi，蓝色=opencode）
- **字体缩放支持** — 所有文本遵循字体缩放滑块设置（90%-130%）

<!-- ## Linux / WebKitGTK 修复

- 移除了全局 `OverlayScrollbars`（与 WebKitGTK 事件处理冲突）
- 修复了可调整大小面板拖动后光标卡住的问题
- 延迟图表渲染以避免点击项目时 2-4 秒的冻结
- 使用共享工具提示系统替代每个元素的 Radix Tooltip 树
- 修复了 Token 分布图中 100% 不可见的圆弧 -->

## macOS 安装（通过 Homebrew）

此分支没有 Apple 开发者证书，因此预构建的 `.dmg` 文件会被 Gatekeeper 阻止。请使用 Homebrew 从源码构建 — 应用在本地编译并直接安装到 `/Applications`：

```bash
brew install --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

新版本发布后更新：

```bash
brew reinstall --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

## 上游项目

原始项目请见 [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)。
