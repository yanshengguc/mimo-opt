use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String, // "ephemeral"
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemContent {
    #[serde(rename = "type")]
    pub content_type: String, // "text"
    pub text: String,
    pub cache_control: CacheControl,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemContent>>,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub delta: Option<Delta>,
    pub message: Option<StreamMessage>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamMessage {
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

/// 流式结果
#[derive(Debug, Clone)]
pub enum StreamResult {
    Token(String),
    Done {
        input_tokens: u64,
        output_tokens: u64,
        cache_creation_tokens: u64,
        cache_read_tokens: u64,
    },
    Error(String),
}
