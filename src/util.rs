use std::path::PathBuf;

/// 获取当前 Unix 时间戳（秒）
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 获取当前工作目录，失败时返回错误信息
pub fn get_cwd() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("获取工作目录失败: {}", e))
}
