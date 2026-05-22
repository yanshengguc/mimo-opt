use std::io::Read;

use tokio::sync::mpsc;

use crate::api::{ChatMessage, Content, MiMoClient, StreamResult};
use crate::config::Config;

/// Non-interactive pipe mode: read stdin, send to API, stream response to stdout.
pub async fn run_pipe(config: &Config, prompt: Option<&str>) -> anyhow::Result<()> {
    let input = if let Some(p) = prompt {
        p.to_string()
    } else {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| anyhow::anyhow!("读取 stdin 失败: {}", e))?;
        if buf.trim().is_empty() {
            anyhow::bail!("stdin 为空，请提供输入内容");
        }
        buf.trim().to_string()
    };

    let client = MiMoClient::new(
        config.base_url.clone(),
        config.api_key.clone(),
        config.model.clone(),
        config.auth_type.clone(),
        config.api_format.clone(),
        config.max_tokens,
        config.proxy_url.clone(),
        config.temperature,
        config.top_p,
    );

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: Content::text(input),
        cache_control: None,
    }];

    let system: Vec<crate::api::SystemContent> = vec![];
    let (tx, mut rx) = mpsc::channel::<StreamResult>(256);

    let handle =
        tokio::spawn(async move { client.send_message_stream(&system, &messages, tx).await });

    while let Some(result) = rx.recv().await {
        match result {
            StreamResult::Token(t) => {
                print!("{}", t);
            }
            StreamResult::Done { .. } => break,
            StreamResult::Error(e) => {
                eprintln!("\n错误: {}", e);
                break;
            }
        }
    }

    handle.abort();
    println!();
    Ok(())
}
