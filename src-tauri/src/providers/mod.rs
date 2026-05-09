use serde::{Deserialize, Serialize};

pub mod aider;
pub mod antigravity;
pub mod claude;
pub mod cline;
pub mod codex;
pub mod cursor;
pub mod forgecode;
pub mod gemini;
pub mod kimi;
pub mod opencode;

/// Provider identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Aider,
    Antigravity,
    Claude,
    Cline,
    Codex,
    Cursor,
    ForgeCode,
    Gemini,
    Kimi,
    OpenCode,
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aider => "aider",
            Self::Antigravity => "antigravity",
            Self::Claude => "claude",
            Self::Cline => "cline",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::ForgeCode => "forgecode",
            Self::Gemini => "gemini",
            Self::Kimi => "kimi",
            Self::OpenCode => "opencode",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "aider" => Some(Self::Aider),
            "antigravity" => Some(Self::Antigravity),
            "claude" => Some(Self::Claude),
            "cline" => Some(Self::Cline),
            "codex" => Some(Self::Codex),
            "cursor" => Some(Self::Cursor),
            "forgecode" => Some(Self::ForgeCode),
            "gemini" => Some(Self::Gemini),
            "kimi" => Some(Self::Kimi),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Aider => "Aider",
            Self::Antigravity => "Antigravity",
            Self::Claude => "Claude Code",
            Self::Cline => "Cline",
            Self::Codex => "Codex CLI",
            Self::Cursor => "Cursor",
            Self::ForgeCode => "ForgeCode",
            Self::Gemini => "Gemini CLI",
            Self::Kimi => "Kimi CLI",
            Self::OpenCode => "OpenCode",
        }
    }
}

/// Information about a detected provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub base_path: String,
    pub is_available: bool,
}

/// Detect all available providers on the system
pub fn detect_providers() -> Vec<ProviderInfo> {
    let mut providers = Vec::new();

    if let Some(info) = claude::detect() {
        providers.push(info);
    }
    if let Some(info) = codex::detect() {
        providers.push(info);
    }
    if let Some(info) = gemini::detect() {
        providers.push(info);
    }
    if let Some(info) = kimi::detect() {
        providers.push(info);
    }
    if let Some(info) = forgecode::detect() {
        providers.push(info);
    }
    if let Some(info) = opencode::detect() {
        providers.push(info);
    }
    if let Some(info) = cline::detect() {
        providers.push(info);
    }
    if let Some(info) = cursor::detect() {
        providers.push(info);
    }
    if let Some(info) = aider::detect() {
        providers.push(info);
    }
    if let Some(info) = antigravity::detect() {
        providers.push(info);
    }

    providers
}
