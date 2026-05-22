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

pub fn build_system_stable(provider: &str) -> String {
    let mut text = String::with_capacity(256);
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
    text.push_str("- Be concise and direct\n");
    text
}

pub fn build_system_dynamic(project_tree: &str) -> String {
    let mut text = String::with_capacity(256);
    text.push_str("\n## Current Working Directory\n");
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

/// System prompt split into 3 blocks for optimal caching:
/// - Block 0 (stable, cached): provider identity + output format rules
/// - Block 1 (dynamic, cached): CWD + project file tree (changes only on dir/file change)
/// - Block 2 (date, NOT cached): current date (changes daily, isolated to avoid polluting block 1)
pub fn build_system_content_split(stable_rules: &str, dynamic_ctx: &str) -> Vec<SystemContent> {
    let date_text = format!("\n## Current Date\n{}", chrono_date());
    vec![
        SystemContent {
            content_type: "text".to_string(),
            text: stable_rules.to_string(),
            cache_control: Some(CacheControl {
                cache_type: "ephemeral".to_string(),
            }),
        },
        SystemContent {
            content_type: "text".to_string(),
            text: dynamic_ctx.to_string(),
            cache_control: Some(CacheControl {
                cache_type: "ephemeral".to_string(),
            }),
        },
        SystemContent {
            content_type: "text".to_string(),
            text: date_text,
            // No cache_control — date changes daily, caching would always miss
            cache_control: None,
        },
    ]
}

pub fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    let n = messages.len();
    if n == 0 {
        return;
    }

    let msgs = messages;
    let m = msgs.len();

    let mk = || CacheControl {
        cache_type: "ephemeral".to_string(),
    };

    let placed = match m {
        1..=3 => {
            msgs[0].cache_control = Some(mk());
            1
        }
        4..=8 => {
            msgs[0].cache_control = Some(mk());
            msgs[m - 1].cache_control = Some(mk());
            2
        }
        9..=16 => {
            msgs[0].cache_control = Some(mk());
            msgs[m / 2].cache_control = Some(mk());
            msgs[m - 1].cache_control = Some(mk());
            3
        }
        _ => {
            msgs[0].cache_control = Some(mk());
            msgs[m / 3].cache_control = Some(mk());
            msgs[2 * m / 3].cache_control = Some(mk());
            msgs[m - 1].cache_control = Some(mk());
            4
        }
    };

    log::debug!("缓存断点: {} 条消息, {} 个断点", n, placed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Content;

    fn make_msg(role: &str, text: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: Content::text(text),
            cache_control: None,
        }
    }

    #[test]
    fn is_leap_basic() {
        assert!(is_leap(2000)); // divisible by 400
        assert!(is_leap(2024)); // divisible by 4, not 100
        assert!(!is_leap(1900)); // divisible by 100, not 400
        assert!(!is_leap(2023));
        assert!(!is_leap(2100));
    }

    #[test]
    fn chrono_date_format() {
        let date = chrono_date();
        // Should be YYYY/MM/DD format
        assert_eq!(date.len(), 10);
        assert_eq!(date.chars().nth(4), Some('/'));
        assert_eq!(date.chars().nth(7), Some('/'));
    }

    #[test]
    fn cache_breakpoints_empty() {
        let mut msgs: Vec<ChatMessage> = vec![];
        apply_cache_breakpoints(&mut msgs);
        // should not panic
    }

    #[test]
    fn cache_breakpoints_single() {
        let mut msgs = vec![make_msg("user", "hello")];
        apply_cache_breakpoints(&mut msgs);
        assert!(msgs[0].cache_control.is_some());
    }

    #[test]
    fn cache_breakpoints_two_messages() {
        let mut msgs = vec![make_msg("user", "hello"), make_msg("assistant", "hi")];
        apply_cache_breakpoints(&mut msgs);
        assert!(msgs[0].cache_control.is_some());
        assert!(msgs[1].cache_control.is_none());
    }

    #[test]
    fn cache_breakpoints_medium_4_to_8() {
        // 5 messages: offset=0 → msgs[0] + msgs[m-1]=msgs[4]
        let mut msgs: Vec<ChatMessage> = (0..5)
            .map(|i| make_msg("user", &format!("msg{}", i)))
            .collect();
        apply_cache_breakpoints(&mut msgs);
        assert!(msgs[0].cache_control.is_some());
        assert!(msgs[4].cache_control.is_some()); // m-1
        assert!(msgs[1].cache_control.is_none());
        assert!(msgs[2].cache_control.is_none());
    }

    #[test]
    fn cache_breakpoints_long_17_plus() {
        // 20 messages: should place 4 breakpoints [0], [m/3], [2m/3], [m-1]
        let mut msgs: Vec<ChatMessage> = (0..20)
            .map(|i| make_msg("user", &format!("msg{}", i)))
            .collect();
        apply_cache_breakpoints(&mut msgs);
        let bp_count = msgs.iter().filter(|m| m.cache_control.is_some()).count();
        assert_eq!(bp_count, 4);
        assert!(msgs[0].cache_control.is_some());
        assert!(msgs[6].cache_control.is_some()); // 20/3 = 6
        assert!(msgs[13].cache_control.is_some()); // 2*20/3 = 13
        assert!(msgs[19].cache_control.is_some()); // 20-1 = 19
    }

    #[test]
    fn cache_breakpoints_max_4_markers() {
        // Even with 50 messages, should never exceed 4 breakpoints
        let mut msgs: Vec<ChatMessage> = (0..50)
            .map(|i| make_msg("user", &format!("msg{}", i)))
            .collect();
        apply_cache_breakpoints(&mut msgs);
        let bp_count = msgs.iter().filter(|m| m.cache_control.is_some()).count();
        assert!(bp_count <= 4);
    }

    #[test]
    fn build_system_stable_deepseek() {
        let text = build_system_stable("deepseek");
        assert!(text.contains("DeepSeek"));
    }

    #[test]
    fn build_system_stable_openai() {
        let text = build_system_stable("openai");
        assert!(text.contains("ChatGPT"));
    }

    #[test]
    fn build_system_stable_default() {
        let text = build_system_stable("mimo");
        assert!(text.contains("MiMo"));
    }

    #[test]
    fn build_system_content_split_three_blocks() {
        let content = build_system_content_split("stable rules", "dynamic ctx");
        assert_eq!(content.len(), 3);
        assert_eq!(content[0].text, "stable rules");
        assert_eq!(content[1].text, "dynamic ctx");
        assert!(content[2].text.contains("Current Date"));
        assert_eq!(
            content[0].cache_control.as_ref().unwrap().cache_type,
            "ephemeral"
        );
        assert_eq!(
            content[1].cache_control.as_ref().unwrap().cache_type,
            "ephemeral"
        );
        assert!(content[2].cache_control.is_none());
    }
}
