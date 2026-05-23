use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::api::ChatMessage;
use crate::util::now_secs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub messages: Arc<Vec<ChatMessage>>,
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
            id: ts.to_string(),
            name,
            messages: Arc::new(Vec::new()),
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
        // 用当前工作目录名作为会话名，比 epoch 天数更友好
        let dir_name = std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "default".to_string());
        let ts = now_secs();
        let secs_of_day = ts % 86400;
        let h = secs_of_day / 3600;
        let m = (secs_of_day % 3600) / 60;
        format!("{}_{:02}{:02}", dir_name, h, m)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = session_dir()?;
        std::fs::create_dir_all(&dir)?;
        log::debug!("保存会话: {} ({} 条消息)", self.id, self.messages.len());
        crate::util::restrict_permissions(&dir, true);
        let path = dir.join(format!("{}.json", self.id));
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        crate::util::restrict_permissions(&path, false);
        Ok(())
    }

    pub fn load(id: &str) -> anyhow::Result<Self> {
        let dir = session_dir()?;
        let path = dir.join(format!("{}.json", id));
        log::debug!("加载会话: {}", id);
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
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub msg_count: usize,
    pub updated_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_new_sets_fields() {
        let s = Session::new("test-session".into());
        assert_eq!(s.name, "test-session");
        assert!(s.messages.is_empty());
        assert_eq!(s.total_input_tokens, 0);
        assert_eq!(s.total_output_tokens, 0);
        assert_eq!(s.total_cost, 0.0);
        assert!(s.created_at > 0);
        assert_eq!(s.created_at, s.updated_at);
    }

    #[test]
    fn session_id_is_timestamp() {
        let s = Session::new("test".into());
        let id_num: u64 = s.id.parse().unwrap();
        assert!(id_num > 1_577_836_800); // after 2020
    }

    #[test]
    fn auto_name_format() {
        let name = Session::auto_name();
        // Should contain underscore separator
        assert!(name.contains('_'));
        // The HHMM part should be 4 digits at the end
        let parts: Vec<&str> = name.split('_').collect();
        assert!(parts.len() >= 2);
        let time_part = parts.last().unwrap();
        assert_eq!(time_part.len(), 4);
        // All chars in time part should be digits
        assert!(time_part.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn session_serialization_roundtrip() {
        let mut s = Session::new("roundtrip-test".into());
        // Arc<Vec<ChatMessage>> should serialize properly
        let msg = crate::api::ChatMessage {
            role: "user".to_string(),
            content: crate::api::Content::text("hello"),
            cache_control: None,
        };
        Arc::make_mut(&mut s.messages).push(msg);

        let json = serde_json::to_string_pretty(&s).unwrap();
        let loaded: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.name, "roundtrip-test");
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content.as_str(), "hello");
    }

    #[test]
    fn session_token_fields_default() {
        // Test that #[serde(default)] works for token fields
        let json = r#"{"id":"123","name":"test","messages":[],"created_at":100,"updated_at":100}"#;
        let s: Session = serde_json::from_str(json).unwrap();
        assert_eq!(s.total_input_tokens, 0);
        assert_eq!(s.total_output_tokens, 0);
        assert_eq!(s.total_cache_creation_tokens, 0);
        assert_eq!(s.total_cache_read_tokens, 0);
        assert_eq!(s.total_cost, 0.0);
    }
}
