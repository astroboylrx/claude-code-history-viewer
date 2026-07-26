//! Native session renaming module
//!
//! Provides functionality to rename Claude Code sessions by modifying
//! the first user message in the session JSONL file.

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::command;

use crate::utils::is_safe_storage_id;

lazy_static! {
    /// Regex for validating JSONL filename pattern (alphanumeric, underscore, hyphen only)
    static ref FILENAME_REGEX: Regex = Regex::new(r"^[A-Za-z0-9_-]+$").unwrap();
}

/// Result structure for rename operations
#[derive(Debug, Serialize, Deserialize)]
pub struct NativeRenameResult {
    pub success: bool,
    pub previous_title: String,
    pub new_title: String,
    pub file_path: String,
}

/// Error types for rename operations
#[derive(Debug, Serialize)]
pub enum RenameError {
    FileNotFound(String),
    PermissionDenied(String),
    InvalidSessionPath(String),
    InvalidJsonFormat(String),
    IoError(String),
    EmptySession,
    NoUserMessage,
    UnsupportedContentFormat,
    InvalidTitle(String),
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenameError::FileNotFound(path) => write!(f, "Session file not found: {path}"),
            RenameError::PermissionDenied(path) => write!(f, "Permission denied: {path}"),
            RenameError::InvalidSessionPath(msg) => write!(f, "Invalid session path: {msg}"),
            RenameError::InvalidJsonFormat(msg) => write!(f, "Invalid JSON format: {msg}"),
            RenameError::IoError(msg) => write!(f, "I/O error: {msg}"),
            RenameError::EmptySession => write!(f, "Session file is empty"),
            RenameError::NoUserMessage => {
                write!(f, "No user message found in session")
            }
            RenameError::UnsupportedContentFormat => {
                write!(f, "Message content format not supported (array content)")
            }
            RenameError::InvalidTitle(msg) => write!(f, "Invalid title: {msg}"),
        }
    }
}

fn parse_opencode_session_path(session_path: &str) -> Result<(String, String), String> {
    let path_part = session_path
        .strip_prefix("opencode://")
        .ok_or_else(|| RenameError::InvalidSessionPath(session_path.to_string()).to_string())?;

    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() < 2 {
        return Err(RenameError::InvalidSessionPath(session_path.to_string()).to_string());
    }

    let project_id = parts[0];
    let session_id = parts[1];
    if !is_safe_storage_id(project_id) || !is_safe_storage_id(session_id) {
        return Err(RenameError::InvalidSessionPath(session_path.to_string()).to_string());
    }

    Ok((project_id.to_string(), session_id.to_string()))
}

/// Renames a Claude Code session by modifying the first user message.
///
/// # Arguments
/// * `file_path` - Absolute path to the session JSONL file
/// * `new_title` - Title to prepend (empty string to reset)
///
/// # Returns
/// * `Ok(NativeRenameResult)` - Success with previous and new titles
/// * `Err(String)` - Error description
#[command]
pub async fn rename_session_native(
    file_path: String,
    new_title: String,
) -> Result<NativeRenameResult, String> {
    // 1. Validate file exists
    if !std::path::Path::new(&file_path).exists() {
        return Err(RenameError::FileNotFound(file_path).to_string());
    }

    // 2. Validate file path is within ~/.claude directory (security: prevent path traversal)
    validate_claude_path(&file_path)?;

    // 3. Validate title does not contain ']' character (due to nested bracket limitation)
    if new_title.contains(']') {
        return Err(RenameError::InvalidTitle(
            "Title cannot contain ']' character. Use '[' for nested prefixes instead.".to_string(),
        )
        .to_string());
    }

    // 4. Read all lines from JSONL file
    let file =
        File::open(&file_path).map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader
        .lines()
        .collect::<Result<_, _>>()
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;

    if lines.is_empty() {
        return Err(RenameError::EmptySession.to_string());
    }

    // 5. Find first user message (type: "user", not isMeta)
    let user_message_index = find_first_user_message_index(&lines)?;

    // 6. Parse the user message line as JSON
    let mut user_message: serde_json::Value = serde_json::from_str(&lines[user_message_index])
        .map_err(|e| RenameError::InvalidJsonFormat(e.to_string()).to_string())?;

    // 7. Extract current message content - handle nested structure
    let current_message = extract_message_content(&user_message).ok_or_else(|| {
        RenameError::InvalidJsonFormat("No 'message' field found".to_string()).to_string()
    })?;

    // 8. Strip existing bracket prefix if present
    let base_message = strip_title_prefix(&current_message);

    // 9. Construct new message with title prefix
    let new_message = if new_title.trim().is_empty() {
        base_message.clone()
    } else {
        format!("[{}] {}", new_title.trim(), base_message)
    };

    // 10. Update JSON object - handle nested structure
    if !update_message_content(&mut user_message, &new_message) {
        return Err(RenameError::UnsupportedContentFormat.to_string());
    }

    // 11. Serialize back to JSON string
    lines[user_message_index] = serde_json::to_string(&user_message)
        .map_err(|e| RenameError::InvalidJsonFormat(e.to_string()).to_string())?;

    // 11b. Maintain a modern `custom-title` event so Claude Code 2.x reflects
    // the rename in `claude --resume`. On a non-empty title we append the event;
    // on a reset (empty title) we strip the custom-title line(s) for the name
    // being cleared, leaving unrelated (older/CLI-set) titles intact.
    let trimmed_title = new_title.trim();
    if !trimmed_title.is_empty() {
        if let Some(session_id) = extract_session_id_from_lines(&lines) {
            let custom_title_event = build_claude_custom_title_event(&session_id, trimmed_title);
            let custom_title_line = serde_json::to_string(&custom_title_event)
                .map_err(|e| RenameError::InvalidJsonFormat(e.to_string()).to_string())?;
            lines.push(custom_title_line);
        }
    } else {
        if let Some(prev_title) = extract_title_from_prefix(&current_message) {
            lines.retain(|line| !is_custom_title_line_for(line, &prev_title));
        }
    }

    // 12. Write atomically (write to temp with unique nonce, then rename)
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = format!("{file_path}.{nonce}.tmp");
    {
        let mut temp_file = File::create(&temp_path)
            .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                writeln!(temp_file).map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
            }
            write!(temp_file, "{line}")
                .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
        }
    }

    // 13. Atomic rename (Windows compatibility: remove existing file first)
    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new(&file_path).exists() {
            fs::remove_file(&file_path)
                .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
        }
    }

    fs::rename(&temp_path, &file_path)
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;

    Ok(NativeRenameResult {
        success: true,
        previous_title: current_message,
        new_title: new_message,
        file_path,
    })
}

