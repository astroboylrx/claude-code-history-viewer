<div align="center">

<img src="docs/assets/app-icon.png" alt="CCHV Logo" width="120" />

# Claude Code History Viewer

**[jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) のフォーク** — 追加機能と Linux 修正を含みます。

**Claude Code**、**Codex CLI**、**OpenCode**、**Kimi CLI** などからの会話を閲覧、検索、分析 — 100% オフライン。

</div>

---

## 追加機能

- **OpenCode ディレクトリベースのグループ化** — 単一の「global」プロジェクトではなく、ワークツリーごとにセッションをグループ化
- **Kimi CLI サポート** — セッション閲覧、検索、トークン統計を備えた完全なプロバイダー
- **統一されたプロジェクト名** — すべてのプロバイダーが `~/path/to/project` 形式で表示
- **プロジェクト別モデル分布** — 個別プロジェクト統計ページのモデル使用分析カード
- **グローバル統計：クリック可能なトッププロジェクト** — トッププロジェクトカードのプロジェクトをクリックしてナビゲーション
- **プロバイダー別カラーバッジ** — トッププロジェクトリストのプロバイダー別バッジ（アンバー=claude、グリーン=codex、オレンジ=kimi、ブルー=opencode）
- **フォントスケール対応** — すべてのテキストがフォントスケールスライダー（90%-130%）に従います

<!-- ## Linux / WebKitGTK 修正

- グローバル `OverlayScrollbars` を削除（WebKitGTK のイベント処理と競合）
- リサイズ可能パネルのドラッグ後カーソルが固まる問題を修正
- プロジェクトクリック時の 2-4 秒のフリーズを回避するためチャートレンダリングを遅延
- 要素ごとの Radix Tooltip ツリーを共有ツールチップシステムに置き換え
- トークン分布チャートの 100% 不可視アークを修正 -->

## macOS インストール（Homebrew 経由）

このフォークには Apple Developer 証明書がないため、プレビルドの `.dmg` ファイルは Gatekeeper でブロックされます。Homebrew を使用してソースからビルドしてください — アプリはローカルでコンパイルされ、`/Applications` に直接インストールされます：

```bash
brew install --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

新しいリリース後のアップデート：

```bash
brew reinstall --cask https://raw.githubusercontent.com/astroboylrx/claude-code-history-viewer/main/scripts/claude-code-history-viewer.rb
```

## アップストリーム

オリジナルプロジェクトは [jhlee0409/claude-code-history-viewer](https://github.com/jhlee0409/claude-code-history-viewer) をご覧ください。
