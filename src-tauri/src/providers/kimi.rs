use crate::models::{ClaudeMessage, ClaudeProject, ClaudeSession, TokenUsage};
use crate::providers::ProviderInfo;
use crate::utils::build_provider_message;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Detect Kimi CLI installation
pub fn detect() -> Option<ProviderInfo> {
    let base = get_base_path()?;
    let sessions_path = PathBuf::from(&base).join("sessions");
    Some(ProviderInfo {
        id: "kimi".to_string(),
        display_name: "Kimi CLI".to_string(),
        base_path: base,
        is_available: sessions_path.is_dir(),
    })
}

/// Get the base path for Kimi CLI data (~/.kimi)
pub fn get_base_path() -> Option<String> {
    if let Ok(val) = std::env::var("KIMI_HOME") {
        let p = PathBuf::from(&val);
        if p.is_dir() {
            return Some(val);
        }
    }
    dirs::home_dir().map(|h| h.join(".kimi").to_string_lossy().to_string())
}

/// Scan for all Kimi CLI projects (sessions grouped by actual session directory)
pub fn scan_projects() -> Result<Vec<ClaudeProject>, String> {
    let base = get_base_path().ok_or("Could not determine Kimi base path")?;
    let base_path = PathBuf::from(&base);
    let sessions_path = base_path.join("sessions");

    if !sessions_path.is_dir() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    for session_dir_entry in fs::read_dir(&sessions_path)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        if session_dir_entry
            .file_type()
            .map(|ft| ft.is_symlink())
            .unwrap_or(true)
        {
            continue;
        }

        let session_dir = session_dir_entry.path();
        if !session_dir.is_dir() {
            continue;
        }

        // Find the actual conversation session within the session directory
        let mut found_sessions = Vec::new();
        if let Ok(entries) = fs::read_dir(&session_dir) {
            for entry in entries.flatten() {
                let subdir = entry.path();
                if subdir.is_dir() {
                    // Check for wire.jsonl in subdirectory
                    let wire_path = subdir.join("wire.jsonl");
                    if wire_path.is_file() {
                        found_sessions.push((subdir, wire_path));
                    }
                }
            }
        }

        if found_sessions.is_empty() {
            continue;
        }

        // Use the first session found for metadata (they all belong to same project)
        let (session_subdir, wire_path) = &found_sessions[0];

        // Get session metadata from wire.jsonl
        let (message_count, last_modified, _summary) = extract_session_metadata(wire_path);

        // Try to get a better project name from state.json in the session subdir
        let project_name = get_project_name_from_state(session_subdir).unwrap_or_else(|| {
            session_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        let actual_path = session_subdir.to_string_lossy().to_string();

        projects.push(ClaudeProject {
            name: project_name.clone(),
            path: format!("kimi://{}", session_dir.to_string_lossy()),
            actual_path,
            session_count: found_sessions.len(),
            message_count,
            last_modified,
            git_info: None,
            provider: Some("kimi".to_string()),
            storage_type: Some("jsonl".to_string()),
            custom_directory_label: None,
        });
    }

    Ok(projects)
}

/// Load sessions for a Kimi project
pub fn load_sessions(
    project_path: &str,
    _exclude_sidechain: bool,
) -> Result<Vec<ClaudeSession>, String> {
    let dir = project_path.strip_prefix("kimi://").unwrap_or(project_path);

    let sessions_dir = PathBuf::from(dir);
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let project_name = sessions_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut sessions = Vec::new();

    for subdir_entry in fs::read_dir(&sessions_dir)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let subdir = subdir_entry.path();
        if !subdir.is_dir() {
            continue;
        }

        let wire_path = subdir.join("wire.jsonl");
        if !wire_path.is_file() {
            continue;
        }

        let (message_count, last_modified, summary) = extract_session_metadata(&wire_path);

        let session_id = subdir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        sessions.push(ClaudeSession {
            session_id: format!("kimi://{}", subdir.to_string_lossy()),
            actual_session_id: session_id.clone(),
            file_path: wire_path.to_string_lossy().to_string(),
            project_name: project_name.clone(),
            message_count,
            first_message_time: last_modified.clone(),
            last_message_time: last_modified.clone(),
            last_modified,
            has_tool_use: true,
            has_errors: false,
            summary,
            is_renamed: false,
            provider: Some("kimi".to_string()),
            storage_type: Some("jsonl".to_string()),
            entrypoint: None,
        });
    }

    sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(sessions)
}

