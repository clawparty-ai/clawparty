use reqwest::Client;
use anyhow::Result;
use crate::models::*;
use std::time::Duration;

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    #[allow(dead_code)]
    token: String,
}

impl ApiClient {
    pub fn new(base_url: String, token: String) -> Self {
        let client = Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "Authorization",
                    format!("Bearer {}", token).parse().unwrap(),
                );
                headers
            })
            .build()
            .unwrap();

        Self { client, base_url, token }
    }

    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[allow(dead_code)]
    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn check_health(&self) -> bool {
        self.client
                .get(format!("{}/api/version", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn get_meshes(&self) -> Result<Vec<Mesh>> {
        let resp = self.client
            .get(format!("{}/api/meshes", self.base_url))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let meshes: Vec<Mesh> = resp.json().await?;
            Ok(meshes)
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_endpoints(&self, mesh: &str) -> Result<Vec<Endpoint>> {
        let resp = self.client
            .get(format!("{}/api/meshes/{}/endpoints?limit=500", self.base_url, mesh))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let endpoints: Vec<Endpoint> = resp.json().await?;
            Ok(endpoints)
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_chats(&self, mesh: &str) -> Result<Vec<Chat>> {
        let resp = self.client
            .get(format!("{}/api/meshes/{}/apps/ztm/chat/api/chats", self.base_url, mesh))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let chats: Vec<Chat> = resp.json().await?;
            Ok(chats)
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_peer_messages(&self, mesh: &str, peer: &str) -> Result<Vec<Message>> {
        let resp = self.client
            .get(format!("{}/api/meshes/{}/apps/ztm/chat/api/peers/{}/messages", 
                self.base_url, mesh, peer))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let messages: Vec<Message> = resp.json().await?;
            Ok(messages)
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_group_messages(&self, mesh: &str, creator: &str, group: &str) -> Result<Vec<Message>> {
        let resp = self.client
            .get(format!("{}/api/meshes/{}/apps/ztm/chat/api/groups/{}/{}/messages", 
                self.base_url, mesh, creator, group))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let messages: Vec<Message> = resp.json().await?;
            Ok(messages)
        } else {
            Ok(vec![])
        }
    }

    pub async fn send_peer_message(&self, mesh: &str, peer: &str, text: &str) -> Result<()> {
        let body = serde_json::json!({ "text": text });
        let resp = self.client
            .post(format!("{}/api/meshes/{}/apps/ztm/chat/api/peers/{}/messages", 
                self.base_url, mesh, peer))
            .json(&body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to send message: {}", resp.status())
        }
    }

    pub async fn send_group_message(&self, mesh: &str, creator: &str, group: &str, text: &str) -> Result<()> {
        let body = serde_json::json!({ "text": text });
        let resp = self.client
            .post(format!("{}/api/meshes/{}/apps/ztm/chat/api/groups/{}/{}/messages", 
                self.base_url, mesh, creator, group))
            .json(&body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to send message: {}", resp.status())
        }
    }

    pub async fn get_openclaw_agents(&self) -> Result<Vec<OpenclawAgent>> {
        let resp = self.client
            .get(format!("{}/api/openclaw/agents", self.base_url))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let text = resp.text().await?;
            
            // Find the JSON array by matching brackets
            if let Some(start) = text.find('[') {
                let mut depth = 0;
                let mut end = None;
                for (i, ch) in text[start..].char_indices() {
                    if ch == '[' { depth += 1; }
                    if ch == ']' { depth -= 1; }
                    if depth == 0 {
                        end = Some(start + i);
                        break;
                    }
                }
                if let Some(end) = end {
                    if let Ok(agents) = serde_json::from_str::<Vec<OpenclawAgent>>(&text[start..=end]) {
                        return Ok(agents);
                    }
                }
            }
            Ok(vec![])
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_openclaw_messages(&self, agent_id: &str) -> Result<Vec<Message>> {
        let resp = self.client
            .get(format!("{}/api/openclaw/{}/chat-log", self.base_url, agent_id))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let messages: Vec<Message> = resp.json().await?;
            Ok(messages)
        } else {
            Ok(vec![])
        }
    }

    pub async fn check_zeroclaw_health(&self) -> bool {
        self.client
            .get("http://127.0.0.1:42617/health")
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn get_zeroclaw_sessions(&self) -> Result<Vec<crate::app::ZeroClawSession>> {
        let resp = self.client
            .get("http://127.0.0.1:42617/api/sessions")
            .send()
            .await?;
        
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            let sessions: Vec<crate::app::ZeroClawSession> = result["sessions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|s| {
                    Some(crate::app::ZeroClawSession {
                        session_id: s["session_id"].as_str()?.to_string(),
                        name: s["name"].as_str()
                            .unwrap_or(s["session_id"].as_str().unwrap_or(""))
                            .to_string(),
                        last_activity: s["last_activity"].as_str()?.to_string(),
                    })
                })
                .collect();
            Ok(sessions)
        } else {
            Ok(vec![])
        }
    }

    pub async fn create_zeroclaw_session(&self, name: Option<&str>) -> Result<crate::app::ZeroClawSession> {
        let body = serde_json::json!({
            "name": name.unwrap_or("default")
        });
        let resp = self.client
            .post("http://127.0.0.1:42617/api/sessions")
            .json(&body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            Ok(crate::app::ZeroClawSession {
                session_id: result["session_id"].as_str().unwrap_or("").to_string(),
                name: result["name"].as_str()
                    .unwrap_or(result["session_id"].as_str().unwrap_or(""))
                    .to_string(),
                last_activity: "".to_string(),
            })
        } else {
            anyhow::bail!("Failed to create session: {}", resp.status())
        }
    }

    pub async fn send_zeroclaw_message(&self, session_id: &str, text: &str) -> Result<String> {
        let body = serde_json::json!({ "message": text });
        let resp = self.client
            .post(&format!("http://127.0.0.1:42617/api/sessions/{}/chat", session_id))
            .json(&body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            Ok(result["response"].as_str().unwrap_or("").to_string())
        } else {
            anyhow::bail!("Failed to send message: {}", resp.status())
        }
    }

    pub async fn get_zeroclaw_messages(&self, session_id: &str) -> Result<Vec<Message>> {
        let resp = self.client
            .get(&format!("http://127.0.0.1:42617/api/sessions/{}/messages", session_id))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            let messages: Vec<Message> = result["messages"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    Some(Message {
                        message: None,
                        text: m["content"].as_str().map(|s| s.to_string()),
                        sender: m["role"].as_str().map(|s| s.to_string()),
                        time: None,
                        timestamp: None,
                        is_sent: None,
                    })
                })
                .collect();
            Ok(messages)
        } else {
            Ok(vec![])
        }
    }

    pub async fn send_openclaw_message(&self, agent_id: &str, text: &str) -> Result<()> {
        let resp = self.client
            .post(format!("{}/api/openclaw/chat/{}", self.base_url, agent_id))
            .body(text.to_string())
            .header("Content-Type", "text/plain")
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to send message: {}", resp.status())
        }
    }

    #[allow(dead_code)]
    pub async fn get_identity(&self) -> Result<String> {
        let resp = self.client
            .get(format!("{}/api/identity", self.base_url))
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(resp.text().await?)
        } else {
            anyhow::bail!("Failed to get identity: {}", resp.status())
        }
    }

    #[allow(dead_code)]
    pub async fn join_mesh(&self, mesh: &str, ep: &str, permit: &str) -> Result<()> {
        let body = serde_json::json!({
            "name": ep,
            "permit": permit
        });
        let resp = self.client
            .post(format!("{}/api/meshes/{}", self.base_url, mesh))
            .json(&body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to join mesh: {} - {}", status, err_text)
        }
    }

    pub async fn leave_mesh(&self, mesh: &str) -> Result<()> {
        let resp = self.client
            .delete(format!("{}/api/meshes/{}", self.base_url, mesh))
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to leave mesh: {} - {}", status, err_text)
        }
    }

    pub async fn join_party(&self, reg_url: &str, user_name: &str) -> Result<serde_json::Value> {
        let body = serde_json::json!({
            "regUrl": reg_url,
            "userName": user_name
        });
        let resp = self.client
            .post(format!("{}/api/join-party", self.base_url))
            .json(&body)
            .send()
            .await?;
        
        let status = resp.status();
        if status.is_success() {
            let result: serde_json::Value = resp.json().await?;
            Ok(result)
        } else {
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to join party: {} - {}", status, err_text)
        }
    }

    #[allow(dead_code)]
    pub async fn get_default_auto_reply(&self) -> Result<String> {
        let resp = self.client
            .get(format!("{}/api/default-auto-reply", self.base_url))
            .send()
            .await?;
        
        if resp.status().is_success() {
            let result: serde_json::Value = resp.json().await?;
            Ok(result["agent"].as_str().unwrap_or("main").to_string())
        } else {
            Ok("main".to_string())
        }
    }

    pub async fn get_agents(&self) -> Result<Vec<AgentStatus>> {
        let resp = self.client
            .get(format!("{}/api/agents", self.base_url))
            .send()
            .await?;

        if resp.status().is_success() {
            let agents: Vec<AgentStatus> = resp.json().await?;
            Ok(agents)
        } else {
            Ok(vec![])
        }
    }

    pub async fn start_agent(&self, name: &str) -> Result<()> {
        let mut url = url::Url::parse(&self.base_url)
            .map_err(|e| anyhow::anyhow!("Invalid base URL: {}", e))?;
        {
            let mut segments = url.path_segments_mut()
                .map_err(|_| anyhow::anyhow!("Cannot modify URL path"))?;
            segments.extend(&["api", "agents", name, "start"]);
        }
        let resp = self.client
            .post(url)
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Failed to start agent {}: {} - {}", name, status, body)
        }
    }

    pub async fn set_default_auto_reply(&self, agent_name: &str) -> Result<()> {
        let body = serde_json::json!({ "agent": agent_name });
        let resp = self.client
            .post(format!("{}/api/default-auto-reply", self.base_url))
            .json(&body)
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to set default auto-reply: {}", resp.status())
        }
    }
}
