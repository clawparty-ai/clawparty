use serde::de;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize)]
pub enum TimeValue {
    String(String),
    Number(u64),
}

impl<'de> Deserialize<'de> for TimeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TimeValueVisitor;

        impl<'de> de::Visitor<'de> for TimeValueVisitor {
            type Value = TimeValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or a number")
            }

            fn visit_str<E>(self, value: &str) -> Result<TimeValue, E>
            where
                E: de::Error,
            {
                Ok(TimeValue::String(value.to_owned()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<TimeValue, E>
            where
                E: de::Error,
            {
                Ok(TimeValue::Number(value))
            }
        }

        deserializer.deserialize_any(TimeValueVisitor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub name: String,
    #[serde(default)]
    pub agent: Option<MeshAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshAgent {
    pub username: String,
    #[serde(default)]
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    #[serde(default)]
    pub peer: Option<String>,
    #[serde(default)]
    pub creator: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub gcid: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub is_group: bool,
    #[serde(default)]
    pub time: Option<u64>,
    #[serde(default)]
    pub updated: Option<u64>,
    #[serde(default)]
    pub latest: Option<LatestMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestMessage {
    #[serde(default)]
    pub time: Option<u64>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub message: Option<MessageBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBody {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub agent_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestedMessage {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub time: Option<TimeValue>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default)]
    #[serde(rename = "isSent")]
    pub is_sent: Option<bool>,
    // For group/peer messages API which has nested message object
    #[serde(default)]
    pub message: Option<NestedMessage>,
}

impl Message {
    pub fn get_text(&self) -> &str {
        // First try nested message.text, then fallback to flat text field
        if let Some(ref nested) = self.message {
            if let Some(ref text) = nested.text {
                return text.as_str();
            }
        }
        self.text.as_deref().unwrap_or("")
    }

    pub fn get_sender(&self) -> &str {
        // First try nested message.sender, then fallback to flat sender field
        if let Some(ref nested) = self.message {
            if let Some(ref sender) = nested.sender {
                return sender.as_str();
            }
        }
        self.sender.as_deref().unwrap_or("Unknown")
    }

    pub fn get_time(&self) -> String {
        // Handle both string and number time formats
        match &self.time {
            Some(TimeValue::String(s)) => s.clone(),
            Some(TimeValue::Number(n)) => {
                // Convert timestamp to HH:MM format
                let d = std::time::UNIX_EPOCH + std::time::Duration::from_millis(*n);
                let datetime: chrono::DateTime<chrono::Local> = d.into();
                datetime.format("%H:%M").to_string()
            }
            None => String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub labels: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenclawAgent {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
    #[serde(default, rename = "identityName")]
    pub identity_name: Option<String>,
    #[serde(default, rename = "identityEmoji")]
    pub identity_emoji: Option<String>,
}

impl OpenclawAgent {
    pub fn display_name(&self) -> String {
        self.identity_name
            .clone()
            .or_else(|| self.name.clone())
            .unwrap_or_else(|| self.id.clone())
    }

    pub fn display_emoji(&self) -> String {
        self.identity_emoji
            .clone()
            .or_else(|| self.emoji.clone())
            .unwrap_or_else(|| "🤖".to_string())
    }
}

impl Chat {
    pub fn display_name(&self) -> String {
        if self.is_group {
            self.name
                .clone()
                .unwrap_or_else(|| format!("#{}", self.group.as_deref().unwrap_or("unknown")))
        } else {
            self.peer.clone().unwrap_or_else(|| "unknown".to_string())
        }
    }

    #[allow(dead_code)]
    pub fn session_id(&self) -> String {
        if self.is_group {
            format!(
                "group:{}:{}",
                self.creator.as_deref().unwrap_or(""),
                self.group.as_deref().unwrap_or("")
            )
        } else {
            format!("peer:{}", self.peer.as_deref().unwrap_or(""))
        }
    }
}
