cask "claude-code-history-viewer" do
  version "1.11.1"
  sha256 :no_check

  url "https://github.com/astroboylrx/claude-code-history-viewer/archive/refs/tags/v#{version}.tar.gz",
      verified: "github.com/astroboylrx/claude-code-history-viewer/"
  name "Claude Code History Viewer"
  desc "History viewer for AI coding assistants (Claude Code, Codex, Kimi, OpenCode, etc.)"
  homepage "https://github.com/astroboylrx/claude-code-history-viewer"
  license "MIT"

  depends_on formula: "pnpm"
  depends_on formula: "rust"

  preflight do
    system_command "/bin/bash",
                   args: ["-lc", "pnpm install --frozen-lockfile && pnpm tauri build"],
                   env: {
                     "HOME" => staged_path.join(".home").to_s,
                   }
  end

  app "src-tauri/target/release/bundle/macos/Claude Code History Viewer.app"

  postflight do
    system_command "/bin/bash",
                   args: ["-c",
                          "rm -rf '#{staged_path}/node_modules' " \
                          "'#{staged_path}/src-tauri/target' " \
                          "'#{staged_path}/.home'"]
  end

  zap trash: [
    "~/.local/share/com.claude.history-viewer",
  ]
end
