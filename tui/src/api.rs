use reqwest::Client;
use anyhow::Result;
use crate::models::*;

pub struct ApiClient {
    client: Client,
    base_url: String,
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

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn check_health(&self) -> bool {
        self.client
            .get(format!("{}/ok", self.base_url))
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
