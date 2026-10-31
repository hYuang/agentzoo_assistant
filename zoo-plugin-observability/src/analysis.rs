//! Read-only analysis of the host's conversation data and log file.
//!
//! The observability page is served by the plugin itself: these functions turn
//! the host's stores (conversations, sessions and their transcripts) and the
//! `zooagent.log` file into JSON the page can render. Everything here is
//! read-only; the plugin never mutates host state.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use zoo_ai::model::ModelPlatform;
use zoo_plugins::HttpError;
use zoo_shared::message::{ConversationId, ProjectId};
use zoo_store::{ConversationStore, SessionStore};

use crate::PluginCtx;

/// Map a storage failure onto the HTTP error envelope.
fn storage_err(error: zoo_store::StorageError) -> HttpError {
    HttpError::Execution(error.to_string())
}

/// The host's conversation store, resolved once per request.
fn conv_store(ctx: &PluginCtx) -> Result<&dyn ConversationStore, HttpError> {
    ctx.conversations
        .as_deref()
        .ok_or_else(|| HttpError::Execution("conversation store unavailable".into()))
}

/// The host's session store, resolved once per request.
fn sess_store(ctx: &PluginCtx) -> Result<&dyn SessionStore, HttpError> {
    ctx.sessions
        .as_deref()
        .ok_or_else(|| HttpError::Execution("session store unavailable".into()))
}

/// The host data directory: the log file is `<data_dir>/zooagent.log`.
fn data_dir(ctx: &PluginCtx) -> &Path {
    &ctx.data_dir
}

/// `GET /projects/{pid}/plugins/observability/analysis/conversations`
///
/// One row per conversation of the project, with its sessions and per-session
/// token/latency aggregates computed from the stored transcripts.
pub async fn conversations(
    ctx: &PluginCtx,
    project_id: &str,
) -> Result<Value, HttpError> {
    let conv_store = conv_store(ctx)?;
    let sess_store = sess_store(ctx)?;
    let pid = ProjectId(project_id.to_string());
    let items = conv_store.list(&pid).await.map_err(storage_err)?;

    let mut rows = Vec::with_capacity(items.len());
    for item in &items {
        let session_ids = sess_store
            .list_by_conversation(&pid, &item.id)
            .await
            .map_err(storage_err)?;
        let mut sessions = Vec::with_capacity(session_ids.len());
        let (mut prompt_tokens, mut completion_tokens) = (0u64, 0u64);
        let (mut turns, mut errors) = (0u64, 0u64);
        let mut first_at: Option<String> = None;
        let mut last_at: Option<String> = None;

        for sid in session_ids {
            let Some(session) = sess_store.load(&pid, &sid).await.map_err(storage_err)? else {
                continue;
            };
            let (pt, ct, t, e, fa, la) = summarize_history(&session.history);
            prompt_tokens += pt;
            completion_tokens += ct;
            turns += t;
            errors += e;
            first_at = match (first_at, fa) {
                (Some(a), Some(b)) => Some(min_rfc3339(&a, &b)),
                (a, b) => a.or(b),
            };
            last_at = match (last_at, la) {
                (Some(a), Some(b)) => Some(max_rfc3339(&a, &b)),
                (a, b) => a.or(b),
            };
            sessions.push(json!({
                "session_id": sid.as_str(),
                "agent": session.agent,
                "model": session.model,
                "is_main": session.is_main,
                "message_count": session.history.len(),
                "context_tokens": session.context_tokens,
                "created_at": session.created_at.to_rfc3339(),
            }));
        }

        rows.push(json!({
            "conversation_id": item.id.as_str(),
            "title": item.title,
            "main_session_id": item.main_session_id.as_str(),
            "created_at": item.created_at.to_rfc3339(),
            "updated_at": item.updated_at.to_rfc3339(),
            "session_count": sessions.len(),
            "message_count": sessions.iter().map(|s| s["message_count"].as_u64().unwrap_or(0)).sum::<u64>(),
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "turns": turns,
            "errors": errors,
            "first_at": first_at,
            "last_at": last_at,
            "sessions": sessions,
        }));
    }

    let total_prompt: u64 = rows.iter().map(|r| r["prompt_tokens"].as_u64().unwrap_or(0)).sum();
    let total_completion: u64 = rows.iter().map(|r| r["completion_tokens"].as_u64().unwrap_or(0)).sum();
    let total_turns: u64 = rows.iter().map(|r| r["turns"].as_u64().unwrap_or(0)).sum();
    let total_errors: u64 = rows.iter().map(|r| r["errors"].as_u64().unwrap_or(0)).sum();

    Ok(json!({
        "project_id": project_id,
        "conversation_count": rows.len(),
        "prompt_tokens": total_prompt,
        "completion_tokens": total_completion,
        "turns": total_turns,
        "errors": total_errors,
        "conversations": rows,
    }))
}

