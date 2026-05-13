//! Peer profile management — static markdown-based persona definitions.
//!
//! Each session has a corresponding peer. A `peer-<name>.md` file (or the
//! fallback `peer.md`) in the workspace directory describes that peer's
//! traits, preferences, and communication style. The content is injected into
//! the system prompt so the agent can adapt its responses to the peer.
//!
//! ## File layout
//!
//! ```text
//! workspace/
//!   peer.md           — default peer profile (fallback)
//!   peer-me.md        — specific peer profile for "me"
//!   peer-alice.md     — specific peer profile for "alice"
//! ```
//!
//! ## PEER.md format
//!
//! ```markdown
//! ---
//! name: me
//! description: My personal preferences
//! tags: [personal]
//! ---
//!
//! ## Preferences
//!
//! - 喜欢简单的内容，不喜欢复杂的原理
//! - 喜欢从核心概念开始了解一个事物
//! - 不喜欢别人用太多的比喻和比拟来介绍事物
//! ```
//!
//! Optional YAML frontmatter is supported for `name`, `description`, and `tags`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A peer profile loaded from a markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Raw markdown content (frontmatter stripped) — injected into system prompt.
    pub content: String,
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
struct PeerMarkdownMeta {
    name: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
}

/// Return the path for a specific peer file: `<workspace>/peer-<name>.md`.
fn peer_file_path(workspace_dir: &Path, name: &str) -> PathBuf {
    workspace_dir.join(format!("peer-{name}.md"))
}

/// Return the path for the default peer file: `<workspace>/peer.md`.
fn default_peer_file_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("peer.md")
}

/// Load a peer profile by name.
///
/// Looks for `<workspace>/peer-<name>.md` first. If that does not exist,
/// falls back to `<workspace>/peer.md` (the default peer profile).
/// Returns `None` if neither file exists.
pub fn load_peer(workspace_dir: &Path, name: &str) -> Option<Peer> {
    let specific = peer_file_path(workspace_dir, name);
    if specific.exists() {
        return load_peer_md(&specific, name).ok();
    }
    let default_path = default_peer_file_path(workspace_dir);
    if default_path.exists() {
        return load_peer_md(&default_path, name).ok();
    }
    None
}

/// Load all specific peer profiles from the workspace.
///
/// Scans for `peer-*.md` files. The fallback `peer.md` is NOT included
/// in this list (it is only used as a default via [`load_peer`]).
pub fn load_peers(workspace_dir: &Path) -> Vec<Peer> {
    let mut peers = Vec::new();

    let Ok(entries) = std::fs::read_dir(workspace_dir) else {
        return peers;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(name) = filename.strip_prefix("peer-").and_then(|s| s.strip_suffix(".md")) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        if let Ok(peer) = load_peer_md(&path, name) {
            peers.push(peer);
        }
    }

    peers.sort_by(|a, b| a.name.cmp(&b.name));
    peers
}

/// Format peer content for injection into the system prompt.
///
/// Returns `None` if the peer or its content is missing/empty.
pub fn peer_to_prompt_fragment(peer: &Peer) -> Option<String> {
    let trimmed = peer.content.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut fragment = String::new();
    fragment.push_str("## Peer Profile\n\n");
    fragment.push_str(&format!("You are chatting with **{}**.", peer.name));
    if !peer.description.is_empty() {
        fragment.push_str(&format!(" {}.", peer.description));
    }
    fragment.push_str("\n\nAdapt your communication style according to the following preferences:\n\n");
    fragment.push_str(trimmed);
    fragment.push('\n');
    Some(fragment)
}

fn load_peer_md(path: &Path, fallback_name: &str) -> anyhow::Result<Peer> {
    let content = std::fs::read_to_string(path)?;
    let parsed = parse_peer_markdown(&content);

    Ok(Peer {
        name: parsed.meta.name.unwrap_or_else(|| fallback_name.to_string()),
        description: parsed
            .meta
            .description
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| extract_description(&parsed.body)),
        tags: parsed.meta.tags,
        content: parsed.body,
        location: Some(path.to_path_buf()),
    })
}

struct ParsedPeerMarkdown {
    meta: PeerMarkdownMeta,
    body: String,
}

