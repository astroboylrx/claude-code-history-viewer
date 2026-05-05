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
<!-- - **子代理会话过滤** — 从所有统计中排除子代理会话
- **改进的活动热力图** — 更大的图块（20px），基于 Portal 的工具提示，将工具图表移至热力图下方
-->

<!-- ## Linux / WebKitGTK 修复

- 移除了全局 `OverlayScrollbars`（与 WebKitGTK 事件处理冲突）
- 修复了可调整大小面板拖动后光标卡住的问题
- 延迟图表渲染以避免点击项目时 2-4 秒的冻结
- 使用共享工具提示系统替代每个元素的 Radix Tooltip 树
- 修复了 Token 分布图中 100% 不可见的圆弧 -->

## 安装

### Linux

从[最新发布](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest)下载 `.AppImage`：

```bash
chmod +x Claude*.AppImage
./Claude*.AppImage
```

### Windows

从[最新发布](https://github.com/astroboylrx/claude-code-history-viewer/releases/latest)下载安装程序（`.exe`）。

### macOS（从源码构建）

由于此应用未使用付费的 Apple Developer 证书，下载预编译的 `.dmg` 会被 macOS Gatekeeper 阻止。要绕过此限制，只需几个步骤即可在本地编译应用。

**1. 安装构建依赖（如果尚未安装）**

需要安装 `pnpm` 和 `rust`。可以通过 Homebrew 轻松安装：

```bash
brew install node pnpm rust
```

**2. 下载源代码**

```bash
git clone --depth 1 --branch v1.11.1 https://github.com/astroboylrx/claude-code-history-viewer.git
cd claude-code-history-viewer
```

**3. 安装包并构建应用**

```bash
pnpm install --frozen-lockfile
pnpm tauri build --no-sign
```

构建过程中可能会短暂出现 Finder 窗口 — 这是正常现象，几秒后会自动关闭。

**4. 将应用移至应用程序文件夹**

```bash
cp -r "src-tauri/target/release/bundle/macos/Claude Code History Viewer.app" "/Applications/"
```

**5. 清理（可选）**

可以安全删除下载的源代码文件夹以释放空间：

```bash
cd ..
rm -rf claude-code-history-viewer
```

## 上游项目

原始项目请见 [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer)。