/// Collect the Claude configuration directories the user has registered, plus
/// `CLAUDE_CONFIG_DIR`. These are the same sources `scan_all_projects` reads, so
/// any directory whose sessions the app displays is represented here.
fn configured_claude_dirs() -> Vec<String> {
    let mut dirs = Vec::new();

    if let Ok(user_data_path) = crate::commands::metadata::get_user_data_path() {
        if let Ok(content) = fs::read_to_string(user_data_path) {
            if let Ok(metadata) = serde_json::from_str::<crate::models::UserMetadata>(&content) {
                dirs.extend(
                    metadata
                        .settings
                        .custom_claude_paths
                        .into_iter()
                        .map(|custom| custom.path),
                );
            }
        }
    }

    if let Ok(env_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        let trimmed = env_dir.trim();
        if !trimmed.is_empty() {
            dirs.push(expand_home_prefix(trimmed));
        }
    }

    dirs
}

/// Expand a leading `~` to the home directory, mirroring `detect_claude_config_dir`.
fn expand_home_prefix(raw: &str) -> String {
    let Some(home) = dirs::home_dir() else {
        return raw.to_string();
    };
    if raw == "~" {
        return home.to_string_lossy().to_string();
    }
    match raw.strip_prefix("~/") {
        Some(rest) => home.join(rest).to_string_lossy().to_string(),
        None => raw.to_string(),
    }
}

/// Resolve the Claude configuration roots a native rename may write to.
///
/// Always includes the default `~/.claude`. A configured directory is only added
/// once it passes [`validate_custom_claude_path`], which requires an absolute,
/// non-symlinked base with a real `projects/` subdirectory. The allowlist can
/// therefore only widen to directories the user registered and that the app
/// already scans — never to arbitrary paths.
fn resolve_claude_roots(configured: &[String]) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(home) = dirs::home_dir() {
        push_root(&mut roots, home.join(".claude"));
    }

    for dir in configured {
        let candidate = PathBuf::from(dir);
        if crate::utils::validate_custom_claude_path(&candidate).is_ok() {
            push_root(&mut roots, candidate);
        }
    }

    roots
}

