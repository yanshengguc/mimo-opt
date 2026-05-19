mod types;

use futures_util::StreamExt;
use tokio::sync::mpsc;

pub use types::*;

pub struct MiMoClient {
    client: reqwest::Client,
    #[allow(dead_code)]
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub auth_type: String,   // "anthropic" | "bearer"
    pub api_format: String,  // "anthropic" | "openai"
    pub max_tokens: u32,
    messages_url: String,
}

impl MiMoClient {
    pub fn new(
        base_url: String,
        api_key: String,
        model: String,
        auth_type: String,
        api_format: String,
        max_tokens: u32,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build reqwest client");

        // 根据 api_format 选择端点路径
        let endpoint = if api_format == "openai" {
            "/v1/chat/completions"
        } else {
            "/v1/messages"
        };
        let messages_url = format!("{}{}", base_url, endpoint);

        Self {
            client,
            base_url,
            api_key,
            model,
            auth_type,
            api_format,
            max_tokens,
            messages_url,
        }
    }

    /// 根据 auth_type 设置请求头
    fn set_auth_headers(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.auth_type == "bearer" {
            builder
                .header("authorization", format!("Bearer {}", self.api_key))
                .header("content-type", "application/json")
        } else {
            // anthropic (默认)
            builder
                .header("api-key", &self.api_key)
                .header("content-type", "application/json")
                .header("anthropic-version", "2023-06-01")
        }
    }

    #[allow(dead_code)]
    pub fn api_url(&self) -> &str {
        &self.messages_url
    }

