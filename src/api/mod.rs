mod types;

use std::sync::RwLock;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc;

pub use types::*;

/// Changeable client settings (behind RwLock for runtime /model /provider)
struct ClientSettings {
    model: String,
    base_url: String,
    api_key: String,
    auth_type: String,
    api_format: String,
    max_tokens: u32,
    messages_url: String,
}

pub struct MiMoClient {
    client: reqwest::Client,
    settings: RwLock<ClientSettings>,
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

        let endpoint = if api_format == "openai" {
            "/v1/chat/completions"
        } else {
            "/v1/messages"
        };
        let messages_url = format!("{}{}", base_url, endpoint);

        Self {
            client,
            settings: RwLock::new(ClientSettings {
                model,
                base_url,
                api_key,
                auth_type,
                api_format,
                max_tokens,
                messages_url,
            }),
        }
    }

    /// 运行时切换模型
    pub fn set_model(&self, model: String) {
        let mut s = self.settings.write().unwrap();
        s.model = model;
    }

    /// 运行时更新 provider 设置（重建 endpoint/auth/费率）
    pub fn update_for_provider(&self, preset: &crate::config::ProviderPreset, api_key: String) {
        let mut s = self.settings.write().unwrap();
        s.base_url = preset.base_url.to_string();
        s.model = preset.model.to_string();
        s.auth_type = preset.auth_type.to_string();
        s.api_format = preset.api_format.to_string();
        s.api_key = api_key;
        let endpoint = if s.api_format == "openai" {
            "/v1/chat/completions"
        } else {
            "/v1/messages"
        };
        s.messages_url = format!("{}{}", s.base_url, endpoint);
    }

    /// 根据 auth_type 设置请求头
    fn set_auth_headers(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let s = self.settings.read().unwrap();
        if s.auth_type == "bearer" {
            builder
                .header("authorization", format!("Bearer {}", s.api_key))
                .header("content-type", "application/json")
        } else {
            builder
                .header("api-key", &s.api_key)
                .header("content-type", "application/json")
                .header("anthropic-version", "2023-06-01")
        }
    }

    /// 查询 DeepSeek API 余额，返回 (显示文本, 数值金额)
    pub async fn query_deepseek_balance(&self) -> Option<(String, f64)> {
        let url = {
            let s = self.settings.read().unwrap();
            format!("{}/user/balance", s.base_url)
        };
        let builder = self.client.get(&url);
        let builder = self.set_auth_headers(builder);
        match builder.send().await {
            Ok(resp) => {
                let body = resp.text().await.unwrap_or_default();
                log::info!("DeepSeek 余额查询: {}", &body[..body.len().min(200)]);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    // DeepSeek 格式: balance_infos[0].{currency, total_balance}
                    if let Some(info) = v["balance_infos"].as_array().and_then(|infos| infos.first()) {
                        let currency = info["currency"].as_str().unwrap_or("CNY");
                        let currency_symbol = match currency {
                            "CNY" => "¥",
                            "USD" => "$",
                            _ => "",
                        };
                        if let Some(bal) = info["total_balance"].as_str() {
                            if let Ok(amount) = bal.parse::<f64>() {
                                return Some((format!("{}{}", currency_symbol, bal), amount));
                            }
                        }
                    }
                    // 通用格式: balance 数值字段
                    if let Some(bal) = v.get("balance").and_then(|b| b.as_f64()) {
                        return Some((format!("¥{:.2}", bal), bal));
                    }
                }
                None
            }
            Err(e) => {
                log::warn!("DeepSeek 余额查询失败: {}", e);
                None
            }
        }
    }

    #[allow(dead_code)]
    pub fn api_url(&self) -> String {
        self.settings.read().unwrap().messages_url.clone()
    }

    /// 探测 API 是否可用（发一个最小请求，不浪费配额）
    #[allow(dead_code)]
    pub async fn check_api(&self) -> anyhow::Result<()> {
        let builder = {
            let s = self.settings.read().unwrap();
            if s.api_format == "openai" {
                let req = OpenAIRequest {
                    model: s.model.clone(),
                    messages: vec![OpenAIMessage {
                        role: "user".to_string(),
                        content: "hi".to_string(),
                    }],
                    max_tokens: 1,
                    stream: false,
                    stream_options: None,
                };
                self.client.post(&s.messages_url).json(&req)
            } else {
                let req = AnthropicRequest {
                    model: s.model.clone(),
                    max_tokens: 1,
                    system: None,
                    messages: vec![ChatMessage {
                        role: "user".to_string(),
                        content: Content::text("hi"),
                        cache_control: None,
                    }],
                    stream: false,
                };
                self.client.post(&s.messages_url).json(&req)
            }
        };

        let response = self.set_auth_headers(builder).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, truncate_body(&body));
        }
    }

    /// 流式请求，通过 channel 逐 token 返回 StreamResult（含自动重试）
    pub async fn send_message_stream(
        &self,
        system: &[SystemContent],
        messages: &[ChatMessage],
        tx: mpsc::Sender<StreamResult>,
    ) -> anyhow::Result<()> {
        let model = {
            let s = self.settings.read().unwrap();
            s.model.clone()
        };
        log::info!("发送请求: model={}, msgs={}", model, messages.len());

        let is_openai = {
            let s = self.settings.read().unwrap();
            s.api_format == "openai"
        };

        for attempt in 0u32..3 {
            if attempt > 0 {
                let delay = 2u64.pow(attempt);
                log::warn!("重试 {}/3，等待 {}s", attempt, delay);
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }

            let builder = {
                let s = self.settings.read().unwrap();
                if s.api_format == "openai" {
                    self.build_openai_request(system, messages, &s)
                } else {
                    self.build_anthropic_request(system, messages, &s)
                }
            };

            let response = match self.set_auth_headers(builder).send().await {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("网络错误: {}", e);
                    if attempt < 2 {
                        continue;
                    }
                    return Err(e.into());
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                log::warn!("API 错误: HTTP {} {}", status, truncate_body(&body));
                if attempt < 2 && is_retryable(status.as_u16()) {
                    continue;
                }
                return Err(anyhow::anyhow!("HTTP {}: {}", status, truncate_body(&body)));
            }

            log::debug!("开始接收流式响应");
            if is_openai {
                return self.stream_openai(response, tx).await;
            } else {
                return self.stream_anthropic(response, tx).await;
            }
        }

        unreachable!()
    }

    // ── Anthropic 格式 ──

    fn build_anthropic_request(
        &self,
        system: &[SystemContent],
        messages: &[ChatMessage],
        s: &ClientSettings,
    ) -> reqwest::RequestBuilder {
        let req = AnthropicRequest {
            model: s.model.clone(),
            max_tokens: s.max_tokens,
            system: Some(system.to_vec()),
            messages: messages.to_vec(),
            stream: true,
        };
        self.client.post(&s.messages_url).json(&req)
    }

    async fn stream_anthropic(
        &self,
        response: reqwest::Response,
        tx: mpsc::Sender<StreamResult>,
    ) -> anyhow::Result<()> {
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut cache_creation_tokens = 0u64;
        let mut cache_read_tokens = 0u64;

        process_sse_stream(response, &tx, |data| {
            let event: AnthropicStreamEvent = serde_json::from_str(data)?;
            match event.event_type.as_str() {
                "content_block_delta" => {
                    if let Some(delta) = event.delta {
                        if delta.delta_type == "text_delta" {
                            if let Some(text) = delta.text {
                                return Ok(SseAction::Token(text));
                            }
                        }
                    }
                    Ok(SseAction::Continue)
                }
                "message_start" => {
                    if let Some(msg) = event.message {
                        if let Some(usage) = msg.usage {
                            input_tokens = usage.input_tokens.unwrap_or(0);
                            cache_creation_tokens = usage.cache_creation_input_tokens.unwrap_or(0);
                            cache_read_tokens = usage.cache_read_input_tokens.unwrap_or(0);
                        }
                    }
                    Ok(SseAction::Continue)
                }
                "message_delta" => {
                    if let Some(msg) = event.message {
                        if let Some(usage) = msg.usage {
                            output_tokens = usage.output_tokens.unwrap_or(0);
                        }
                    }
                    Ok(SseAction::Continue)
                }
                "message_stop" => Ok(SseAction::Done),
                _ => Ok(SseAction::Continue),
            }
        })
        .await;

        log::info!(
            "流完成(Anthropic): input={}, output={}, cache_read={}",
            input_tokens,
            output_tokens,
            cache_read_tokens
        );
        let _ = tx
            .send(StreamResult::Done {
                input_tokens,
                output_tokens,
                cache_creation_tokens,
                cache_read_tokens,
            })
            .await;
        Ok(())
    }

    // ── OpenAI 格式 ──

    fn build_openai_request(
        &self,
        system: &[SystemContent],
        messages: &[ChatMessage],
        s: &ClientSettings,
    ) -> reqwest::RequestBuilder {
        let mut openai_msgs: Vec<OpenAIMessage> =
            Vec::with_capacity(messages.len() + system.len() + 1);

        for sc in system {
            openai_msgs.push(OpenAIMessage {
                role: "system".to_string(),
                content: sc.text.clone(),
            });
        }

        for msg in messages {
            if msg.role == "system" {
                openai_msgs.insert(
                    0,
                    OpenAIMessage {
                        role: "system".to_string(),
                        content: msg.content.as_str().to_string(),
                    },
                );
            } else {
                openai_msgs.push(OpenAIMessage {
                    role: msg.role.clone(),
                    content: msg.content.as_str().to_string(),
                });
            }
        }

        let req = OpenAIRequest {
            model: s.model.clone(),
            messages: openai_msgs,
            max_tokens: s.max_tokens,
            stream: true,
            stream_options: Some(OpenAIStreamOptions {
                include_usage: true,
            }),
        };
        self.client.post(&s.messages_url).json(&req)
    }

    async fn stream_openai(
        &self,
        response: reqwest::Response,
        tx: mpsc::Sender<StreamResult>,
    ) -> anyhow::Result<()> {
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;

        process_sse_stream(response, &tx, |data| {
            if data == "[DONE]" {
                return Ok(SseAction::Done);
            }
            let event: OpenAIStreamEvent = serde_json::from_str(data)?;
            if let Some(usage) = event.usage {
                input_tokens = usage.input_tokens.unwrap_or(0);
                output_tokens = usage.output_tokens.unwrap_or(0);
            }
            if let Some(choices) = event.choices {
                for choice in choices {
                    if let Some(delta) = choice.delta {
                        if let Some(text) = delta.content {
                            if !text.is_empty() {
                                return Ok(SseAction::Token(text));
                            }
                        }
                    }
                }
            }
            Ok(SseAction::Continue)
        })
        .await;

        log::info!(
            "流完成(OpenAI): input={}, output={}",
            input_tokens,
            output_tokens
        );
        let _ = tx
            .send(StreamResult::Done {
                input_tokens,
                output_tokens,
                cache_creation_tokens: 0,
                cache_read_tokens: 0,
            })
            .await;
        Ok(())
    }
}

