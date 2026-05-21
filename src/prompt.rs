use crate::api::{CacheControl, ChatMessage, SystemContent};
use crate::util::now_secs;

pub fn chrono_date() -> String {
    let total_secs = now_secs();
    let days_since_epoch = total_secs / 86400;
    let date = chrono_from_days(days_since_epoch);
    format!("{:04}/{:02}/{:02}", date.0, date.1, date.2)
}

fn chrono_from_days(days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    let mut d = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let months = [
        31,
        if is_leap(y) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0u64;
    for (i, &days_in_month) in months.iter().enumerate() {
        if d < days_in_month {
            m = i as u64 + 1;
            d += 1;
            break;
        }
        d -= days_in_month;
    }
    (y, m, d)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

pub fn build_system_prompt_text(project_tree: &str, provider: &str) -> String {
    let mut text = String::with_capacity(512);
    match provider {
        "deepseek" => {
            text.push_str("You are DeepSeek, a helpful AI assistant created by DeepSeek. ");
            text.push_str(
                "You can help with coding, analysis, writing, math, and general Q&A.\n\n",
            );
        }
        "openai" => {
            text.push_str("You are ChatGPT, a helpful AI assistant created by OpenAI. ");
            text.push_str(
                "You can help with coding, analysis, writing, math, and general Q&A.\n\n",
            );
        }
        _ => {
            text.push_str(
                "You are MiMo, a helpful AI assistant created by Xiaomi's LLM-Core team. ",
            );
            text.push_str(
                "You can help with coding, analysis, writing, math, and general Q&A.\n\n",
            );
        }
    }
    text.push_str("## Output Format\n");
    text.push_str("- Use markdown for formatting\n");
    text.push_str("- Use code blocks with language tags for code\n");
    text.push_str("- Be concise and direct\n\n");
    text.push_str("## Current Working Directory\n");
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    text.push_str(&cwd);
    text.push('\n');
    if !project_tree.is_empty() {
        text.push_str("\n## Project Files\n");
        text.push_str(project_tree);
    }
    text
}

pub fn build_system_content(system_text: &str) -> Vec<SystemContent> {
    vec![SystemContent {
        content_type: "text".to_string(),
        text: system_text.to_string(),
        cache_control: CacheControl {
            cache_type: "ephemeral".to_string(),
        },
    }]
}

/// 首条 user 消息注入日期（跨天缓存失效保护）
pub fn inject_date_if_needed(messages: &mut [ChatMessage]) {
    if let Some(first) = messages.first_mut() {
        if first.role == "user" && !first.content.as_str().contains("[Current date:") {
            let today = chrono_date();
            first
                .content
                .as_mut_str()
                .push_str(&format!("\n\n[Current date: {}]", today));
        }
    }
}

pub fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    let n = messages.len();
    if n == 0 {
        return;
    }
    messages[0].cache_control = Some(CacheControl {
        cache_type: "ephemeral".to_string(),
    });
    let mut placed = 1;
    for i in (3..n).step_by(5).take(3) {
        messages[i].cache_control = Some(CacheControl {
            cache_type: "ephemeral".to_string(),
        });
        placed += 1;
    }
    log::debug!("缓存断点: {} 条消息, {} 个断点", n, placed);
}
