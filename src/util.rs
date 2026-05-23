use std::path::PathBuf;

/// 获取当前 Unix 时间戳（秒）
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|e| {
            log::warn!("系统时钟异常，使用默认时间戳 0: {:?}", e);
            std::time::Duration::from_secs(0)
        })
        .as_secs()
}

/// 获取当前工作目录，失败时返回错误信息
pub fn get_cwd() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("获取工作目录失败: {}", e))
}

/// 粗略估算输入文本的 token 数（中文 ~1.5 tok/字，英文 ~0.75 tok/字）
pub fn estimate_tokens(text: &str) -> usize {
    let cjk = text
        .chars()
        .filter(|c| matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3040}'..='\u{30FF}' | '\u{AC00}'..='\u{D7AF}'))
        .count();
    let ascii = text.chars().count() - cjk;
    (cjk as f64 * 1.5 + ascii as f64 * 0.75) as usize
}

/// 格式化费用预估显示
pub fn format_cost_estimate(tokens: usize, input_price_per_mtok: f64) -> String {
    let cost = tokens as f64 / 1_000_000.0 * input_price_per_mtok;
    if cost >= 0.01 {
        format!("~¥{:.2}", cost)
    } else {
        format!("~¥{:.4}", cost)
    }
}

/// 限制文件/目录权限：仅当前用户可读写
pub fn restrict_permissions(path: &std::path::Path, is_dir: bool) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if is_dir { 0o700 } else { 0o600 };
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    #[cfg(windows)]
    {
        let username = match std::env::var("USERNAME") {
            Ok(u) => u,
            Err(_) => return,
        };
        let grant = if is_dir {
            format!("{}:(OI)(CI)F", username)
        } else {
            format!("{}:F", username)
        };
        let _ = std::process::Command::new("icacls")
            .arg(path.as_os_str())
            .args(["/inheritance:r", "/grant:r"])
            .arg(&grant)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// 掩码 API key：显示前4后4位，中间用 ...
pub fn mask_key(key: &str) -> String {
    if key.is_empty() {
        return "(empty)".into();
    }
    if key.len() <= 8 {
        return "***".into();
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
}

/// 将 Unix 时间戳格式化为相对时间（刚刚 / X分钟 / X小时 / X天）
pub fn format_age(timestamp: u64) -> String {
    let now = now_secs();
    let diff = now.saturating_sub(timestamp);
    if diff < 60 {
        "刚刚".into()
    } else if diff < 3600 {
        format!("{}分钟前", diff / 60)
    } else if diff < 86400 {
        format!("{}小时前", diff / 3600)
    } else {
        format!("{}天前", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_pure_ascii() {
        // 10 ASCII chars * 0.75 = 7.5 → 7
        assert_eq!(estimate_tokens("abcdefghij"), 7);
    }

    #[test]
    fn estimate_tokens_pure_cjk() {
        // 4 CJK chars * 1.5 = 6.0 → 6
        assert_eq!(estimate_tokens("中文测试"), 6);
    }

    #[test]
    fn estimate_tokens_mixed() {
        let text = "hello你好";
        // 5 ASCII * 0.75 + 2 CJK * 1.5 = 3.75 + 3.0 = 6.75 → 6
        assert_eq!(estimate_tokens(text), 6);
    }

    #[test]
    fn estimate_tokens_japanese() {
        // hiragana/katakana counted as CJK
        assert!(estimate_tokens("こんにちは") > 0);
    }

    #[test]
    fn format_cost_high() {
        let s = format_cost_estimate(1_000_000, 2.0);
        assert_eq!(s, "~¥2.00");
    }

    #[test]
    fn format_cost_low() {
        let s = format_cost_estimate(100, 2.0);
        assert_eq!(s, "~¥0.0002");
    }

    #[test]
    fn format_cost_zero() {
        let s = format_cost_estimate(0, 2.0);
        assert_eq!(s, "~¥0.0000");
    }

    #[test]
    fn now_secs_returns_reasonable_value() {
        let ts = now_secs();
        // Should be after 2020-01-01 (1577836800) and before 2100
        assert!(ts > 1_577_836_800);
        assert!(ts < 4_102_444_800);
    }

    #[test]
    fn get_cwd_returns_existing_path() {
        let cwd = get_cwd().expect("should get cwd");
        assert!(cwd.exists());
    }

    #[test]
    fn format_age_recent() {
        let now = now_secs();
        assert_eq!(format_age(now), "刚刚");
        assert_eq!(format_age(now - 30), "刚刚");
    }

    #[test]
    fn format_age_minutes() {
        let now = now_secs();
        assert_eq!(format_age(now - 120), "2分钟前");
        assert_eq!(format_age(now - 3599), "59分钟前");
    }

    #[test]
    fn format_age_hours() {
        let now = now_secs();
        assert_eq!(format_age(now - 3600), "1小时前");
        assert_eq!(format_age(now - 72000), "20小时前");
    }

    #[test]
    fn format_age_days() {
        let now = now_secs();
        assert_eq!(format_age(now - 86400), "1天前");
        assert_eq!(format_age(now - 259200), "3天前");
    }
}