/// `GET /projects/{pid}/plugins/observability/analysis/conversations/{cid}`
///
/// The merged transcript of one conversation: every session's messages in
/// chronological order, each tagged with its session and agent so routed work
/// reads as one continuous dialogue.
pub async fn conversation_detail(
    ctx: &PluginCtx,
    project_id: &str,
    conversation_id: &str,
) -> Result<Value, HttpError> {
    let conv_store = conv_store(ctx)?;
    let sess_store = sess_store(ctx)?;
    let pid = ProjectId(project_id.to_string());
    let cid = ConversationId(conversation_id.to_string());

    let Some(item) = conv_store.load(&pid, &cid).await.map_err(storage_err)? else {
        return Err(HttpError::NotFound(format!("conversation not found: {conversation_id}")));
    };
    let session_ids = sess_store
        .list_by_conversation(&pid, &cid)
        .await
        .map_err(storage_err)?;

    let mut messages = Vec::new();
    let mut by_session: Vec<Value> = Vec::new();
    for sid in session_ids {
        let Some(session) = sess_store.load(&pid, &sid).await.map_err(storage_err)? else {
            continue;
        };
        let mut count = 0u64;
        for message in &session.history {
            count += 1;
            messages.push(json!({
                "session_id": sid.as_str(),
                "agent": session.agent,
                "role": format!("{:?}", message.role).to_lowercase(),
                "at": message.at.0.to_rfc3339(),
                "seq": message.seq,
                "turn_id": message.turn_id,
                "synthetic": message.synthetic,
                "content": message.content,
                "tool_calls": message.tool_calls,
                "tool_call_id": message.tool_call_id,
                "usage": message.usage,
            }));
        }
        by_session.push(json!({
            "session_id": sid.as_str(),
            "agent": session.agent,
            "model": session.model,
            "is_main": session.is_main,
            "message_count": count,
        }));
    }

    Ok(json!({
        "conversation_id": item.id.as_str(),
        "title": item.title,
        "created_at": item.created_at.to_rfc3339(),
        "updated_at": item.updated_at.to_rfc3339(),
        "sessions": by_session,
        "message_count": messages.len(),
        "messages": messages,
    }))
}

/// `GET /projects/{pid}/plugins/observability/analysis/logs`
///
/// The tail of the host's `zooagent.log`, parsed into structured lines with
/// per-level totals. `limit` bounds how many lines are returned; `level`
/// filters on the parsed level; `q` is a case-insensitive substring match.
pub async fn logs(
    ctx: &PluginCtx,
    limit: usize,
    level: &str,
    query: &str,
) -> Result<Value, HttpError> {
    let path = data_dir(ctx).join("zooagent.log");
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({
                "path": path.to_string_lossy(),
                "available": false,
                "total_lines": 0,
                "returned": 0,
                "levels": {},
                "lines": [],
            }));
        }
        Err(error) => return Err(HttpError::Execution(format!("failed to read log: {error}"))),
    };
    let owned = String::from_utf8_lossy(&bytes).into_owned();
    let text = strip_bom(&owned);

    let wanted = level.to_ascii_lowercase();
    let needle = query.to_lowercase();
    let mut lines: Vec<Value> = Vec::new();
    let mut levels: BTreeMap<String, u64> = BTreeMap::new();
    let mut total = 0u64;

    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            continue;
        }
        total += 1;
        let (at, lvl, target, message) = parse_line(line);
        let lvl = lvl.unwrap_or("INFO");
        let matches_level = wanted.is_empty() || wanted == lvl.to_ascii_lowercase();
        let matches_query = needle.is_empty() || line.to_lowercase().contains(&needle);
        if !matches_level || !matches_query {
            continue;
        }
        *levels.entry(lvl.to_string()).or_insert(0) += 1;
        if lines.len() < limit {
            lines.push(json!({
                "at": at,
                "level": lvl,
                "target": target,
                "message": message,
            }));
        }
    }

    Ok(json!({
        "path": path.to_string_lossy(),
        "available": true,
        "total_lines": total,
        "returned": lines.len(),
        "levels": levels,
        "lines": lines,
    }))
}

