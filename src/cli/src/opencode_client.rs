use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum OpenCodeWsMessage {
    Chunk { content: String, timestamp: Option<u64> },
    Thinking { content: String },
    Done { full_response: String },
    Error { message: String },
    ToolCall { name: String, args: serde_json::Value },
    SessionStart { session_id: String },
    PermissionRequest {
        permission_id: String,
        permission: String,
        patterns: Vec<String>,
        metadata: serde_json::Value,
    },
}

impl OpenCodeWsMessage {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            OpenCodeWsMessage::Chunk { content, timestamp } => {
                let mut m = serde_json::json!({"type": "chunk", "content": content});
                if let Some(ts) = timestamp {
                    m["timestamp"] = serde_json::Value::Number((*ts).into());
                }
                m
            }
            OpenCodeWsMessage::Thinking { content } => {
                serde_json::json!({"type": "thinking", "content": content})
            }
            OpenCodeWsMessage::Done { full_response } => {
                serde_json::json!({"type": "done", "full_response": full_response})
            }
            OpenCodeWsMessage::Error { message } => {
                serde_json::json!({"type": "error", "message": message})
            }
            OpenCodeWsMessage::ToolCall { name, args } => {
                serde_json::json!({"type": "tool_call", "name": name, "args": args})
            }
            OpenCodeWsMessage::SessionStart { session_id } => {
                serde_json::json!({"type": "session_start", "session_id": session_id})
            }
            OpenCodeWsMessage::PermissionRequest { permission_id, permission, patterns, metadata } => {
                serde_json::json!({
                    "type": "permission_request",
                    "permission_id": permission_id,
                    "permission": permission,
                    "patterns": patterns,
                    "metadata": metadata
                })
            }
        }
    }
}

pub struct OpenCodeClient {
    base_url: String,
    client: Client,
}

impl OpenCodeClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    pub async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/global/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn create_session(&self, title: &str) -> anyhow::Result<String> {
        let resp = self
            .client
            .post(format!("{}/session", self.base_url))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({"title": title}))
            .send()
            .await?;

        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            Ok(result["id"].as_str().unwrap_or("").to_string())
        } else {
            anyhow::bail!("Failed to create session: {}", resp.status())
        }
    }

    pub async fn send_message(
        &self,
        session_id: &str,
        text: &str,
    ) -> anyhow::Result<()> {
        let resp = self
            .client
            .post(format!("{}/session/{}/message", self.base_url, session_id))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "parts": [{"type": "text", "text": text}],
                "agent": "build",
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to send message: {}", resp.status())
        }
        Ok(())
    }

    pub async fn list_messages(
        &self,
        session_id: &str,
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let resp = self
            .client
            .get(format!("{}/session/{}/message", self.base_url, session_id))
            .send()
            .await?;

        if resp.status().is_success() {
            let messages: Vec<serde_json::Value> = resp.json().await?;
            Ok(messages)
        } else {
            anyhow::bail!("Failed to list messages: {}", resp.status())
        }
    }

    pub async fn abort_session(&self, session_id: &str) -> anyhow::Result<()> {
        let resp = self
            .client
            .post(format!("{}/session/{}/abort", self.base_url, session_id))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to abort session: {}", resp.status())
        }
        Ok(())
    }

    pub async fn reply_permission(&self, permission_id: &str, response: &str) -> anyhow::Result<()> {
        let resp = self
            .client
            .post(format!("{}/permission/{}/reply", self.base_url, permission_id))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({"response": response}))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to reply to permission: {}", resp.status())
        }
        Ok(())
    }
}
