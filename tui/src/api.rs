use reqwest::Client;
use anyhow::Result;
use crate::models::*;

pub struct ApiClient {
    client: Client,
    base_url: String,
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

        Self { client, base_url }
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
            // Handle both array and string responses
            let text = resp.text().await?;
            if text.starts_with('[') {
                // Try to find the JSON array in the response
                if let Ok(agents) = serde_json::from_str::<Vec<OpenclawAgent>>(&text) {
                    return Ok(agents);
                }
            }
            // Try to extract JSON array from mixed content
            if let Some(start) = text.find('[') {
                if let Some(end) = text.rfind(']') {
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
}
