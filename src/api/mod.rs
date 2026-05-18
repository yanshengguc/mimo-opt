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
    pub auth_type: String, // "anthropic" | "bearer"
    messages_url: String,
}

impl MiMoClient {
    pub fn new(base_url: String, api_key: String, model: String, auth_type: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build reqwest client");
        let messages_url = format!("{}/v1/messages", base_url);
        Self {
            client,
            base_url,
            api_key,
            model,
            auth_type,
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

    fn build_request(
        &self,
        system: Option<&[SystemContent]>,
        messages: Vec<ChatMessage>,
        max_tokens: u32,
        stream: bool,
    ) -> ChatRequest {
        ChatRequest {
            model: self.model.clone(),
            max_tokens,
            system: system.map(|s| s.to_vec()),
            messages,
            stream,
        }
    }

    /// 探测 API 是否可用（发一个最小请求，不浪费配额）
    pub async fn check_api(&self) -> anyhow::Result<()> {
        let request = self.build_request(
            None,
            vec![ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
                cache_control: None,
            }],
            1,
            false,
        );

        let builder = self.client.post(&self.messages_url).json(&request);
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
        let request = self.build_request(
            Some(system),
            messages.to_vec(),
            4096,
            true,
        );

        let builder = self.client.post(&self.messages_url).json(&request);
        let response = self.set_auth_headers(builder).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("{} {}", status, body);
        }

        // 用 bytes 处理避免 UTF-8 跨 chunk 分片问题
        let mut stream = response.bytes_stream();
        let mut raw_buffer: Vec<u8> = Vec::new();
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut cache_creation_tokens = 0u64;
        let mut cache_read_tokens = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            raw_buffer.extend_from_slice(&chunk);

            // 尝试将 buffer 解码为 UTF-8
            // 如果失败说明有跨 chunk 的多字节字符，等下一个 chunk
            let (decoded, valid_up_to) = match std::str::from_utf8(&raw_buffer) {
                Ok(s) => (s.to_string(), raw_buffer.len()),
                Err(e) => {
                    let valid_up_to = e.valid_up_to();
                    if valid_up_to == 0 {
                        // 还没有完整的字符，等下一个 chunk
                        continue;
                    }
                    // 解码有效部分
                    let valid_part = String::from_utf8_lossy(&raw_buffer[..valid_up_to]);
                    (valid_part.to_string(), valid_up_to)
                }
            };

            // 保留未处理的字节
            raw_buffer = raw_buffer[valid_up_to..].to_vec();

            // 解析 SSE 事件
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
                        if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                            match event.event_type.as_str() {
                                "content_block_delta" => {
                                    if let Some(delta) = event.delta {
                                        if delta.delta_type == "text_delta" {
                                            if let Some(text) = delta.text {
                                                output_tokens += 1; // 粗略计数
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
                                    // 最终 usage 在 message_delta 事件中
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

        // 流意外结束（没有收到 message_stop）
        let _ = tx.send(StreamResult::Done {
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
        });
        Ok(())
    }
}
