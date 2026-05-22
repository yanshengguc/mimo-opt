use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
}

impl Content {
    pub fn text(s: impl Into<String>) -> Self {
        Content::Text(s.into())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Content::Text(s) => s,
        }
    }

    #[cfg(test)]
    pub fn as_mut_str(&mut self) -> &mut String {
        match self {
            Content::Text(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Content,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

// ── Anthropic 格式 ──

#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemContent>>,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub delta: Option<AnthropicDelta>,
    pub message: Option<AnthropicStreamMessage>,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicStreamMessage {
    pub usage: Option<Usage>,
}

// ── OpenAI 格式 ──

#[derive(Debug, Serialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct OpenAIStreamOptions {
    pub include_usage: bool,
}

#[derive(Debug, Serialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<OpenAIStreamOptions>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIStreamEvent {
    pub choices: Option<Vec<OpenAIChoice>>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIChoice {
    pub delta: Option<OpenAIDelta>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIDelta {
    pub content: Option<String>,
}

// ── 共用 ──

#[derive(Debug, Deserialize)]
pub struct Usage {
    #[serde(alias = "prompt_tokens")]
    pub input_tokens: Option<u64>,
    #[serde(alias = "completion_tokens")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_text_roundtrip() {
        let c = Content::text("hello");
        assert_eq!(c.as_str(), "hello");
    }

    #[test]
    fn content_from_string() {
        let s = String::from("world");
        let c = Content::text(s);
        assert_eq!(c.as_str(), "world");
    }

    #[test]
    fn content_as_mut_str() {
        let mut c = Content::text("hello");
        c.as_mut_str().push_str(" world");
        assert_eq!(c.as_str(), "hello world");
    }

    #[test]
    fn content_serde_json_roundtrip() {
        let c = Content::text("test content");
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Content = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.as_str(), "test content");
    }

    #[test]
    fn chat_message_serde_roundtrip() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: Content::text("hello"),
            cache_control: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.role, "user");
        assert_eq!(decoded.content.as_str(), "hello");
        assert!(decoded.cache_control.is_none());
    }

    #[test]
    fn chat_message_with_cache_control() {
        let msg = ChatMessage {
            role: "assistant".to_string(),
            content: Content::text("response"),
            cache_control: Some(CacheControl {
                cache_type: "ephemeral".to_string(),
            }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"cache_control\""));
        assert!(json.contains("\"ephemeral\""));
    }

    #[test]
    fn cache_control_not_serialized_when_none() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: Content::text("hello"),
            cache_control: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("cache_control"));
    }

    #[test]
    fn usage_serde_anthropic_format() {
        let json = r#"{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":20}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(usage.cache_creation_input_tokens, Some(10));
        assert_eq!(usage.cache_read_input_tokens, Some(20));
    }

    #[test]
    fn usage_serde_openai_format() {
        // OpenAI uses prompt_tokens/completion_tokens (aliased)
        let json = r#"{"prompt_tokens":200,"completion_tokens":80}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, Some(200));
        assert_eq!(usage.output_tokens, Some(80));
        assert_eq!(usage.cache_creation_input_tokens, None);
    }

    #[test]
    fn system_content_serialization() {
        let sc = SystemContent {
            content_type: "text".to_string(),
            text: "You are helpful.".to_string(),
            cache_control: Some(CacheControl {
                cache_type: "ephemeral".to_string(),
            }),
        };
        let json = serde_json::to_string(&sc).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"You are helpful.\""));
    }

    #[test]
    fn openai_request_serialization() {
        let req = OpenAIRequest {
            model: "gpt-4o".to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: "hello".to_string(),
            }],
            max_tokens: 4096,
            temperature: Some(0.7),
            top_p: None,
            stream: true,
            stream_options: Some(OpenAIStreamOptions {
                include_usage: true,
            }),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\":\"gpt-4o\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(!json.contains("top_p")); // None should be skipped
    }
}