fn parse_peer_markdown(content: &str) -> ParsedPeerMarkdown {
    if let Some((frontmatter, body)) = split_peer_frontmatter(content) {
        let meta = parse_simple_frontmatter(&frontmatter);
        return ParsedPeerMarkdown { meta, body };
    }

    ParsedPeerMarkdown {
        meta: PeerMarkdownMeta::default(),
        body: content.to_string(),
    }
}

/// Lightweight YAML-like frontmatter parser for simple `key: value` pairs.
fn parse_simple_frontmatter(s: &str) -> PeerMarkdownMeta {
    let mut meta = PeerMarkdownMeta::default();
    let mut collecting_tags = false;
    for line in s.lines() {
        if collecting_tags {
            let trimmed = line.trim();
            if let Some(item) = trimmed.strip_prefix("- ") {
                let tag = item.trim().trim_matches('"').trim_matches('\'');
                if !tag.is_empty() {
                    meta.tags.push(tag.to_string());
                }
                continue;
            }
            collecting_tags = false;
        }
        let Some((key, val)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let val = val.trim().trim_matches('"').trim_matches('\'');
        match key {
            "name" => meta.name = Some(val.to_string()),
            "description" => meta.description = Some(val.to_string()),
            "tags" => {
                if val.is_empty() {
                    collecting_tags = true;
                } else {
                    let val = val.trim_start_matches('[').trim_end_matches(']');
                    meta.tags = val
                        .split(',')
                        .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|t| !t.is_empty())
                        .collect();
                }
            }
            _ => {}
        }
    }
    meta
}

fn split_peer_frontmatter(content: &str) -> Option<(String, String)> {
    let normalized = content.replace("\r\n", "\n");
    let rest = normalized.strip_prefix("---\n")?;
    if let Some(idx) = rest.find("\n---\n") {
        let frontmatter = rest[..idx].to_string();
        let body = rest[idx + 5..].to_string();
        return Some((frontmatter, body));
    }
    if let Some(frontmatter) = rest.strip_suffix("\n---") {
        return Some((frontmatter.to_string(), String::new()));
    }
    None
}

fn extract_description(content: &str) -> String {
    content
        .lines()
        .find(|line| !line.starts_with('#') && !line.trim().is_empty())
        .unwrap_or("No description")
        .trim()
        .to_string()
}

/// Initialize the default peer file in the workspace.
///
/// Creates `peer.md` with a template if it does not already exist.
pub fn init_default_peer(workspace_dir: &Path) -> anyhow::Result<()> {
    let path = default_peer_file_path(workspace_dir);
    if path.exists() {
        return Ok(());
    }

    let template = "# Peer: default\n\n\
        ## Preferences\n\n\
        - Add your default preferences here.\n\
        - These apply when no specific peer profile matches the session.\n";
    std::fs::write(&path, template)?;
    Ok(())
}

