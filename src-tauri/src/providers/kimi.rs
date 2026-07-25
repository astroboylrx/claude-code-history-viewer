use crate::models::{ClaudeMessage, ClaudeProject, ClaudeSession, TokenUsage};
use crate::providers::ProviderInfo;
use crate::utils::build_provider_message;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn detect() -> Option<ProviderInfo> {
    let bases = get_base_paths();
    if bases.is_empty() {
        return None;
    }
    let base = bases[0].clone();
    let sessions_path = PathBuf::from(&base).join("sessions");
    Some(ProviderInfo {
        id: "kimi".to_string(),
        display_name: "Kimi Code".to_string(),
        base_path: base,
        is_available: bases
            .iter()
            .any(|b| PathBuf::from(b).join("sessions").is_dir()),
    })
}

pub fn get_base_path() -> Option<String> {
    get_base_paths().into_iter().next()
}

fn get_base_paths() -> Vec<String> {
    if let Ok(val) = std::env::var("KIMI_HOME") {
        let p = PathBuf::from(&val);
        if p.is_dir() {
            return vec![val];
        }
    }
    let mut result = Vec::new();
    if let Some(h) = dirs::home_dir() {
        let new = h.join(".kimi-code");
        if new.is_dir() {
            result.push(new.to_string_lossy().to_string());
        }
        let old = h.join(".kimi");
        if old.is_dir() {
            result.push(old.to_string_lossy().to_string());
        }
    }
    result
}

fn path_is_v2(path: &str) -> bool {
    path.contains(".kimi-code")
}

fn read_session_index(base: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let index_path = PathBuf::from(base).join("session_index.jsonl");
    if let Ok(data) = fs::read_to_string(&index_path) {
        for line in data.lines() {
            if let Ok(obj) = serde_json::from_str::<Value>(line.trim()) {
                if let (Some(sid), Some(wd)) = (
                    obj.get("sessionId").and_then(Value::as_str),
                    obj.get("workDir").and_then(Value::as_str),
                ) {
                    map.insert(sid.to_string(), wd.to_string());
                }
            }
        }
    }
    map
}

fn workdir_to_display(wd: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && wd.starts_with(&home) {
        format!("~{}", &wd[home.len()..])
    } else {
        wd.to_string()
    }
}

// ============================================================================
// scan_projects
// ============================================================================

pub fn scan_projects() -> Result<Vec<ClaudeProject>, String> {
    let bases = get_base_paths();
    if bases.is_empty() {
        return Err("Could not determine Kimi base path".to_string());
    }
    let mut all_projects = Vec::new();
    for base in &bases {
        let projects = if path_is_v2(base) {
            scan_projects_v2(base)
        } else {
            scan_projects_v1(base)
        }?;
        all_projects.extend(projects);
    }
    Ok(all_projects)
}

fn scan_projects_v1(base: &str) -> Result<Vec<ClaudeProject>, String> {
    let sessions_path = PathBuf::from(base).join("sessions");
    if !sessions_path.is_dir() {
        return Ok(Vec::new());
    }
    let mut projects_map: std::collections::HashMap<String, Vec<(PathBuf, PathBuf)>> = std::collections::HashMap::new();

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
        if let Ok(entries) = fs::read_dir(&session_dir) {
            for entry in entries.flatten() {
                let subdir = entry.path();
                if !subdir.is_dir() {
                    continue;
                }
                let wire_path = subdir.join("wire.jsonl");
                if wire_path.is_file() {
                    let key = get_project_name_from_state_v1(&subdir).unwrap_or_else(|| {
                        session_dir
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    });
                    projects_map
                        .entry(key)
                        .or_default()
                        .push((subdir, wire_path));
                }
            }
        }
    }

    let mut projects = Vec::new();
    for (project_name, sessions) in projects_map {
        let mut total_messages = 0usize;
        let mut last_modified = String::new();
        for (_, wire_path) in &sessions {
            let (mc, lm, _) = extract_session_metadata_v1(wire_path);
            total_messages += mc;
            if lm > last_modified {
                last_modified = lm;
            }
        }
        if last_modified.is_empty() {
            last_modified = Utc::now().to_rfc3339();
        }
        let actual_path = sessions[0].0.parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        projects.push(ClaudeProject {
            name: project_name,
            path: format!("kimi://{}", actual_path),
            actual_path,
            session_count: sessions.len(),
            message_count: total_messages,
            last_modified,
            git_info: None,
            provider: Some("kimi".to_string()),
            storage_type: Some("jsonl".to_string()),
            custom_directory_label: Some("CLI".to_string()),
        });
    }
    Ok(projects)
}