/// Aggregate the token usage, turn count, error count and time span of a
/// session transcript.
fn summarize_history(history: &[zoo_shared::message::Message]) -> (u64, u64, u64, u64, Option<String>, Option<String>) {
    let mut prompt = 0u64;
    let mut completion = 0u64;
    let mut turns = 0u64;
    let mut errors = 0u64;
    let mut first: Option<String> = None;
    let mut last: Option<String> = None;
    for message in history {
        if let Some(usage) = &message.usage {
            prompt += usage.prompt_tokens;
            completion += usage.completion_tokens;
        }
        if message.role == zoo_shared::message::Role::Assistant {
            turns += 1;
        }
        let has_error = message.content.iter().any(|block| {
            matches!(
                block,
                zoo_shared::content::ContentBlock::Text { text }
                    if text.starts_with("Error") || text.contains("failed")
            )
        });
        if has_error {
            errors += 1;
        }
        let at = message.at.0.to_rfc3339();
        first = match (first, &at) {
            (Some(a), b) => Some(min_rfc3339(&a, b)),
            (None, b) => Some(b.clone()),
        };
        last = match (last, &at) {
            (Some(a), b) => Some(max_rfc3339(&a, b)),
            (None, b) => Some(b.clone()),
        };
    }
    (prompt, completion, turns, errors, first, last)
}

/// Split one `tracing` fmt line into (timestamp, level, target, message).
///
/// The host writes lines like
/// `2026-10-22T14:03:11.123456+08:00  INFO zoo_server: message key=value`;
/// anything that does not start with a timestamp is reported with `None`
/// fields and the whole line as the message.
fn parse_line(line: &str) -> (Option<String>, Option<&str>, Option<&str>, &str) {
    let rest = line
        .find(' ')
        .and_then(|i| line[i + 1..].strip_prefix(' '))
        .unwrap_or(line);
    let (at, rest) = if rest.contains('T') && rest.starts_with(|c: char| c.is_ascii_digit()) {
        let sp = rest.find(' ').unwrap_or(rest.len());
        let ts = &rest[..sp];
        let parsed = DateTime::parse_from_rfc3339(ts)
            .ok()
            .map(|d| d.with_timezone(&Utc).to_rfc3339());
        (parsed, &rest[sp + 1.min(rest.len() - sp)..])
    } else {
        (None, rest)
    };
    let (lvl, rest) = rest
        .split_once(' ')
        .map(|(a, b)| (a, b.strip_prefix(' ').unwrap_or(b)))
        .unwrap_or((rest, ""));
    let is_level = matches!(lvl, "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR");
    let (lvl, rest) = if is_level { (Some(lvl), rest) } else { (None, rest) };
    let (target, message) = match rest.find(": ") {
        Some(i) => (Some(&rest[..i]), &rest[i + 2..]),
        None => (None, rest),
    };
    (at, lvl, target, message)
}

fn min_rfc3339(a: &str, b: &str) -> String {
    if lexicographic_rfc3339(a, b) {
        a.to_string()
    } else {
        b.to_string()
    }
}

fn max_rfc3339(a: &str, b: &str) -> String {
    if lexicographic_rfc3339(a, b) {
        b.to_string()
    } else {
        a.to_string()
    }
}

