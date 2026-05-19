use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::api::ChatMessage;
use crate::util::now_secs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub total_input_tokens: u64,
    #[serde(default)]
    pub total_output_tokens: u64,
    #[serde(default)]
    pub total_cache_creation_tokens: u64,
    #[serde(default)]
    pub total_cache_read_tokens: u64,
    #[serde(default)]
    pub total_cost: f64,
}

fn session_dir() -> anyhow::Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("无法获取配置目录"))?
        .join("mimo-opt")
        .join("sessions");
    Ok(dir)
}

impl Session {
    pub fn new(name: String) -> Self {
        let ts = now_secs();
        Self {
            id: format!("{}", ts),
            name,
            messages: Vec::new(),
            created_at: ts,
            updated_at: ts,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cache_creation_tokens: 0,
            total_cache_read_tokens: 0,
            total_cost: 0.0,
        }
    }

    pub fn auto_name() -> String {
        let ts = now_secs();
        let days = ts / 86400;
        let secs_of_day = ts % 86400;
        let h = secs_of_day / 3600;
        let m = (secs_of_day % 3600) / 60;
        // 简易日期（从 1970 天数算年月日太复杂，直接用时间戳缩写）
        format!("session-{}_{:02}{:02}", days, h, m)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = session_dir()?;
        std::fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        }
        let path = dir.join(format!("{}.json", self.id));
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    pub fn load(id: &str) -> anyhow::Result<Self> {
        let dir = session_dir()?;
        let path = dir.join(format!("{}.json", id));
        let content = std::fs::read_to_string(&path)?;
        let session: Self = serde_json::from_str(&content)?;
        Ok(session)
    }

    pub fn list() -> Vec<SessionInfo> {
        let dir = match session_dir() {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        if !dir.exists() {
            return Vec::new();
        }

        let mut list = Vec::new();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.ends_with(".json") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Ok(session) = serde_json::from_str::<Session>(&content) {
                    list.push(SessionInfo {
                        id: session.id,
                        name: session.name,
                        msg_count: session.messages.len(),
                        updated_at: session.updated_at,
                    });
                }
            }
        }

        list.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        list
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub msg_count: usize,
    pub updated_at: u64,
}
