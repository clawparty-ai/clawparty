//! Memory integration helpers for the agent loop.
//!
//! This module bridges the `zeroclaw-memory` SQLite backend with the
//! agent loop at three tiers:
//!
//! - L1: Tool-result offloading (store oversized results, inject retrieval key).
//! - L2: Session-memory injection (retrieve structured notes on demand).
//! - L4: Long-term index queries (pruner/compressor use as fallback).

use zeroclaw_api::provider::ChatMessage;
use zeroclaw_memory::SqliteMemory;

/// Known model context windows (tokens). Used when no explicit `context_window`
/// is configured in `history-pruning`.
const KNOWN_MODEL_WINDOWS: &[(&str, usize)] = &[
    ("kimi-k2.6", 262_144),
    ("kimi-k2.5", 256_000),
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("gpt-4.5-preview", 128_000),
    ("gpt-4-turbo", 128_000),
    ("claude-sonnet-4-20250514", 200_000),
    ("claude-sonnet-4", 200_000),
    ("claude-sonnet-4-20250514", 200_000),
    ("anthropic/claude-sonnet-4-20250514", 200_000),
    ("deepseek-chat", 64_000),
    ("deepseek-reasoner", 64_000),
    ("gemini-1.5-pro", 1_048_576),
    ("gemini-1.5-flash", 1_048_576),
];

/// Derive a context-window size from a model name.
pub fn infer_context_window(model: &str) -> Option<usize> {
    let lowered = model.to_ascii_lowercase();
    for &(name, window) in KNOWN_MODEL_WINDOWS {
        if lowered.contains(name) {
            return Some(window);
        }
    }
    // Suffix-style detection: e.g. "some-model-128k"
    if lowered.ends_with("-128k") || lowered.ends_with("/128k") {
        return Some(128_000);
    }
    if lowered.ends_with("-256k") || lowered.ends_with("/256k") {
        return Some(256_000);
    }
    if lowered.ends_with("-1m") || lowered.ends_with("/1m") {
        return Some(1_000_000);
    }
    if lowered.ends_with("-200k") || lowered.ends_with("/200k") {
        return Some(200_000);
    }
    None
}

/// Offload oversized tool results from the message history into SQLite.
///
/// When a `tool` message's content exceeds `char_threshold`, the full text is
/// written to `tool_results` and replaced by a `[tool_result:{id}]` marker.
/// Returns the number of offload operations performed.
pub async fn offload_tool_results(
    history: &mut [ChatMessage],
    session_id: &str,
    mem: &SqliteMemory,
    char_threshold: usize,
) -> usize {
    if char_threshold == 0 {
        return 0;
    }
    let mut offloaded = 0;
    for msg in history {
        if msg.role != "tool" {
            continue;
        }
        if msg.content.len() <= char_threshold {
            continue;
        }
        // Construct a preview (first 200 chars, or truncated)
        let preview = msg.content.chars().take(200).collect::<String>() + "...";
        // Try to identify the tool name from the content (best-effort heuristic)
        let tool_name = guess_tool_name_from_result(&msg.content);

        match mem
            .store_tool_result(Some(session_id), &tool_name, None, &msg.content, &preview)
            .await
        {
            Ok(id) => {
                msg.content = format!("[tool_result:{id}] {preview}");
                offloaded += 1;
            }
            Err(e) => {
                tracing::warn!(error = %e, "Tool-result offload failed, keeping in-context");
            }
        }
    }
    offloaded
}

/// Try to guess which tool produced a result from its text.
fn guess_tool_name_from_result(content: &str) -> String {
    for name in ["shell", "file_read", "file_write", "git", "grep", "ls"] {
        if content.contains(name) {
            return name.to_string();
        }
    }
    "tool".to_string()
}

/// Retrieve session memory for a given session and format it as a system
/// message fragment. Returns `None` when there is no stored session memory.
pub async fn get_session_memory_fragment(session_id: &str, mem: &SqliteMemory) -> Option<String> {
    let (summary, key_facts, _last_turn) = mem.get_session_memory(session_id).await.ok()??;
    if summary.is_empty() && key_facts == "[]" {
        return None;
    }

    let mut fragment = "## Session Notes\n".to_string();
    if !summary.is_empty() {
        fragment.push_str(&summary);
        fragment.push('\n');
    }
    if key_facts != "[]" {
        fragment.push_str("Key facts:\n");
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(&key_facts) {
            for fact in arr {
                let _ = std::fmt::Write::write_fmt(&mut fragment, format_args!("- {fact}\n"));
            }
        }
    }
    Some(fragment)
}

/// Persist session memory after a successful LLM turn.
/// The caller should provide the latest assistant message as the incremental
/// summary source.
pub async fn persist_session_memory(
    session_id: &str,
    mem: &SqliteMemory,
    _summary_delta: &str,
    key_facts_delta: &[String],
    last_turn_idx: usize,
) {
    // Retrieve existing or start fresh
    let (existing_summary, existing_facts_json, _) = match mem.get_session_memory(session_id).await
    {
        Ok(Some(t)) => t,
        _ => (String::new(), "[]".to_string(), 0),
    };

    let mut facts: Vec<String> = serde_json::from_str(&existing_facts_json).unwrap_or_default();

    for f in key_facts_delta {
        if !facts.contains(f) {
            facts.push(f.clone());
        }
    }
    // Cap to prevent unbounded growth
    while facts.len() > 20 {
        facts.remove(0);
    }

    let facts_json = serde_json::to_string(&facts).unwrap_or_else(|_| "[]".to_string());

    if let Err(e) = mem
        .store_session_memory(session_id, &existing_summary, &facts_json, last_turn_idx)
        .await
    {
        tracing::warn!(error = %e, "Failed to persist session memory");
    }
}

/// Search the long-term index for facts relevant to the current query.
/// Returns a formatted bullet list suitable for injection into system prompt.
pub async fn search_long_term_index(
    query: &str,
    mem: &SqliteMemory,
    limit: usize,
) -> Option<String> {
    let results = mem.search_long_term(query, None, limit).await.ok()?;
    if results.is_empty() {
        return None;
    }
    let mut out = String::from("### Long-term Context\n");
    for (key, content) in results {
        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("- {key}: {content}\n"));
    }
    Some(out)
}