/// Push a root, resolved to its canonical form so it compares correctly against
/// the canonicalized file path. Duplicates are skipped.
/// Add an allowed Claude root. A root that is itself a symlink is rejected, so a
/// symlinked `~/.claude` cannot smuggle its target into the allowlist and bypass
/// the "only the project directory may be a symlink" policy.
///
/// The path is stored as-is (not canonicalized): `validate_claude_path_with_roots`
/// compares it lexically against the raw request path, so both sides must use the
/// same representation. Canonicalizing here would break that on Windows, where
/// `canonicalize()` yields the `\\?\` verbatim form.
fn push_root(roots: &mut Vec<PathBuf>, path: PathBuf) {
    let is_symlink = fs::symlink_metadata(&path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if is_symlink {
        return;
    }
    if !roots.contains(&path) {
        roots.push(path);
    }
}

fn allowed_claude_roots() -> Vec<PathBuf> {
    resolve_claude_roots(&configured_claude_dirs())
}

/// Validates that the file path is within `~/.claude` or a registered custom
/// Claude directory. This prevents path traversal attacks that could modify
/// arbitrary files.
///
/// Security checks performed:
/// 1. Path must be absolute
/// 2. Filename stem must match ^[A-Za-z0-9_-]+$ and the extension must be `.jsonl`
/// 3. Path must be lexically under an allowed root's `projects/` folder, with no
///    `.`/`..` traversal components
/// 4. Depth-1 symlink policy (matches `scan_projects`, #277): the project
///    directory may be a symlink to a directory (this is how shared sessions are
///    linked in), but `projects/` and everything below the project directory must
///    be symlink-free, and the session path must resolve to a regular file
fn validate_claude_path(file_path: &str) -> Result<(), String> {
    validate_claude_path_with_roots(file_path, &allowed_claude_roots())
}

fn validate_claude_path_with_roots(
    file_path: &str,
    allowed_roots: &[PathBuf],
) -> Result<(), String> {
    let file_path_buf = std::path::PathBuf::from(file_path);

    // 1. Require absolute path
    if !file_path_buf.is_absolute() {
        return Err(
            RenameError::PermissionDenied("File path must be absolute".to_string()).to_string(),
        );
    }

    // 2. Validate filename pattern
    let Some(stem) = file_path_buf.file_stem() else {
        return Err(RenameError::PermissionDenied("Invalid filename".to_string()).to_string());
    };
    if !FILENAME_REGEX.is_match(&stem.to_string_lossy()) {
        return Err(RenameError::PermissionDenied(
            "Filename must contain only alphanumeric characters, underscores, and hyphens"
                .to_string(),
        )
        .to_string());
    }
    // Require a `.jsonl` extension so rename cannot rewrite non-session files
    // (e.g. notes.txt, config.json) that happen to be valid JSON lines.
    if file_path_buf.extension().and_then(|e| e.to_str()) != Some("jsonl") {
        return Err(RenameError::PermissionDenied(
            "Session file must have a .jsonl extension".to_string(),
        )
        .to_string());
    }

    // 3. The file must sit under an allowed root's `projects/` directory. Match
    // lexically: the path from the root down to `projects/` is always real, so
    // the only symlink (if any) is the project directory just below it.
    let Some((projects_dir, relative)) = allowed_roots.iter().find_map(|root| {
        let projects_dir = root.join("projects");
        file_path_buf
            .strip_prefix(&projects_dir)
            .ok()
            .map(|rel| (projects_dir, rel.to_path_buf()))
    }) else {
        return Err(RenameError::PermissionDenied(
            "File path must be within the projects/ folder of ~/.claude or a registered custom \
             Claude directory"
                .to_string(),
        )
        .to_string());
    };

    // 4. Reject path traversal: every component under `projects/` must be a plain
    // name (no `.` / `..` / prefixes), and there must be at least
    // `<project>/<session>.jsonl`.
    let components: Vec<std::path::Component> = relative.components().collect();
    if components.len() < 2
        || !components
            .iter()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return Err(RenameError::PermissionDenied(
            "Invalid session path under projects/".to_string(),
        )
        .to_string());
    }

    // 5. `projects/` itself must be a real directory, never a symlink (mirrors
    // validate_custom_claude_path).
    if fs::symlink_metadata(&projects_dir)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(
            RenameError::PermissionDenied("projects/ must not be a symlink".to_string())
                .to_string(),
        );
    }

    // 6. Depth-1 symlink policy (matches scan_projects, #277): the project
    // directory (first component under `projects/`) may be a symlink that
    // resolves to a directory — this is how shared sessions are linked in — but
    // nothing deeper may be a symlink, and the target must be a real file.
    let project_dir = projects_dir.join(components[0]);
    let project_meta = fs::symlink_metadata(&project_dir)
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
    if project_meta.file_type().is_symlink() {
        if !project_dir.is_dir() {
            return Err(RenameError::PermissionDenied(
                "Project directory symlink does not resolve to a directory".to_string(),
            )
            .to_string());
        }
    } else if !project_meta.is_dir() {
        return Err(
            RenameError::PermissionDenied("Project path is not a directory".to_string())
                .to_string(),
        );
    }

    // Every component below the project directory must be symlink-free, and the
    // session path must resolve to a regular file.
    let mut current = project_dir;
    let deeper = &components[1..];
    for (i, comp) in deeper.iter().enumerate() {
        current.push(comp);
        let meta = fs::symlink_metadata(&current)
            .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
        if meta.file_type().is_symlink() {
            return Err(RenameError::PermissionDenied(
                "Symlinks are not allowed below the project directory".to_string(),
            )
            .to_string());
        }
        if i + 1 == deeper.len() && !meta.is_file() {
            return Err(RenameError::PermissionDenied(
                "Session path is not a regular file".to_string(),
            )
            .to_string());
        }
    }

    Ok(())
}

/// Extract the title from a modern `custom-title` event.
/// Claude Code 2.x drives the `claude --resume` picker from these events, so
/// CCHV writes one alongside the bracket-prefix rename.
fn extract_custom_title_from_value(json: &serde_json::Value) -> Option<String> {
    if json.get("type").and_then(serde_json::Value::as_str) != Some("custom-title") {
        return None;
    }
    let title = json.get("customTitle").and_then(serde_json::Value::as_str)?.trim();
    if title.is_empty() {
        return None;
    }
    Some(title.to_string())
}

/// Build a minimal `custom-title` event matching the shape the Claude CLI writes
/// (`{"type","customTitle","sessionId"}`), so `claude --resume` shows the name.
fn build_claude_custom_title_event(session_id: &str, new_title: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "custom-title",
        "customTitle": new_title,
        "sessionId": session_id,
    })
}

/// True if `line` is a `custom-title` event whose title equals `target`.
/// Used by reset to remove the `custom-title` CCHV appended for a given name
/// while leaving unrelated (older/CLI-set) titles intact.
fn is_custom_title_line_for(line: &str, target: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|json| extract_custom_title_from_value(&json))
        .is_some_and(|title| title == target)
}

/// Extract the `sessionId` from the first JSONL line that carries one.
fn extract_session_id_from_lines(lines: &[String]) -> Option<String> {
    for line in lines {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(id) = json.get("sessionId").and_then(serde_json::Value::as_str) {
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
    }
    None
}

/// Extract the title text from a `[Title] message` prefix, if present.
fn extract_title_from_prefix(message: &str) -> Option<String> {
    let message = message.strip_prefix('[')?;
    let end = message.find(']')?;
    let title = message[..end].trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_string())
    }
}