// ── SSE 流式解析（共用） ──

enum SseAction {
    Token(String),
    Done,
    Continue,
}

/// 通用 SSE 流处理器：UTF-8 安全解码 + 事件分割 + 调用 handler 处理每个 data 行
async fn process_sse_stream<F>(
    response: reqwest::Response,
    tx: &mpsc::Sender<StreamResult>,
    mut handler: F,
) where
    F: FnMut(&str) -> anyhow::Result<SseAction>,
{
    let mut stream = response.bytes_stream();
    let mut raw_buffer: Vec<u8> = Vec::new();
    let mut partial: String = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(_) => break,
        };
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

        let mut remaining = partial.clone();
        remaining.push_str(&decoded);
        partial.clear();

        while let Some((pos, sep_len)) = find_sse_separator(&remaining) {
            let event_str = remaining[..pos].to_string();
            remaining = remaining[pos + sep_len..].to_string();

            for line in event_str.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    match handler(data) {
                        Ok(SseAction::Token(text)) => {
                            let _ = tx.send(StreamResult::Token(text)).await;
                        }
                        Ok(SseAction::Done) => return,
                        Ok(SseAction::Continue) => {}
                        Err(_) => {}
                    }
                }
            }
        }

        partial = remaining;
    }
}

/// 判断 HTTP 错误是否可重试（5xx、429 rate limit）
fn is_retryable(status: u16) -> bool {
    status >= 500 || status == 429
}

fn truncate_body(body: &str) -> String {
    let body = body.trim();
    if body.len() <= 200 {
        body.to_string()
    } else {
        format!("{}...", &body[..197])
    }
}

/// Find the next SSE event separator (\n\n or \r\n\r\n).
/// Returns (position, separator_length_in_bytes).
fn find_sse_separator(text: &str) -> Option<(usize, usize)> {
    let nn = text.find("\n\n");
    let rnrn = text.find("\r\n\r\n");
    match (nn, rnrn) {
        (Some(n), Some(r)) => {
            if n < r {
                Some((n, 2))
            } else {
                Some((r, 4))
            }
        }
        (Some(n), None) => Some((n, 2)),
        (None, Some(r)) => Some((r, 4)),
        (None, None) => None,
    }
}