/// RFC 3339 timestamps with the same offset compare lexicographically; the
/// host always writes UTC (`Z`), so a plain string order is chronological.
fn lexicographic_rfc3339(a: &str, b: &str) -> bool {
    a <= b
}

fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

/// `GET /projects/{pid}/plugins/observability/agents`
///
/// The registered agents with their prompt layers and tool specs.
pub async fn agents(ctx: &PluginCtx, project_id: &str) -> Result<Value, HttpError> {
    let runtimes = ctx
        .runtimes
        .as_deref()
        .ok_or_else(|| HttpError::Execution("runtimes unavailable".into()))?;
    let map = runtimes.read().unwrap();
    let project_root = std::path::Path::new(project_id);

    let mut agents = Vec::new();
    for (name, runtime) in map.iter() {
        let layers = runtime.prompt_layers(project_root, ModelPlatform::Local, false);
        let tools = runtime.tool_specs();
        agents.push(json!({
            "name": name,
            "context_budget": runtime.context_budget(),
            "prompt_layers": layers,
            "tools": tools,
        }));
    }
    agents.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));

    Ok(json!({
        "project_id": project_id,
        "agent_count": agents.len(),
        "agents": agents,
    }))
}

/// `GET /projects/{pid}/plugins/observability/analysis/sessions/{sid}`
///
/// Per-session statistics: LLM calls, tool calls, token usage, errors,
/// and the tool-call distribution.
pub async fn session_stats(
    ctx: &PluginCtx,
    project_id: &str,
    session_id: &str,
) -> Result<Value, HttpError> {
    let sess_store = sess_store(ctx)?;
    let pid = ProjectId(project_id.to_string());
    let sid = zoo_shared::message::SessionId(session_id.to_string());

    let Some(session) = sess_store.load(&pid, &sid).await.map_err(storage_err)? else {
        return Err(HttpError::NotFound(format!("session not found: {session_id}")));
    };

    let mut llm_calls = 0u64;
    let mut tool_calls = 0u64;
    let mut prompt_tokens = 0u64;
    let mut completion_tokens = 0u64;
    let mut errors = 0u64;
    let mut tool_dist: BTreeMap<String, u64> = BTreeMap::new();
    let mut first_at: Option<String> = None;
    let mut last_at: Option<String> = None;

    for message in &session.history {
        let at = message.at.0.to_rfc3339();
        first_at = match (first_at, &at) {
            (Some(a), b) => Some(min_rfc3339(&a, b)),
            (None, b) => Some(b.clone()),
        };
        last_at = match (last_at, &at) {
            (Some(a), b) => Some(max_rfc3339(&a, b)),
            (None, b) => Some(b.clone()),
        };

        if let Some(usage) = &message.usage {
            llm_calls += 1;
            prompt_tokens += usage.prompt_tokens;
            completion_tokens += usage.completion_tokens;
        }
        for call in &message.tool_calls {
            tool_calls += 1;
            *tool_dist.entry(call.name.clone()).or_insert(0) += 1;
        }
        let has_error = message.content.iter().any(|block| {
            matches!(
                block,
                zoo_shared::content::ContentBlock::Text { text }
                    if text.starts_with("Error") || text.contains("failed")
            )
        });
        if has_error {
            errors += 1;
        }
    }

    let tool_list: Vec<Value> = tool_dist
        .iter()
        .map(|(name, count)| json!({ "name": name, "count": count }))
        .collect();

    Ok(json!({
        "session_id": sid.as_str(),
        "agent": session.agent,
        "model": session.model,
        "is_main": session.is_main,
        "message_count": session.history.len(),
        "context_tokens": session.context_tokens,
        "created_at": session.created_at.to_rfc3339(),
        "llm_calls": llm_calls,
        "tool_calls": tool_calls,
        "prompt_tokens": prompt_tokens,
        "completion_tokens": completion_tokens,
        "errors": errors,
        "tool_distribution": tool_list,
        "first_at": first_at,
        "last_at": last_at,
    }))
}