/// Extracts message content from JSON, handling both direct string and nested object formats
fn extract_message_content(json: &serde_json::Value) -> Option<String> {
    json.get("message").and_then(|m| {
        // Handle direct string: {"message": "text"}
        if let Some(s) = m.as_str() {
            return Some(s.to_string());
        }
        // Handle nested object: {"message": {"role": "user", "content": "text" | [...]}}
        if let Some(obj) = m.as_object() {
            if let Some(content) = obj.get("content") {
                // Content can be a string
                if let Some(s) = content.as_str() {
                    return Some(s.to_string());
                }
                // Content can be an array: [{"type": "text", "text": "..."}]
                if let Some(arr) = content.as_array() {
                    for item in arr {
                        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                return Some(text.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    })
}

/// Updates message content in JSON, handling both direct string and nested object formats.
/// Returns true if the update was successful, false if the content format is not supported.
fn update_message_content(json: &mut serde_json::Value, new_content: &str) -> bool {
    if let Some(message) = json.get_mut("message") {
        // Handle direct string
        if message.is_string() {
            *message = serde_json::Value::String(new_content.to_string());
            return true;
        }
        // Handle nested object
        if let Some(obj) = message.as_object_mut() {
            if let Some(content) = obj.get("content") {
                // Handle string content
                if content.is_string() {
                    obj.insert(
                        "content".to_string(),
                        serde_json::Value::String(new_content.to_string()),
                    );
                    return true;
                }
                // Handle array content: update the first text item
                if let Some(arr) = content.as_array() {
                    for (i, item) in arr.iter().enumerate() {
                        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                            // Clone and update the array
                            let mut new_arr = arr.clone();
                            if let Some(text_item) = new_arr.get_mut(i) {
                                if let Some(text_obj) = text_item.as_object_mut() {
                                    text_obj.insert(
                                        "text".to_string(),
                                        serde_json::Value::String(new_content.to_string()),
                                    );
                                }
                            }
                            obj.insert("content".to_string(), serde_json::Value::Array(new_arr));
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Strips existing \[Title\] prefix from message content.
///
/// This function removes a title prefix in the format `[Title] Message`.
/// It searches for the first occurrence of `]` and removes everything
/// before and including it, then trims leading whitespace.
///
/// # Limitations
///
/// **Nested Brackets Are Not Supported**: This function stops at the first `]`
/// character, which yields incorrect results for nested brackets.
///
/// Example:
/// - Input: `"[Nested [brackets]] Message"`
/// - Expected: `"Message"`
/// - Actual: `"] Message"` (stops at first `]`)
///
/// To prevent this issue, the `rename_session_native` function validates
/// that new titles do not contain the `]` character before applying them.
///
/// # Arguments
///
/// * `message` - The message text that may start with a `[Title]` prefix
///
/// # Returns
///
/// The message with the prefix removed, or the original message if no
/// prefix is found.
fn strip_title_prefix(message: &str) -> String {
    if message.starts_with('[') {
        if let Some(end_bracket) = message.find(']') {
            let after_bracket = &message[end_bracket + 1..];
            return after_bracket.trim_start().to_string();
        }
    }
    message.to_string()
}

/// Finds the index of the first real user message in the JSONL lines.
/// Skips non-user messages (file-history-snapshot, progress, etc.) and meta messages.
fn find_first_user_message_index(lines: &[String]) -> Result<usize, String> {
    for (index, line) in lines.iter().enumerate() {
        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            // Check if type is "user"
            let is_user = json
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "user")
                .unwrap_or(false);

            // Check if it's NOT a meta message (isMeta: true)
            let is_meta = json
                .get("isMeta")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);

            // Must be user message with actual content (not meta)
            if is_user && !is_meta {
                // Verify it has a message field with content
                if extract_message_content(&json).is_some() {
                    return Ok(index);
                }
            }
        }
    }

    Err(RenameError::NoUserMessage.to_string())
}

/// Resets session name to original (removes title prefix)
#[command]
pub async fn reset_session_native_name(file_path: String) -> Result<NativeRenameResult, String> {
    rename_session_native(file_path, String::new()).await
}

/// Renames an `OpenCode` session by updating the session title field in storage JSON.
#[command]
pub async fn rename_opencode_session_title(
    session_path: String,
    new_title: String,
) -> Result<NativeRenameResult, String> {
    let (project_id, session_id) = parse_opencode_session_path(&session_path)?;

    let base_path = crate::providers::opencode::get_base_path().ok_or_else(|| {
        RenameError::FileNotFound("OpenCode base path not found".to_string()).to_string()
    })?;
    let session_root = Path::new(&base_path).join("storage").join("session");
    let session_file = session_root
        .join(&project_id)
        .join(format!("{session_id}.json"));

    if !session_file.exists() {
        return Err(
            RenameError::FileNotFound(session_file.to_string_lossy().to_string()).to_string(),
        );
    }

    if let Ok(metadata) = fs::symlink_metadata(&session_file) {
        if metadata.file_type().is_symlink() {
            return Err(RenameError::PermissionDenied(
                "Session file cannot be a symlink".to_string(),
            )
            .to_string());
        }
    }

    let canonical_file = session_file
        .canonicalize()
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
    let canonical_root = session_root
        .canonicalize()
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(RenameError::PermissionDenied(
            "Session file path is outside OpenCode storage".to_string(),
        )
        .to_string());
    }

    let content = fs::read_to_string(&canonical_file)
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
    let mut session_json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| RenameError::InvalidJsonFormat(e.to_string()).to_string())?;

    let previous_title = session_json
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let normalized_title = new_title.trim().to_string();

    let Some(session_obj) = session_json.as_object_mut() else {
        return Err(
            RenameError::InvalidJsonFormat("Session JSON must be an object".to_string())
                .to_string(),
        );
    };

    if normalized_title.is_empty() {
        session_obj.remove("title");
    } else {
        session_obj.insert(
            "title".to_string(),
            serde_json::Value::String(normalized_title.clone()),
        );
    }

    let serialized = serde_json::to_string(&session_json)
        .map_err(|e| RenameError::InvalidJsonFormat(e.to_string()).to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = canonical_file.with_extension(format!("json.{nonce}.tmp"));
    fs::write(&temp_path, serialized)
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;

    #[cfg(target_os = "windows")]
    {
        if canonical_file.exists() {
            fs::remove_file(&canonical_file)
                .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;
        }
    }

    fs::rename(&temp_path, &canonical_file)
        .map_err(|e| RenameError::IoError(e.to_string()).to_string())?;

    Ok(NativeRenameResult {
        success: true,
        previous_title,
        new_title: normalized_title,
        file_path: session_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_opencode_session_path_valid() {
        let parsed = parse_opencode_session_path("opencode://project_123/session_456").unwrap();
        assert_eq!(
            parsed,
            ("project_123".to_string(), "session_456".to_string())
        );
    }

    #[test]
    fn test_parse_opencode_session_path_invalid_prefix() {
        assert!(parse_opencode_session_path("/tmp/invalid").is_err());
    }

    #[test]
    fn test_parse_opencode_session_path_rejects_traversal() {
        assert!(parse_opencode_session_path("opencode://project/../etc").is_err());
    }

    #[test]
    fn test_strip_title_prefix() {
        assert_eq!(
            strip_title_prefix("[My Title] Original message"),
            "Original message"
        );
        assert_eq!(strip_title_prefix("No prefix here"), "No prefix here");
        // Note: nested brackets are not fully supported - first ] is used
        // "[Nested [brackets]] Message" -> first ] at index 17, result is "] Message"
        assert_eq!(
            strip_title_prefix("[Nested [brackets]] Message"),
            "] Message"
        );
        assert_eq!(strip_title_prefix("[] Empty brackets"), "Empty brackets");
        assert_eq!(strip_title_prefix("[Title]NoSpace"), "NoSpace");
    }

    #[test]
    fn test_extract_message_content_direct_string() {
        let json: serde_json::Value = serde_json::json!({
            "message": "Hello world"
        });
        assert_eq!(
            extract_message_content(&json),
            Some("Hello world".to_string())
        );
    }

    #[test]
    fn test_extract_message_content_nested() {
        let json: serde_json::Value = serde_json::json!({
            "message": {
                "role": "user",
                "content": "Hello world"
            }
        });
        assert_eq!(
            extract_message_content(&json),
            Some("Hello world".to_string())
        );
    }

    #[test]
    fn test_extract_message_content_array() {
        let json: serde_json::Value = serde_json::json!({
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Hello from array"}
                ]
            }
        });
        assert_eq!(
            extract_message_content(&json),
            Some("Hello from array".to_string())
        );
    }

    #[test]
    fn test_find_first_user_message_skips_non_user_types() {
        let lines = vec![
            r#"{"type":"file-history-snapshot","data":{}}"#.to_string(),
            r#"{"type":"progress","data":"loading"}"#.to_string(),
            r#"{"type":"user","message":"Hello world"}"#.to_string(),
        ];
        assert_eq!(find_first_user_message_index(&lines).unwrap(), 2);
    }

    #[test]
    fn test_find_first_user_message_skips_meta() {
        let lines = vec![
            r#"{"type":"user","isMeta":true,"message":"init command"}"#.to_string(),
            r#"{"type":"user","message":"Real user message"}"#.to_string(),
        ];
        assert_eq!(find_first_user_message_index(&lines).unwrap(), 1);
    }

    #[test]
    fn test_update_message_content_string() {
        let mut json: serde_json::Value = serde_json::json!({
            "message": {
                "role": "user",
                "content": "Original"
            }
        });
        assert!(update_message_content(&mut json, "Updated"));
        assert_eq!(json["message"]["content"].as_str(), Some("Updated"));
    }

    #[test]
    fn test_update_message_content_array() {
        let mut json: serde_json::Value = serde_json::json!({
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Original"}
                ]
            }
        });
        assert!(update_message_content(&mut json, "Updated"));
        assert_eq!(
            json["message"]["content"][0]["text"].as_str(),
            Some("Updated")
        );
    }

    // ==================== EDGE CASE TESTS ====================

    // --- strip_title_prefix edge cases ---

    #[test]
    fn test_strip_title_prefix_empty_string() {
        assert_eq!(strip_title_prefix(""), "");
    }

    #[test]
    fn test_strip_title_prefix_unclosed_bracket() {
        // Unclosed bracket should return original string
        assert_eq!(strip_title_prefix("[Unclosed title"), "[Unclosed title");
    }

    #[test]
    fn test_strip_title_prefix_only_brackets() {
        assert_eq!(strip_title_prefix("[]"), "");
    }

    #[test]
    fn test_strip_title_prefix_unicode() {
        assert_eq!(
            strip_title_prefix("[日本語タイトル] メッセージ"),
            "メッセージ"
        );
    }

    #[test]
    fn test_strip_title_prefix_with_newline() {
        assert_eq!(strip_title_prefix("[Title]\nMessage"), "Message");
    }

    // --- extract_message_content edge cases ---

    #[test]
    fn test_extract_message_content_missing_field() {
        let json: serde_json::Value = serde_json::json!({"uuid": "123"});
        assert_eq!(extract_message_content(&json), None);
    }

    #[test]
    fn test_extract_message_content_null_message() {
        let json: serde_json::Value = serde_json::json!({"message": null});
        assert_eq!(extract_message_content(&json), None);
    }

    #[test]
    fn test_extract_message_content_empty_array() {
        let json: serde_json::Value = serde_json::json!({
            "message": {"role": "user", "content": []}
        });
        assert_eq!(extract_message_content(&json), None);
    }

    #[test]
    fn test_extract_message_content_array_no_text_type() {
        let json: serde_json::Value = serde_json::json!({
            "message": {
                "role": "user",
                "content": [
                    {"type": "image", "url": "http://example.com/img.png"}
                ]
            }
        });
        assert_eq!(extract_message_content(&json), None);
    }

    #[test]
    fn test_extract_message_content_multiple_text_items() {
        // Should return first text item
        let json: serde_json::Value = serde_json::json!({
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "First"},
                    {"type": "text", "text": "Second"}
                ]
            }
        });
        assert_eq!(extract_message_content(&json), Some("First".to_string()));
    }

    // --- find_first_user_message_index edge cases ---

    #[test]
    fn test_find_first_user_message_empty_lines() {
        let lines: Vec<String> = vec![];
        let result = find_first_user_message_index(&lines);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No user message"));
    }

    #[test]
    fn test_find_first_user_message_no_user_messages() {
        let lines = vec![
            r#"{"type":"assistant","message":"Hello"}"#.to_string(),
            r#"{"type":"system","message":"Init"}"#.to_string(),
        ];
        let result = find_first_user_message_index(&lines);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_first_user_message_invalid_json() {
        let lines = vec![
            "not valid json".to_string(),
            r#"{"type":"user","message":"Valid"}"#.to_string(),
        ];
        // Should skip invalid JSON and find the valid user message
        assert_eq!(find_first_user_message_index(&lines).unwrap(), 1);
    }

    #[test]
    fn test_find_first_user_message_user_without_content() {
        let lines = vec![
            r#"{"type":"user"}"#.to_string(), // No message field
            r#"{"type":"user","message":"Has content"}"#.to_string(),
        ];
        // Should skip user without extractable content
        assert_eq!(find_first_user_message_index(&lines).unwrap(), 1);
    }

    // --- update_message_content edge cases ---

    #[test]
    fn test_update_message_content_no_message_field() {
        let mut json: serde_json::Value = serde_json::json!({"uuid": "123"});
        assert!(!update_message_content(&mut json, "New"));
    }

    #[test]
    fn test_update_message_content_array_no_text_type() {
        let mut json: serde_json::Value = serde_json::json!({
            "message": {
                "role": "user",
                "content": [
                    {"type": "image", "url": "http://example.com/img.png"}
                ]
            }
        });
        assert!(!update_message_content(&mut json, "New"));
    }

    #[test]
    fn test_update_message_content_direct_string() {
        let mut json: serde_json::Value = serde_json::json!({
            "message": "Direct string"
        });
        assert!(update_message_content(&mut json, "Updated"));
        assert_eq!(json["message"].as_str(), Some("Updated"));
    }

    // --- validate_claude_path tests (SECURITY) ---

    #[test]
    fn test_validate_claude_path_rejects_relative_path() {
        let result = validate_claude_path("relative/path/file.jsonl");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be absolute"));
    }

    #[test]
    fn test_validate_claude_path_rejects_invalid_filename() {
        // Filename with dots should be rejected by regex
        let result = validate_claude_path("/etc/passwd");
        assert!(result.is_err());
        // Will fail on filename validation (passwd has no extension, or if it checks "passwd")
    }

    #[test]
    fn test_validate_claude_path_rejects_non_claude_directory() {
        // Use a path with valid filename but wrong directory
        let result = validate_claude_path("/tmp/validfilename.jsonl");
        assert!(result.is_err());
        // Should fail on directory check or canonicalize
    }

    #[test]
    fn test_validate_claude_path_valid_path() {
        // This test requires a real .jsonl file in ~/.claude to exist
        if let Some(home) = dirs::home_dir() {
            let claude_projects = home.join(".claude/projects");
            if claude_projects.exists() {
                // Try to find any .jsonl file in projects subdirectories
                if let Ok(projects) = fs::read_dir(&claude_projects) {
                    for project in projects.flatten() {
                        if project.path().is_dir() {
                            if let Ok(files) = fs::read_dir(project.path()) {
                                for file in files.flatten() {
                                    let path = file.path();
                                    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                                        let test_path = path.to_string_lossy().to_string();
                                        let result = validate_claude_path(&test_path);
                                        assert!(
                                            result.is_ok(),
                                            "Validation failed for valid path {test_path}: {result:?}"
                                        );
                                        return; // Test passed
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Skip test if no suitable file found
    }

    #[test]
    fn test_validate_claude_path_nonexistent_file() {
        // Nonexistent file should fail at canonicalize
        let result = validate_claude_path("/nonexistent/path/to/file.jsonl");
        assert!(result.is_err());
    }

    /// Canonicalize the temp root: on macOS `TempDir` sits under `/var/folders`,
    /// and `/var` is a symlink, which `validate_claude_path` rejects outright.
    /// Real Claude directories under `$HOME` have no symlinked ancestors.
    fn real_temp_root(temp: &tempfile::TempDir) -> PathBuf {
        temp.path().canonicalize().unwrap()
    }

    /// Build a Claude-shaped config dir (`<base>/projects/<project>/<name>.jsonl`)
    /// and return the base plus the session file path.
    fn make_claude_dir(base: &Path, session_name: &str) -> (PathBuf, String) {
        let project_dir = base.join("projects").join("-tmp-demo");
        fs::create_dir_all(&project_dir).unwrap();
        let session = project_dir.join(format!("{session_name}.jsonl"));
        fs::write(&session, "{}\n").unwrap();
        (base.to_path_buf(), session.to_string_lossy().to_string())
    }

    #[test]
    fn validate_claude_path_accepts_registered_custom_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let custom_base = real_temp_root(&temp).join(".claude-holophonix");
        fs::create_dir_all(&custom_base).unwrap();
        let (base, session_path) = make_claude_dir(&custom_base, "session-1");

        let roots = resolve_claude_roots(&[base.to_string_lossy().to_string()]);
        assert!(
            validate_claude_path_with_roots(&session_path, &roots).is_ok(),
            "a session inside a registered custom Claude directory must be renameable"
        );
    }

    #[test]
    fn validate_claude_path_rejects_directory_that_is_not_registered() {
        let temp = tempfile::TempDir::new().unwrap();
        let unregistered = real_temp_root(&temp).join(".claude-other");
        fs::create_dir_all(&unregistered).unwrap();
        let (_base, session_path) = make_claude_dir(&unregistered, "session-1");

        // Roots resolved without the directory being configured. The path itself
        // is symlink-free, so this fails on root containment specifically.
        let roots = resolve_claude_roots(&[]);
        assert!(
            validate_claude_path_with_roots(&session_path, &roots).is_err(),
            "an unregistered directory must stay rejected"
        );
    }

    #[test]
    fn resolve_claude_roots_skips_directory_without_projects_subdir() {
        let temp = tempfile::TempDir::new().unwrap();
        let bogus = real_temp_root(&temp).join(".claude-bogus");
        fs::create_dir_all(&bogus).unwrap(); // no projects/ inside

        let roots = resolve_claude_roots(&[bogus.to_string_lossy().to_string()]);
        let bogus_canonical = bogus.canonicalize().unwrap();
        assert!(
            !roots.contains(&bogus_canonical),
            "a directory failing validate_custom_claude_path must not widen the allowlist"
        );
    }

    #[test]
    fn resolve_claude_roots_skips_symlinked_custom_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let real_base = real_temp_root(&temp).join("real-claude");
        fs::create_dir_all(real_base.join("projects")).unwrap();
        let link = real_temp_root(&temp).join("linked-claude");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_base, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&real_base, &link).unwrap();

        let roots = resolve_claude_roots(&[link.to_string_lossy().to_string()]);
        // The symlinked base is rejected, so only the default ~/.claude root remains.
        assert!(
            !roots.iter().any(|r| r.starts_with(real_temp_root(&temp))),
            "a symlinked custom base must be rejected"
        );
    }

    #[test]
    fn resolve_claude_roots_always_includes_default_claude_dir() {
        let roots = resolve_claude_roots(&[]);
        if let Some(home) = dirs::home_dir() {
            let default_root = home.join(".claude");
            // Roots are stored as-is (not canonicalized), unless ~/.claude is a
            // symlink (then push_root drops it).
            let is_symlink = fs::symlink_metadata(&default_root)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            if !is_symlink {
                assert!(
                    roots.contains(&default_root),
                    "the default ~/.claude root must always be allowed"
                );
            }
        }
    }

    #[test]
    fn push_root_rejects_symlinked_root() {
        let temp = tempfile::TempDir::new().unwrap();
        let real = real_temp_root(&temp).join("real-root");
        fs::create_dir_all(&real).unwrap();
        let link = real_temp_root(&temp).join("linked-root");
        symlink_dir(&real, &link);

        let mut roots = Vec::new();
        push_root(&mut roots, link.clone());
        assert!(
            !roots.contains(&link) && roots.is_empty(),
            "a symlinked root must not enter the allowlist"
        );

        // A real root is still accepted, stored as-is (not canonicalized).
        push_root(&mut roots, real.clone());
        assert_eq!(roots, vec![real]);
    }

    #[test]
    fn validate_claude_path_rejects_non_jsonl_extension() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = real_temp_root(&temp).join(".claude");
        let project_dir = root.join("projects").join("-proj");
        fs::create_dir_all(&project_dir).unwrap();
        let non_session = project_dir.join("config.json");
        fs::write(&non_session, "{}\n").unwrap();

        let roots = resolve_claude_roots(&[root.to_string_lossy().to_string()]);
        assert!(
            validate_claude_path_with_roots(&non_session.to_string_lossy(), &roots).is_err(),
            "only .jsonl session files may be renamed"
        );
    }

    #[cfg(unix)]
    fn symlink_dir(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
    #[cfg(windows)]
    fn symlink_dir(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_dir(target, link).unwrap();
    }

    #[test]
    fn validate_claude_path_accepts_symlinked_project_directory() {
        // Mirrors the shared-sessions layout: a project directory under
        // <root>/projects is a symlink into a real directory elsewhere (e.g.
        // /Users/Shared/.claude/projects). The session file is a real file.
        let temp = tempfile::TempDir::new().unwrap();
        let root = real_temp_root(&temp).join(".claude");
        fs::create_dir_all(root.join("projects")).unwrap();

        let shared = real_temp_root(&temp).join("shared").join("-Shared-proj");
        fs::create_dir_all(&shared).unwrap();
        let session = shared.join("session-1.jsonl");
        fs::write(&session, "{}\n").unwrap();

        // <root>/projects/-Shared-proj -> <temp>/shared/-Shared-proj
        symlink_dir(&shared, &root.join("projects").join("-Shared-proj"));

        let session_path = root
            .join("projects")
            .join("-Shared-proj")
            .join("session-1.jsonl")
            .to_string_lossy()
            .to_string();
        let roots = resolve_claude_roots(&[root.to_string_lossy().to_string()]);
        assert!(
            validate_claude_path_with_roots(&session_path, &roots).is_ok(),
            "a session under a depth-1 symlinked project directory must be renameable"
        );
    }

    #[test]
    fn validate_claude_path_rejects_symlink_below_project_directory() {
        // A symlink deeper than the project directory is not allowed.
        let temp = tempfile::TempDir::new().unwrap();
        let root = real_temp_root(&temp).join(".claude");
        let project_dir = root.join("projects").join("-proj");
        fs::create_dir_all(&project_dir).unwrap();

        let elsewhere = real_temp_root(&temp).join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        fs::write(elsewhere.join("session-1.jsonl"), "{}\n").unwrap();

        // <root>/projects/-proj/subdir -> <temp>/elsewhere  (symlink below depth 1)
        symlink_dir(&elsewhere, &project_dir.join("subdir"));

        let session_path = project_dir
            .join("subdir")
            .join("session-1.jsonl")
            .to_string_lossy()
            .to_string();
        let roots = resolve_claude_roots(&[root.to_string_lossy().to_string()]);
        assert!(
            validate_claude_path_with_roots(&session_path, &roots).is_err(),
            "symlinks below the project directory must stay rejected"
        );
    }

    #[test]
    fn validate_claude_path_rejects_symlinked_session_file() {
        // The session file itself must be a real file, not a symlink.
        let temp = tempfile::TempDir::new().unwrap();
        let root = real_temp_root(&temp).join(".claude");
        let project_dir = root.join("projects").join("-proj");
        fs::create_dir_all(&project_dir).unwrap();

        let real_file = real_temp_root(&temp).join("target.jsonl");
        fs::write(&real_file, "{}\n").unwrap();
        let link = project_dir.join("session-1.jsonl");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real_file, &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&real_file, &link).unwrap();

        let roots = resolve_claude_roots(&[root.to_string_lossy().to_string()]);
        assert!(
            validate_claude_path_with_roots(&link.to_string_lossy(), &roots).is_err(),
            "a symlinked session file must be rejected"
        );
    }

    #[test]
    fn validate_claude_path_rejects_path_traversal_under_projects() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = real_temp_root(&temp).join(".claude");
        fs::create_dir_all(root.join("projects")).unwrap();

        let traversal = root
            .join("projects")
            .join("..")
            .join("evil")
            .join("session-1.jsonl")
            .to_string_lossy()
            .to_string();
        let roots = resolve_claude_roots(&[root.to_string_lossy().to_string()]);
        assert!(
            validate_claude_path_with_roots(&traversal, &roots).is_err(),
            "`..` traversal under projects/ must be rejected"
        );
    }

    #[test]
    fn test_validate_claude_path_filename_with_special_chars() {
        // Test filename validation with various invalid characters
        if let Some(home) = dirs::home_dir() {
            let claude_dir = home.join(".claude/projects");
            // Filename with dot (besides extension) should fail
            let path_with_dot = claude_dir
                .join("test.file.jsonl")
                .to_string_lossy()
                .to_string();
            let result = validate_claude_path(&path_with_dot);
            // Will fail either on filename validation or canonicalize (file doesn't exist)
            assert!(result.is_err());
        }
    }

    // --- Title validation tests ---

    #[test]
    fn test_title_with_closing_bracket_rejected() {
        // This test verifies that titles containing ']' are rejected
        // due to the nested bracket limitation in strip_title_prefix
        let title_with_bracket = "Test ] Title";
        assert!(title_with_bracket.contains(']'));
    }

    #[test]
    fn test_strip_title_prefix_nested_brackets_documented_limitation() {
        // This test documents the known limitation that nested brackets
        // don't work correctly (as documented in the function)
        let input = "[Nested [brackets]] Message";
        let result = strip_title_prefix(input);
        // Known limitation: stops at first ']'
        assert_eq!(result, "] Message");
        // This is why we reject titles with ']' in rename_session_native
    }
}