/// Load messages from a Kimi session wire.jsonl file
pub fn load_messages(session_path: &str) -> Result<Vec<ClaudeMessage>, String> {
    let path = session_path.strip_prefix("kimi://").unwrap_or(session_path);

    let wire_path = PathBuf::from(path);

    if !wire_path.is_file() {
        return Err(format!("Session file not found: {session_path}"));
    }

    let data =
        fs::read_to_string(&wire_path).map_err(|e| format!("Failed to read session file: {e}"))?;

    let session_id = wire_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let session_dir = wire_path.parent().and_then(|p| p.parent());
    let wire_dir = wire_path.parent();
    let _project_name = session_dir
        .and_then(get_project_name_from_state)
        .unwrap_or_else(|| {
            session_dir
                .and_then(|d| d.file_name())
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let mut messages = Vec::new();
    let mut counter = 0u64;
    let mut assistant_msg_indices_per_turn: Vec<Vec<usize>> = Vec::new();
    let mut current_turn_assistant: Vec<usize> = Vec::new();

    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let raw: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = raw
            .get("message")
            .and_then(|m| m.get("type"))
            .and_then(Value::as_str);

        match msg_type {
            Some("metadata") => {}
            Some("TurnBegin" | "StepBegin" | "TurnEnd" | "StepEnd") => {
                // These are event markers, create a message for important ones
                if msg_type == Some("TurnBegin") {
                    if !current_turn_assistant.is_empty() {
                        assistant_msg_indices_per_turn.push(current_turn_assistant.clone());
                        current_turn_assistant.clear();
                    }
                    let timestamp = raw.get("timestamp").and_then(Value::as_f64);
                    counter += 1;
                    let uuid = format!("kimi-{counter}");
                    let ts = timestamp_to_rfc3339(timestamp);

                    let user_input = raw
                        .get("message")
                        .and_then(|m| m.get("payload"))
                        .and_then(|p| p.get("user_input"))
                        .and_then(Value::as_array)
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    if item.get("type").and_then(Value::as_str) == Some("text") {
                                        item.get("text").and_then(Value::as_str).map(String::from)
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default();

                    if !user_input.is_empty() {
                        let content = serde_json::json!([{
                            "type": "text",
                            "text": user_input
                        }]);
                        messages.push(build_provider_message(
                            "kimi",
                            uuid,
                            &session_id,
                            ts,
                            "user",
                            Some("user"),
                            Some(content),
                            None,
                        ));
                    }
                }
            }
            Some("ToolCall") => {
                let timestamp = raw.get("timestamp").and_then(Value::as_f64);
                counter += 1;
                let uuid = format!("kimi-{counter}");
                let ts = timestamp_to_rfc3339(timestamp);

                let payload = raw.get("message").and_then(|m| m.get("payload"));

                let func_name = payload
                    .and_then(|p| p.get("function"))
                    .and_then(|f| f.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");

                let tool_id = payload
                    .and_then(|p| p.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or(uuid.as_str())
                    .to_string();

                let arguments_str = payload
                    .and_then(|p| p.get("function"))
                    .and_then(|f| f.get("arguments"))
                    .and_then(Value::as_str)
                    .unwrap_or("{}");

                let arguments: Value = serde_json::from_str(arguments_str).unwrap_or(Value::Null);

                let content = serde_json::json!([{
                    "type": "tool_use",
                    "id": tool_id,
                    "name": map_kimi_tool_name(func_name),
                    "input": arguments
                }]);

                messages.push(build_provider_message(
                    "kimi",
                    uuid,
                    &session_id,
                    ts,
                    "assistant",
                    Some("assistant"),
                    Some(content),
                    None,
                ));
                current_turn_assistant.push(messages.len() - 1);
            }
            Some("ToolResult") => {
                let timestamp = raw.get("timestamp").and_then(Value::as_f64);
                counter += 1;
                let uuid = format!("kimi-{counter}");
                let ts = timestamp_to_rfc3339(timestamp);

                let payload = raw.get("message").and_then(|m| m.get("payload"));

                let tool_call_id = payload
                    .and_then(|p| p.get("tool_call_id"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();

                let return_value = payload
                    .and_then(|p| p.get("return_value"))
                    .unwrap_or(&Value::Null);

                let is_error = return_value
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                let output = return_value
                    .get("output")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let mut content_block = serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_call_id,
                    "content": output
                });
                if is_error {
                    content_block["is_error"] = Value::Bool(true);
                }

                let content = serde_json::json!([content_block]);

                messages.push(build_provider_message(
                    "kimi",
                    uuid,
                    &session_id,
                    ts,
                    "user",
                    Some("user"),
                    Some(content),
                    None,
                ));
            }
            Some("ContentPart") => {
                let timestamp = raw.get("timestamp").and_then(Value::as_f64);
                counter += 1;
                let uuid = format!("kimi-{counter}");
                let ts = timestamp_to_rfc3339(timestamp);

                let payload = raw.get("message").and_then(|m| m.get("payload"));
                let part_type = payload.and_then(|p| p.get("type")).and_then(Value::as_str);

                match part_type {
                    Some("think") => {
                        let think_content = payload
                            .and_then(|p| p.get("think"))
                            .and_then(Value::as_str)
                            .unwrap_or("");

                        // The think content is often a JSON string with the actual think
                        let actual_think = if think_content.starts_with('{') {
                            if let Ok(parsed) = serde_json::from_str::<Value>(think_content) {
                                parsed
                                    .get("think")
                                    .and_then(Value::as_str)
                                    .unwrap_or(think_content)
                                    .to_string()
                            } else {
                                think_content.to_string()
                            }
                        } else {
                            think_content.to_string()
                        };

                        if !actual_think.is_empty() {
                            let content = serde_json::json!([{
                                "type": "thinking",
                                "thinking": actual_think
                            }]);
                            messages.push(build_provider_message(
                                "kimi",
                                uuid,
                                &session_id,
                                ts,
                                "assistant",
                                Some("assistant"),
                                Some(content),
                                None,
                            ));
                            current_turn_assistant.push(messages.len() - 1);
                        }
                    }
                    Some("text") => {
                        let text = payload
                            .and_then(|p| p.get("text"))
                            .and_then(Value::as_str)
                            .unwrap_or("");

                        if !text.is_empty() {
                            let content = serde_json::json!([{
                                "type": "text",
                                "text": text
                            }]);
                            messages.push(build_provider_message(
                                "kimi",
                                uuid,
                                &session_id,
                                ts,
                                "assistant",
                                Some("assistant"),
                                Some(content),
                                None,
                            ));
                            current_turn_assistant.push(messages.len() - 1);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    if !current_turn_assistant.is_empty() {
        assistant_msg_indices_per_turn.push(current_turn_assistant);
    }

    let total_tokens = wire_dir
        .map(|d| read_total_tokens(d))
        .unwrap_or(0);
    if total_tokens > 0 && !assistant_msg_indices_per_turn.is_empty() {
        let per_turn = total_tokens / assistant_msg_indices_per_turn.len() as u64;
        if per_turn > 0 {
            for msg_indices in &assistant_msg_indices_per_turn {
                if let Some(&last_idx) = msg_indices.last() {
                    if let Some(msg) = messages.get_mut(last_idx) {
                        msg.usage = Some(TokenUsage {
                            input_tokens: Some(per_turn as u32),
                            output_tokens: Some(0),
                            cache_creation_input_tokens: None,
                            cache_read_input_tokens: None,
                            service_tier: None,
                        });
                    }
                }
            }
        }
    }

    for msg in &mut messages {
        if msg.role.as_deref() == Some("assistant") && msg.model.is_none() {
            msg.model = Some("kimi-for-coding".to_string());
        }
    }

    Ok(messages)
}

/// Search across all Kimi sessions
pub fn search(query: &str, limit: usize) -> Result<Vec<ClaudeMessage>, String> {
    let base = get_base_path().ok_or("Could not determine Kimi base path")?;
    let sessions_path = PathBuf::from(&base).join("sessions");

    if !sessions_path.is_dir() {
        return Ok(Vec::new());
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for session_dir_entry in fs::read_dir(&sessions_path)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        if session_dir_entry
            .file_type()
            .map(|ft| ft.is_symlink())
            .unwrap_or(true)
        {
            continue;
        }

        let session_dir = session_dir_entry.path();
        if !session_dir.is_dir() {
            continue;
        }

        // Find all wire.jsonl files
        if let Ok(entries) = fs::read_dir(&session_dir) {
            for entry in entries.flatten() {
                let subdir = entry.path();
                if !subdir.is_dir() {
                    continue;
                }

                let wire_path = subdir.join("wire.jsonl");
                if !wire_path.is_file() {
                    continue;
                }

                let data = match fs::read_to_string(&wire_path) {
                    Ok(d) => d,
                    Err(_) => continue,
                };

                let session_id = subdir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                for line in data.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let raw: Value = match serde_json::from_str(trimmed) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    // Search in ContentPart text and ToolResult output
                    if let Some(payload) = raw.get("message").and_then(|m| m.get("payload")) {
                        let search_text = match payload.get("type").and_then(Value::as_str) {
                            Some("text") => payload.get("text").and_then(Value::as_str),
                            Some("think") => payload.get("think").and_then(Value::as_str),
                            _ => None,
                        };

                        if let Some(text) = search_text {
                            if text.to_lowercase().contains(&query_lower) {
                                let timestamp = raw.get("timestamp").and_then(Value::as_f64);
                                let ts = timestamp_to_rfc3339(timestamp);

                                let content = serde_json::json!([{
                                    "type": "text",
                                    "text": format!("[Found in search] {}", text.chars().take(200).collect::<String>())
                                }]);

                                results.push(build_provider_message(
                                    "kimi",
                                    format!("kimi-search-{}", results.len()),
                                    &session_id,
                                    ts,
                                    "assistant",
                                    Some("assistant"),
                                    Some(content),
                                    None,
                                ));

                                if results.len() >= limit {
                                    return Ok(results);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

// ============================================================================
// Private helpers
// ============================================================================

/// Read total cumulative token count from the last `_usage` entry in context.jsonl.
fn read_total_tokens(session_dir: &Path) -> u64 {
    let context_paths = [
        session_dir.join("context.jsonl"),
        session_dir.join("context_1.jsonl"),
    ];

    let ctx_path = match context_paths.iter().find(|p| p.is_file()) {
        Some(p) => p,
        None => return 0,
    };

    let data = match fs::read_to_string(ctx_path) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    let mut max_tokens = 0u64;
    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(obj) = serde_json::from_str::<Value>(trimmed) {
            if obj.get("role").and_then(Value::as_str) == Some("_usage") {
                if let Some(count) = obj.get("token_count").and_then(Value::as_u64) {
                    max_tokens = max_tokens.max(count);
                }
            }
        }
    }
    max_tokens
}

fn get_project_name_from_state(session_dir: &Path) -> Option<String> {
    let context_paths = [
        session_dir.join("context.jsonl"),
        session_dir.join("context_1.jsonl"),
    ];

    for context_path in &context_paths {
        if let Ok(content) = fs::read_to_string(context_path) {
            for line in content.lines() {
                if let Ok(obj) = serde_json::from_str::<Value>(line.trim()) {
                    if obj.get("role").and_then(Value::as_str) == Some("_system_prompt") {
                        if let Some(content_str) = obj.get("content").and_then(Value::as_str) {
                            if let Some(idx) = content_str.find("Working Directory") {
                                let search = "is `";
                                if let Some(pos) = content_str[idx..].find(search) {
                                    let actual_pos = idx + pos;
                                    let rest = &content_str[actual_pos + 4..];
                                    if let Some(end_idx) = rest.find('`') {
                                        let path = &rest[..end_idx];
                                        let home = std::env::var("HOME").unwrap_or_default();
                                        let display_path = if path.starts_with(&home) {
                                            format!("~{}", &path[home.len()..])
                                        } else {
                                            path.to_string()
                                        };
                                        return Some(display_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let state_path = session_dir.join("state.json");
    if let Ok(content) = fs::read_to_string(&state_path) {
        if let Ok(val) = serde_json::from_str::<Value>(&content) {
            if let Some(title) = val.get("custom_title").and_then(Value::as_str) {
                if !title.is_empty() {
                    return Some(if title.len() > 50 {
                        format!("{}...", &title[..50])
                    } else {
                        title.to_string()
                    });
                }
            }
        }
    }

    None
}

fn timestamp_to_rfc3339(ts: Option<f64>) -> String {
    match ts {
        Some(t) => {
            let secs = (t as i64).max(0);
            let nsecs = ((t - secs as f64) * 1e9) as u32;
            chrono::DateTime::from_timestamp(secs, nsecs)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| Utc::now().to_rfc3339())
        }
        None => Utc::now().to_rfc3339(),
    }
}

fn extract_session_metadata(wire_path: &Path) -> (usize, String, Option<String>) {
    let data = match fs::read_to_string(wire_path) {
        Ok(d) => d,
        Err(_) => return (0, String::new(), None),
    };

    let mut message_count = 0usize;
    let mut last_modified = String::new();
    let mut summary = None;

    for line in data.lines().take(100) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let raw: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = raw
            .get("message")
            .and_then(|m| m.get("type"))
            .and_then(Value::as_str);

        if msg_type == Some("TurnBegin") {
            message_count += 1;

            // Extract timestamp
            if let Some(ts) = raw.get("timestamp").and_then(Value::as_f64) {
                last_modified = timestamp_to_rfc3339(Some(ts));
            }

            // Extract first user input as summary
            if summary.is_none() {
                if let Some(user_input) = raw
                    .get("message")
                    .and_then(|m| m.get("payload"))
                    .and_then(|p| p.get("user_input"))
                    .and_then(Value::as_array)
                {
                    for item in user_input {
                        if item.get("type").and_then(Value::as_str) == Some("text") {
                            if let Some(text) = item.get("text").and_then(Value::as_str) {
                                summary = Some(if text.len() > 100 {
                                    format!("{}...", &text[..100])
                                } else {
                                    text.to_string()
                                });
                                break;
                            }
                        }
                    }
                }
            }
        } else if msg_type == Some("ContentPart") || msg_type == Some("ToolCall") {
            message_count += 1;
        }
    }

    if last_modified.is_empty() {
        last_modified = Utc::now().to_rfc3339();
    }

    (message_count, last_modified, summary)
}

fn map_kimi_tool_name(name: &str) -> &str {
    match name {
        "ReadFile" => "Read",
        "WriteFile" | "CreateFile" => "Write",
        "EditFile" => "Edit",
        "Shell" | "Bash" | "run_command" => "Bash",
        "ListDir" | "Glob" => "Glob",
        "SearchFiles" | "Grep" => "Grep",
        "WebSearch" => "WebSearch",
        "WebFetch" => "WebFetch",
        "TodoWrite" => "TodoWrite",
        "Agent" | "SubAgent" => "Task",
        _ => name,
    }
}

use chrono::Utc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_kimi_tool_name() {
        assert_eq!(map_kimi_tool_name("ReadFile"), "Read");
        assert_eq!(map_kimi_tool_name("WriteFile"), "Write");
        assert_eq!(map_kimi_tool_name("Shell"), "Bash");
        assert_eq!(map_kimi_tool_name("Glob"), "Glob");
        assert_eq!(map_kimi_tool_name("unknown"), "unknown");
    }

    #[test]
    fn test_timestamp_to_rfc3339() {
        let ts = 1776984243.0105853;
        let result = timestamp_to_rfc3339(Some(ts));
        assert!(result.contains("2026"));
    }

    #[test]
    fn test_timestamp_to_rfc3339_none() {
        let result = timestamp_to_rfc3339(None);
        assert!(!result.is_empty());
    }
}