fn scan_projects_v2(base: &str) -> Result<Vec<ClaudeProject>, String> {
    let sessions_path = PathBuf::from(base).join("sessions");
    if !sessions_path.is_dir() {
        return Ok(Vec::new());
    }
    let workdir_map = read_session_index(base);
    let mut projects_map: std::collections::HashMap<String, Vec<(PathBuf, PathBuf, Option<String>)>> = std::collections::HashMap::new();

    for wd_entry in fs::read_dir(&sessions_path)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        if wd_entry
            .file_type()
            .map(|ft| ft.is_symlink())
            .unwrap_or(true)
        {
            continue;
        }
        let wd_dir = wd_entry.path();
        if !wd_dir.is_dir() {
            continue;
        }
        if let Ok(ses_entries) = fs::read_dir(&wd_dir) {
            for ses_entry in ses_entries.flatten() {
                let ses_dir = ses_entry.path();
                if !ses_dir.is_dir() {
                    continue;
                }
                let wire_path = ses_dir.join("agents").join("main").join("wire.jsonl");
                if !wire_path.is_file() {
                    continue;
                }
                let ses_id = ses_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let workdir = workdir_map.get(&ses_id).cloned();
                let project_name = workdir
                    .as_deref()
                    .map(workdir_to_display)
                    .unwrap_or_else(|| {
                        wd_dir
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    });
                projects_map
                    .entry(project_name)
                    .or_default()
                    .push((ses_dir, wire_path, workdir));
            }
        }
    }

    let mut projects = Vec::new();
    for (project_name, sessions) in projects_map {
        let mut total_messages = 0usize;
        let mut last_modified = String::new();
        for (_, wire_path, _) in &sessions {
            let (mc, lm, _) = extract_session_metadata_v2(wire_path);
            total_messages += mc;
            if lm > last_modified {
                last_modified = lm;
            }
        }
        if last_modified.is_empty() {
            last_modified = Utc::now().to_rfc3339();
        }
        let actual_path = sessions.first()
            .and_then(|(d, _, wd)| {
                wd.clone().or_else(|| d.to_str().map(String::from))
            })
            .unwrap_or_default();
        projects.push(ClaudeProject {
            name: project_name,
            path: format!("kimi://{}", sessions[0].0.parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")),
            actual_path,
            session_count: sessions.len(),
            message_count: total_messages,
            last_modified,
            git_info: None,
            provider: Some("kimi".to_string()),
            storage_type: Some("jsonl".to_string()),
            custom_directory_label: Some("Code".to_string()),
        });
    }
    Ok(projects)
}

// ============================================================================
// load_sessions
// ============================================================================