/// Resolve a peer name from a session identifier.
///
/// Session IDs often carry prefixes like `cli:/path` or `telegram:12345`.
/// This extracts the meaningful peer name from common session ID formats.
/// Falls back to the raw session ID if no specific mapping exists.
pub fn peer_name_from_session_id(session_id: &str) -> &str {
    // Strip common prefixes
    if let Some(rest) = session_id.strip_prefix("cli:") {
        // cli:path/to/session.json -> use file stem as peer name
        return std::path::Path::new(rest)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(session_id);
    }
    // For other prefixes like telegram:12345, use the prefix itself as peer
    if let Some((prefix, _rest)) = session_id.split_once(':') {
        if !prefix.is_empty() {
            return prefix;
        }
    }
    session_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_peer_frontmatter() {
        let md = r#"---
name: me
description: My preferences
tags: [personal, dev]
---

## Preferences

- Like simple things
"#;
        let parsed = parse_peer_markdown(md);
        assert_eq!(parsed.meta.name, Some("me".to_string()));
        assert_eq!(parsed.meta.description, Some("My preferences".to_string()));
        assert_eq!(parsed.meta.tags, vec!["personal", "dev"]);
        assert!(parsed.body.contains("## Preferences"));
    }

    #[test]
    fn parse_peer_no_frontmatter() {
        let md = "## Preferences\n\n- Like simple things\n";
        let parsed = parse_peer_markdown(md);
        assert!(parsed.meta.name.is_none());
        assert_eq!(parsed.body, md);
    }

    #[test]
    fn load_specific_peer_from_disk() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("peer-me.md"),
            "## Preferences\n\n- 喜欢简单的内容\n",
        )
        .unwrap();

        let peer = load_peer(tmp.path(), "me").unwrap();
        assert_eq!(peer.name, "me");
        assert!(peer.content.contains("喜欢简单的内容"));
    }

    #[test]
    fn load_peer_falls_back_to_default() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("peer.md"),
            "## Preferences\n\n- Default preference\n",
        )
        .unwrap();

        // No peer-me.md exists, so should fall back to peer.md
        let peer = load_peer(tmp.path(), "me").unwrap();
        assert_eq!(peer.name, "me");
        assert!(peer.content.contains("Default preference"));
    }

    #[test]
    fn load_peer_specific_takes_priority_over_default() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("peer.md"),
            "## Preferences\n\n- Default preference\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("peer-me.md"),
            "## Preferences\n\n- Specific preference\n",
        )
        .unwrap();

        let peer = load_peer(tmp.path(), "me").unwrap();
        assert!(peer.content.contains("Specific preference"));
        assert!(!peer.content.contains("Default preference"));
    }

    #[test]
    fn load_peer_missing() {
        let tmp = TempDir::new().unwrap();
        assert!(load_peer(tmp.path(), "nobody").is_none());
    }

    #[test]
    fn peer_to_prompt_fragment_basic() {
        let peer = Peer {
            name: "me".into(),
            description: "Myself".into(),
            tags: vec![],
            content: "- 喜欢简单的内容\n".into(),
            location: None,
        };
        let frag = peer_to_prompt_fragment(&peer).unwrap();
        assert!(frag.contains("You are chatting with **me**"));
        assert!(frag.contains("Myself"));
        assert!(frag.contains("喜欢简单的内容"));
    }

    #[test]
    fn peer_to_prompt_fragment_empty_content() {
        let peer = Peer {
            name: "me".into(),
            description: "".into(),
            tags: vec![],
            content: "".into(),
            location: None,
        };
        assert!(peer_to_prompt_fragment(&peer).is_none());
    }

    #[test]
    fn peer_name_from_session_cli() {
        assert_eq!(peer_name_from_session_id("cli:/home/me/.zeroclaw/session.json"), "session");
        assert_eq!(peer_name_from_session_id("cli:/path/to/my-chat.json"), "my-chat");
    }

    #[test]
    fn peer_name_from_session_prefix() {
        assert_eq!(peer_name_from_session_id("telegram:12345"), "telegram");
        assert_eq!(peer_name_from_session_id("discord:my-server"), "discord");
    }

    #[test]
    fn peer_name_from_session_plain() {
        assert_eq!(peer_name_from_session_id("me"), "me");
        assert_eq!(peer_name_from_session_id("alice"), "alice");
    }

    #[test]
    fn load_peers_sorts_and_filters() {
        let tmp = TempDir::new().unwrap();

        for name in &["charlie", "alice", "bob"] {
            std::fs::write(
                tmp.path().join(format!("peer-{name}.md")),
                format!("# {name}\n"),
            )
            .unwrap();
        }

        // peer.md should NOT appear in load_peers list
        std::fs::write(tmp.path().join("peer.md"), "# default\n").unwrap();
        // Other files should be ignored
        std::fs::write(tmp.path().join("other.md"), "oops").unwrap();

        let peers = load_peers(tmp.path());
        assert_eq!(peers.len(), 3);
        assert_eq!(peers[0].name, "alice");
        assert_eq!(peers[1].name, "bob");
        assert_eq!(peers[2].name, "charlie");
    }

    #[test]
    fn init_default_peer_creates_file() {
        let tmp = TempDir::new().unwrap();
        init_default_peer(tmp.path()).unwrap();
        assert!(tmp.path().join("peer.md").exists());
    }

    #[test]
    fn init_default_peer_idempotent() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("peer.md"), "existing").unwrap();
        init_default_peer(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join("peer.md")).unwrap();
        assert_eq!(content, "existing");
    }
}