    /// 探测 API 是否可用（发一个最小请求，不浪费配额）
    pub async fn check_api(&self) -> anyhow::Result<()> {
        let builder = if self.api_format == "openai" {
            let req = OpenAIRequest {
                model: self.model.clone(),
                messages: vec![OpenAIMessage {
                    role: "user".to_string(),
                    content: "hi".to_string(),
                }],
                max_tokens: 1,
                stream: false,
            };
            self.client.post(&self.messages_url).json(&req)
        } else {
            let req = AnthropicRequest {
                model: self.model.clone(),
                max_tokens: 1,
                system: None,
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: Content::text("hi"),
                    cache_control: None,
                }],
                stream: false,
            };
            self.client.post(&self.messages_url).json(&req)
        };

        let response = self.set_auth_headers(builder).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("{} {}", status, body);
        }
    }

    /// 流式请求，通过 channel 逐 token 返回 StreamResult
    pub async fn send_message_stream(
        &self,
        system: &[SystemContent],
        messages: &[ChatMessage],
        tx: mpsc::UnboundedSender<StreamResult>,
    ) -> anyhow::Result<()> {
        let builder = if self.api_format == "openai" {
            self.build_openai_request(messages)
        } else {
            self.build_anthropic_request(system, messages)
        };

        let response = self.set_auth_headers(builder).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("{} {}", status, body);
        }

        if self.api_format == "openai" {
            self.stream_openai(response, tx).await
        } else {
            self.stream_anthropic(response, tx).await
        }
    }

    // ── Anthropic 格式 ──

    fn build_anthropic_request(
        &self,
        system: &[SystemContent],
        messages: &[ChatMessage],
    ) -> reqwest::RequestBuilder {
        let req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: Some(system.to_vec()),
            messages: messages.to_vec(),
            stream: true,
        };
        self.client.post(&self.messages_url).json(&req)
    }

    async fn stream_anthropic(
        &self,
        response: reqwest::Response,
        tx: mpsc::UnboundedSender<StreamResult>,
    ) -> anyhow::Result<()> {
        let mut stream = response.bytes_stream();
        let mut raw_buffer: Vec<u8> = Vec::new();
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut cache_creation_tokens = 0u64;
        let mut cache_read_tokens = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            raw_buffer.extend_from_slice(&chunk);

            let (decoded, valid_up_to) = match std::str::from_utf8(&raw_buffer) {
                Ok(s) => (s.to_string(), raw_buffer.len()),
                Err(e) => {
                    let valid_up_to = e.valid_up_to();
                    if valid_up_to == 0 {
                        continue;
                    }
                    let valid_part = String::from_utf8_lossy(&raw_buffer[..valid_up_to]);
                    (valid_part.to_string(), valid_up_to)
                }
            };

            raw_buffer = raw_buffer[valid_up_to..].to_vec();

            let mut remaining = decoded.as_str();
            while let Some(pos) = remaining.find("\n\n") {
                let event_str = remaining[..pos].to_string();
                remaining = &remaining[pos + 2..];

                for line in event_str.lines() {
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                            match event.event_type.as_str() {
                                "content_block_delta" => {
                                    if let Some(delta) = event.delta {
                                        if delta.delta_type == "text_delta" {
                                            if let Some(text) = delta.text {
                                                output_tokens += 1;
                                                let _ = tx.send(StreamResult::Token(text));
                                            }
                                        }
                                    }
                                }
                                "message_start" => {
                                    if let Some(msg) = event.message {
                                        if let Some(usage) = msg.usage {
                                            input_tokens = usage.input_tokens.unwrap_or(0);
                                            cache_creation_tokens =
                                                usage.cache_creation_input_tokens.unwrap_or(0);
                                            cache_read_tokens =
                                                usage.cache_read_input_tokens.unwrap_or(0);
                                        }
                                    }
                                }
                                "message_delta" => {
                                    if let Some(msg) = event.message {
                                        if let Some(usage) = msg.usage {
                                            output_tokens = usage.output_tokens.unwrap_or(0);
                                        }
                                    }
                                }
                                "message_stop" => {
                                    let _ = tx.send(StreamResult::Done {
                                        input_tokens,
                                        output_tokens,
                                        cache_creation_tokens,
                                        cache_read_tokens,
                                    });
                                    return Ok(());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        let _ = tx.send(StreamResult::Done {
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
        });
        Ok(())
    }

    // ── OpenAI 格式 ──

    fn build_openai_request(
        &self,
        messages: &[ChatMessage],
    ) -> reqwest::RequestBuilder {
        // 将 system prompt 作为第一条 system 消息
        let mut openai_msgs: Vec<OpenAIMessage> = Vec::with_capacity(messages.len() + 1);
        for msg in messages {
            if msg.role == "system" {
                openai_msgs.insert(0, OpenAIMessage {
                    role: "system".to_string(),
                    content: msg.content.as_str().to_string(),
                });
            } else {
                openai_msgs.push(OpenAIMessage {
                    role: msg.role.clone(),
                    content: msg.content.as_str().to_string(),
                });
            }
        }

        let req = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_msgs,
            max_tokens: self.max_tokens,
            stream: true,
        };
        self.client.post(&self.messages_url).json(&req)
    }

    async fn stream_openai(
        &self,
        response: reqwest::Response,
        tx: mpsc::UnboundedSender<StreamResult>,
    ) -> anyhow::Result<()> {
        let mut stream = response.bytes_stream();
        let mut raw_buffer: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            raw_buffer.extend_from_slice(&chunk);

            let (decoded, valid_up_to) = match std::str::from_utf8(&raw_buffer) {
                Ok(s) => (s.to_string(), raw_buffer.len()),
                Err(e) => {
                    let valid_up_to = e.valid_up_to();
                    if valid_up_to == 0 {
                        continue;
                    }
                    let valid_part = String::from_utf8_lossy(&raw_buffer[..valid_up_to]);
                    (valid_part.to_string(), valid_up_to)
                }
            };

            raw_buffer = raw_buffer[valid_up_to..].to_vec();

            let mut remaining = decoded.as_str();
            while let Some(pos) = remaining.find("\n\n") {
                let event_str = remaining[..pos].to_string();
                remaining = &remaining[pos + 2..];

                for line in event_str.lines() {
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            let _ = tx.send(StreamResult::Done {
                                input_tokens: 0,
                                output_tokens: 0,
                                cache_creation_tokens: 0,
                                cache_read_tokens: 0,
                            });
                            return Ok(());
                        }
                        if let Ok(event) = serde_json::from_str::<OpenAIStreamEvent>(data) {
                            if let Some(choices) = event.choices {
                                for choice in choices {
                                    if let Some(delta) = choice.delta {
                                        if let Some(text) = delta.content {
                                            if !text.is_empty() {
                                                let _ = tx.send(StreamResult::Token(text));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 流意外结束
        let _ = tx.send(StreamResult::Done {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        });
        Ok(())
    }
}