pub fn load_sessions(
    project_path: &str,
    _exclude_sidechain: bool,
) -> Result<Vec<ClaudeSession>, String> {
    let dir = project_path.strip_prefix("kimi://").unwrap_or(project_path);
    let sessions_dir = PathBuf::from(dir);
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let new_fmt = path_is_v2(dir);

    let project_name = if new_fmt {
        let kimi_code_base = get_base_paths()
            .into_iter()
            .find(|b| path_is_v2(b))
            .unwrap_or_default();
        let workdir_map = read_session_index(&kimi_code_base);
        sessions_dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| {
                workdir_map
                    .get(s)
                    .map(|wd| workdir_to_display(wd))
                    .unwrap_or_else(|| s.to_string())
            })
            .unwrap_or_default()
    } else {
        sessions_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    };

    let mut sessions = Vec::new();

    // Gather all session directories
    let session_subdirs: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Ok(entries) = fs::read_dir(&sessions_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    v.push(p);
                }
            }
        }
        v
    };

    for subdir in session_subdirs {
        let wire_path = if new_fmt {
            subdir.join("agents").join("main").join("wire.jsonl")
        } else {
            subdir.join("wire.jsonl")
        };
        if !wire_path.is_file() {
            continue;
        }

        let (message_count, last_modified, default_summary) = if new_fmt {
            extract_session_metadata_v2(&wire_path)
        } else {
            extract_session_metadata_v1(&wire_path)
        };

        let session_id = subdir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // For v2, try state.json title first
        let summary = if new_fmt {
            get_title_from_state_v2(&subdir).or(default_summary)
        } else {
            default_summary
        };

        let file_path_str = wire_path.to_string_lossy().to_string();

        sessions.push(ClaudeSession {
            session_id: format!("kimi://{}", subdir.to_string_lossy()),
            actual_session_id: session_id,
            file_path: file_path_str,
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

// ============================================================================
// load_messages
// ============================================================================

pub fn load_messages(session_path: &str) -> Result<Vec<ClaudeMessage>, String> {
    let path = session_path.strip_prefix("kimi://").unwrap_or(session_path);
    let wire_path = PathBuf::from(path);
    if !wire_path.is_file() {
        return Err(format!("Session file not found: {session_path}"));
    }

    if path_is_v2(path) {
        load_messages_v2(&wire_path)
    } else {
        load_messages_v1(&wire_path)
    }
}

// ---- v1 (old ~/.kimi/ format) ----

fn load_messages_v1(wire_path: &Path) -> Result<Vec<ClaudeMessage>, String> {
    let data = fs::read_to_string(wire_path)
        .map_err(|e| format!("Failed to read session file: {e}"))?;

    let session_id = wire_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let session_dir = wire_path.parent().and_then(|p| p.parent());
    let wire_dir = wire_path.parent();

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
                            "kimi", uuid, &session_id, ts, "user",
                            Some("user"), Some(content), None,
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
                    "kimi", uuid, &session_id, ts, "assistant",
                    Some("assistant"), Some(content), None,
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
                    "kimi", uuid, &session_id, ts, "user",
                    Some("user"), Some(content), None,
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
                        let actual_think = if think_content.starts_with('{') {
                            if let Ok(parsed) = serde_json::from_str::<Value>(think_content) {
                                parsed.get("think").and_then(Value::as_str).unwrap_or(think_content).to_string()
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
                                "kimi", uuid, &session_id, ts, "assistant",
                                Some("assistant"), Some(content), None,
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
                                "kimi", uuid, &session_id, ts, "assistant",
                                Some("assistant"), Some(content), None,
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
        .map(|d| read_total_tokens_v1(d))
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

// ---- v2 (new ~/.kimi-code/ format) ----

fn load_messages_v2(wire_path: &Path) -> Result<Vec<ClaudeMessage>, String> {
    let data = fs::read_to_string(wire_path)
        .map_err(|e| format!("Failed to read session file: {e}"))?;

    let session_id = wire_path
        .ancestors()
        .nth(2)
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let session_dir = wire_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent());

    // Try to get created_at from metadata line for timestamps
    let base_timestamp = data
        .lines()
        .find_map(|line| {
            serde_json::from_str::<Value>(line.trim()).ok()
        })
        .and_then(|obj| {
            if obj.get("type").and_then(Value::as_str) == Some("metadata") {
                obj.get("created_at").and_then(Value::as_u64)
            } else {
                None
            }
        });

    let mut messages: Vec<ClaudeMessage> = Vec::new();
    let mut counter = 0u64;
    let mut kimi_model: Option<String> = None;

    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let raw: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let line_type = raw.get("type").and_then(Value::as_str).unwrap_or("");

        // Capture model name from llm.request events
        if line_type == "llm.request" {
            if let Some(alias) = raw.get("modelAlias").and_then(Value::as_str) {
                kimi_model = Some(alias.to_string());
            }
        }

        // usage.record duplicates step.end data — skip to avoid double counting
        if line_type == "usage.record" {
            continue;
        }

        // Also handle context.append_loop_event with step.end for usage
        // AND content.part for assistant messages (newer Kimi Code format)
        if line_type == "context.append_loop_event" {
            if let Some(event) = raw.get("event") {
                let evt_type = event.get("type").and_then(Value::as_str).unwrap_or("");

                // step.end → extract usage and apply to last assistant message
                if evt_type == "step.end" {
                    if let Some(usage) = event.get("usage") {
                        let input = usage.get("inputOther").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let output = usage.get("output").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let cache_read = usage.get("inputCacheRead").and_then(Value::as_u64).unwrap_or(0) as u32;
                        let cache_create = usage.get("inputCacheCreation").and_then(Value::as_u64).unwrap_or(0) as u32;
                        if input > 0 || output > 0 {
                            for msg in messages.iter_mut().rev() {
                                if msg.role.as_deref() == Some("assistant") && msg.usage.is_none() {
                                    msg.usage = Some(TokenUsage {
                                        input_tokens: Some(input),
                                        output_tokens: Some(output),
                                        cache_creation_input_tokens: if cache_create > 0 { Some(cache_create) } else { None },
                                        cache_read_input_tokens: if cache_read > 0 { Some(cache_read) } else { None },
                                        service_tier: None,
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }

                // content.part → create assistant message
                if evt_type == "content.part" {
                    if let Some(part) = event.get("part") {
                        let part_type = part.get("type").and_then(Value::as_str).unwrap_or("");
                        let ts = base_timestamp
                            .map(|t| timestamp_to_rfc3339(Some(t as f64)))
                            .unwrap_or_else(|| Utc::now().to_rfc3339());

                        match part_type {
                            "think" => {
                                let think = part.get("think").and_then(Value::as_str).unwrap_or("");
                                if !think.is_empty() {
                                    counter += 1;
                                    let uuid = format!("kimi-{counter}");
                                    let content = serde_json::json!([{"type": "thinking", "thinking": think}]);
                                    messages.push(build_provider_message(
                                        "kimi", uuid, &session_id, ts, "assistant",
                                        Some("assistant"), Some(content), None,
                                    ));
                                }
                            }
                            "text" => {
                                let text = part.get("text").and_then(Value::as_str).unwrap_or("");
                                if !text.is_empty() {
                                    counter += 1;
                                    let uuid = format!("kimi-{counter}");
                                    let content = serde_json::json!([{"type": "text", "text": text}]);
                                    messages.push(build_provider_message(
                                        "kimi", uuid, &session_id, ts, "assistant",
                                        Some("assistant"), Some(content), None,
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            continue;
        }

        if line_type != "context.append_message" {
            continue;
        }

        let msg = match raw.get("message") {
            Some(m) => m,
            None => continue,
        };

        let role = msg.get("role").and_then(Value::as_str).unwrap_or("user");
        let content_arr = msg.get("content").and_then(Value::as_array);
        let tool_calls = msg.get("toolCalls").and_then(Value::as_array);
        let tool_call_id = msg.get("toolCallId").and_then(Value::as_str);

        counter += 1;
        let uuid = format!("kimi-{counter}");
        let ts = base_timestamp
            .map(|t| timestamp_to_rfc3339(Some(t as f64)))
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        match role {
            "user" => {
                let mut text_parts = Vec::new();
                if let Some(arr) = content_arr {
                    for c in arr {
                        if c.get("type").and_then(Value::as_str) == Some("text") {
                            if let Some(t) = c.get("text").and_then(Value::as_str) {
                                text_parts.push(t.to_string());
                            }
                        }
                    }
                }
                let text = text_parts.join("\n");
                if !text.is_empty() {
                    let content = serde_json::json!([{"type": "text", "text": text}]);
                    messages.push(build_provider_message(
                        "kimi", uuid, &session_id, ts, "user",
                        Some("user"), Some(content), None,
                    ));
                }
            }
            "assistant" => {
                // Emit thinking blocks
                if let Some(arr) = content_arr {
                    for c in arr {
                        let ctype = c.get("type").and_then(Value::as_str).unwrap_or("");
                        match ctype {
                            "think" => {
                                let think = c.get("think").and_then(Value::as_str).unwrap_or("");
                                if !think.is_empty() {
                                    counter += 1;
                                    let uuid2 = format!("kimi-{counter}");
                                    let content = serde_json::json!([{
                                        "type": "thinking",
                                        "thinking": think
                                    }]);
                                    messages.push(build_provider_message(
                                        "kimi", uuid2, &session_id, ts.clone(),
                                        "assistant", Some("assistant"), Some(content), None,
                                    ));
                                }
                            }
                            "text" => {
                                let text = c.get("text").and_then(Value::as_str).unwrap_or("");
                                if !text.is_empty() {
                                    counter += 1;
                                    let uuid2 = format!("kimi-{counter}");
                                    let content = serde_json::json!([{
                                        "type": "text",
                                        "text": text
                                    }]);
                                    messages.push(build_provider_message(
                                        "kimi", uuid2, &session_id, ts.clone(),
                                        "assistant", Some("assistant"), Some(content), None,
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // Emit tool_use blocks from toolCalls
                if let Some(tc_arr) = tool_calls {
                    for tc in tc_arr {
                        let func = tc.get("function");
                        let func_name = func
                            .and_then(|f| f.get("name"))
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        let tool_id = tc
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown")
                            .to_string();
                        let args_str = func
                            .and_then(|f| f.get("arguments"))
                            .and_then(Value::as_str)
                            .unwrap_or("{}");
                        let args: Value = serde_json::from_str(args_str).unwrap_or(Value::Null);
                        counter += 1;
                        let uuid2 = format!("kimi-{counter}");
                        let content = serde_json::json!([{
                            "type": "tool_use",
                            "id": tool_id,
                            "name": map_kimi_tool_name(func_name),
                            "input": args
                        }]);
                        messages.push(build_provider_message(
                            "kimi", uuid2, &session_id, ts.clone(),
                            "assistant", Some("assistant"), Some(content), None,
                        ));
                    }
                }
            }
            "tool" => {
                let mut text_parts = Vec::new();
                if let Some(arr) = content_arr {
                    for c in arr {
                        if c.get("type").and_then(Value::as_str) == Some("text") {
                            if let Some(t) = c.get("text").and_then(Value::as_str) {
                                text_parts.push(t.to_string());
                            }
                        }
                    }
                }
                let text = text_parts.join("\n");
                let tc_id = tool_call_id.unwrap_or("unknown").to_string();
                let content = serde_json::json!([{
                    "type": "tool_result",
                    "tool_use_id": tc_id,
                    "content": text
                }]);
                messages.push(build_provider_message(
                    "kimi", uuid, &session_id, ts, "user",
                    Some("user"), Some(content), None,
                ));
            }
            _ => {}
        }
    }

    let model_name = kimi_model.as_deref().unwrap_or("kimi-for-coding");
    for msg in &mut messages {
        if msg.role.as_deref() == Some("assistant") && msg.model.is_none() {
            msg.model = Some(model_name.to_string());
        }
    }

    // Try to recover token data from old format if this session was migrated
    let session_dir = wire_path.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent());
    if let Some(dir) = session_dir {
        let total_tokens = read_tokens_from_migrated_session(dir);
        if total_tokens > 0 {
            let assistant_indices: Vec<usize> = messages.iter().enumerate()
                .filter(|(_, m)| m.role.as_deref() == Some("assistant"))
                .map(|(i, _)| i)
                .collect();
            if !assistant_indices.is_empty() {
                let per_turn = total_tokens / assistant_indices.len() as u64;
                if per_turn > 0 {
                    for &idx in &assistant_indices {
                        if let Some(msg) = messages.get_mut(idx) {
                            if msg.usage.is_none() {
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
        }
    }

    for msg in &mut messages {
        if msg.role.as_deref() == Some("assistant") && msg.model.is_none() {
            msg.model = Some("kimi-for-coding".to_string());
        }
    }

    Ok(messages)
}

// ============================================================================
// search
// ============================================================================

pub fn search(query: &str, limit: usize) -> Result<Vec<ClaudeMessage>, String> {
    let bases = get_base_paths();
    if bases.is_empty() {
        return Err("Could not determine Kimi base path".to_string());
    }
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for base in &bases {
        let sessions_path = PathBuf::from(base).join("sessions");
        if !sessions_path.is_dir() {
            continue;
        }
        let new_fmt = path_is_v2(base);

        let wire_finder = |subdir: &Path| -> Option<PathBuf> {
            if new_fmt {
                let p = subdir.join("agents").join("main").join("wire.jsonl");
                if p.is_file() { Some(p) } else { None }
            } else {
                let p = subdir.join("wire.jsonl");
                if p.is_file() { Some(p) } else { None }
            }
        };

        for wd_entry in fs::read_dir(&sessions_path)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            if wd_entry
                .file_type()
                .map(|ft| ft.is_symlink())
                .unwrap_or(true)
            {
                continue;
            }
            let wd_dir = wd_entry.path();
            if !wd_dir.is_dir() {
                continue;
            }
            if let Ok(ses_entries) = fs::read_dir(&wd_dir) {
                for ses_entry in ses_entries.flatten() {
                    let subdir = ses_entry.path();
                    if !subdir.is_dir() {
                        continue;
                    }
                    let wire_path = match wire_finder(&subdir) {
                        Some(p) => p,
                        None => continue,
                    };
                    let data = match fs::read_to_string(&wire_path) {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    let ses_id = subdir
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

                        let texts: Vec<String> = if new_fmt {
                            raw.get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(Value::as_array)
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|c| {
                                            let t = c.get("type").and_then(Value::as_str)?;
                                            match t {
                                                "text" => c.get("text").and_then(Value::as_str).map(String::from),
                                                "think" => c.get("think").and_then(Value::as_str).map(String::from),
                                                _ => None,
                                            }
                                        })
                                        .collect()
                                })
                                .unwrap_or_default()
                        } else {
                            raw.get("message")
                                .and_then(|m| m.get("payload"))
                                .and_then(|p| {
                                    match p.get("type").and_then(Value::as_str) {
                                        Some("text") => p.get("text").and_then(Value::as_str).map(|s| vec![s.to_string()]),
                                        Some("think") => p.get("think").and_then(Value::as_str).map(|s| vec![s.to_string()]),
                                        _ => None,
                                    }
                                })
                                .unwrap_or_default()
                        };

                        for text in texts {
                            if text.to_lowercase().contains(&query_lower) {
                                let ts = Utc::now().to_rfc3339();
                                let content = serde_json::json!([{
                                    "type": "text",
                                    "text": format!("[Found in search] {}", &text.chars().take(200).collect::<String>())
                                }]);
                                results.push(build_provider_message(
                                    "kimi",
                                    format!("kimi-search-{}", results.len()),
                                    &ses_id, ts, "assistant",
                                    Some("assistant"), Some(content), None,
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
// Metadata helpers
// ============================================================================

fn extract_session_metadata_v1(wire_path: &Path) -> (usize, String, Option<String>) {
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
            if let Some(ts) = raw.get("timestamp").and_then(Value::as_f64) {
                last_modified = timestamp_to_rfc3339(Some(ts));
            }
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
                                summary = Some(truncate_safe(text, 100));
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

fn extract_session_metadata_v2(wire_path: &Path) -> (usize, String, Option<String>) {
    let data = match fs::read_to_string(wire_path) {
        Ok(d) => d,
        Err(_) => return (0, String::new(), None),
    };
    let mut message_count = 0usize;
    let mut last_modified = String::new();
    let mut summary = None;
    let mut created_at: Option<u64> = None;

    for line in data.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let raw: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let line_type = raw.get("type").and_then(Value::as_str).unwrap_or("");

        if line_type == "metadata" {
            created_at = raw.get("created_at").and_then(Value::as_u64);
            continue;
        }
        if line_type != "context.append_message" {
            continue;
        }

        let msg = match raw.get("message") {
            Some(m) => m,
            None => continue,
        };
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
        if role == "user" || role == "assistant" {
            message_count += 1;
        }

        if summary.is_none() && role == "user" {
            if let Some(arr) = msg.get("content").and_then(Value::as_array) {
                for c in arr {
                    if c.get("type").and_then(Value::as_str) == Some("text") {
                        if let Some(text) = c.get("text").and_then(Value::as_str) {
                            if !text.is_empty() && !text.starts_with("<system>") {
                                summary = Some(truncate_safe(text, 100));
                                break;
                            }
                        }
                    }
                }
            }
        }

        if let Some(ts) = raw.get("time").and_then(Value::as_u64).or(created_at) {
            last_modified = timestamp_to_rfc3339(Some(ts as f64));
        }
    }

    if last_modified.is_empty() {
        if let Some(ts) = created_at {
            last_modified = timestamp_to_rfc3339(Some(ts as f64));
        } else {
            last_modified = Utc::now().to_rfc3339();
        }
    }
    (message_count, last_modified, summary)
}

fn get_title_from_state_v2(session_dir: &Path) -> Option<String> {
    let state_path = session_dir.join("state.json");
    let content = fs::read_to_string(&state_path).ok()?;
    let val: Value = serde_json::from_str(&content).ok()?;
    let title = val.get("title").and_then(Value::as_str)?;
    if title.is_empty() || title == "New Session" {
        return None;
    }
    Some(truncate_safe(title, 100))
}

fn get_project_name_from_state_v1(session_dir: &Path) -> Option<String> {
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
                                        return Some(workdir_to_display(path));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn read_tokens_from_migrated_session(session_dir: &Path) -> u64 {
    let state_path = session_dir.join("state.json");
    let content = match std::fs::read_to_string(&state_path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let val: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let source_path = match val
        .get("custom")
        .and_then(|c| c.get("kimi_cli_source_path"))
        .and_then(Value::as_str)
    {
        Some(p) => p,
        None => return 0,
    };
    let old_dir = Path::new(source_path);
    read_total_tokens_v1(old_dir)
}

fn read_total_tokens_v1(session_dir: &Path) -> u64 {
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

fn truncate_safe(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((idx, _)) => format!("{}...", &s[..idx]),
        None => s.to_string(),
    }
}

fn timestamp_to_rfc3339(ts: Option<f64>) -> String {
    match ts {
        Some(t) => {
            // Auto-detect millisecond timestamps (values > 1e12 are ms, not seconds)
            let t = if t > 1e12 { t / 1000.0 } else { t };
            let secs = (t as i64).max(0);
            let nsecs = ((t - secs as f64) * 1e9) as u32;
            chrono::DateTime::from_timestamp(secs, nsecs)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| Utc::now().to_rfc3339())
        }
        None => Utc::now().to_rfc3339(),
    }
}

fn map_kimi_tool_name(name: &str) -> &str {
    match name {
        "ReadFile" | "Read" => "Read",
        "WriteFile" | "CreateFile" | "Write" => "Write",
        "EditFile" | "Edit" => "Edit",
        "Shell" | "Bash" | "run_command" => "Bash",
        "ListDir" | "Glob" => "Glob",
        "SearchFiles" | "Grep" => "Grep",
        "WebSearch" => "WebSearch",
        "WebFetch" | "FetchURL" => "WebFetch",
        "TodoWrite" | "TodoList" => "TodoWrite",
        "Agent" | "SubAgent" | "AgentSwarm" => "Task",
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

    #[test]
    fn test_workdir_to_display() {
        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            let result = workdir_to_display(&format!("{}/projects/foo", home));
            assert_eq!(result, "~/projects/foo");
        }
    }
}
