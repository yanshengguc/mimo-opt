use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub auth_type: String, // "anthropic" | "bearer"
    pub skills: HashMap<String, String>,
}

impl Config {
    pub fn config_path() -> anyhow::Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取配置目录"))?
            .join("mimo-opt");
        Ok(dir.join("config.json"))
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let raw: ConfigRaw = serde_json::from_str(&content)?;
            Ok(Self {
                api_key: raw.api_key.unwrap_or_default(),
                base_url: raw.base_url.unwrap_or_else(|| {
                    "https://token-plan-sgp.xiaomimimo.com/anthropic".to_string()
                }),
                model: raw.model.unwrap_or_else(|| "mimo-v2-flash".to_string()),
                auth_type: raw.auth_type.unwrap_or_else(|| "anthropic".to_string()),
                skills: raw.skills.unwrap_or_default(),
            })
        } else {
            // 创建默认配置
            let dir = path.parent().unwrap();
            std::fs::create_dir_all(dir)?;

            let default = ConfigRaw {
                api_key: Some(String::new()),
                base_url: Some(
                    "https://token-plan-sgp.xiaomimimo.com/anthropic".to_string(),
                ),
                model: Some("mimo-v2-flash".to_string()),
                auth_type: Some("anthropic".to_string()),
                skills: None,
            };

            let content = serde_json::to_string_pretty(&default)?;
            std::fs::write(&path, content)?;

            Ok(Self {
                api_key: String::new(),
                base_url: "https://token-plan-sgp.xiaomimimo.com/anthropic".to_string(),
                model: "mimo-v2-flash".to_string(),
                auth_type: "anthropic".to_string(),
                skills: HashMap::new(),
            })
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigRaw {
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    auth_type: Option<String>,
    skills: Option<HashMap<String, String>>,
}
