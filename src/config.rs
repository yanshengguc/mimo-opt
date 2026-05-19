use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub auth_type: String,   // "anthropic" | "bearer"
    pub api_format: String,  // "anthropic" | "openai"
    pub max_tokens: u32,
    pub skills: HashMap<String, String>,
}

impl Config {
    pub fn config_path() -> anyhow::Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取配置目录"))?
            .join("mimo-opt");
        Ok(dir.join("config.json"))
    }

    pub fn apply_defaults(&mut self) {
        if self.skills.is_empty() {
            self.skills = Self::default_skills();
        }
    }

    fn default_skills() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("lint".into(), "cargo clippy 2>&1".into());
        m.insert("test".into(), "cargo test 2>&1".into());
        m.insert("build".into(), "cargo build 2>&1".into());
        m.insert("git".into(), "git log --oneline -10".into());
        m.insert("diff".into(), "git diff".into());
        m
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
                api_format: raw.api_format.unwrap_or_else(|| "anthropic".to_string()),
                max_tokens: raw.max_tokens.unwrap_or(4096),
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
                api_format: Some("anthropic".to_string()),
                max_tokens: Some(4096),
                skills: None,
            };

            let content = serde_json::to_string_pretty(&default)?;
            std::fs::write(&path, content)?;

            Ok(Self {
                api_key: String::new(),
                base_url: "https://token-plan-sgp.xiaomimimo.com/anthropic".to_string(),
                model: "mimo-v2-flash".to_string(),
                auth_type: "anthropic".to_string(),
                api_format: "anthropic".to_string(),
                max_tokens: 4096,
                skills: HashMap::new(),
            })
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        let raw = ConfigRaw {
            api_key: Some(self.api_key.clone()),
            base_url: Some(self.base_url.clone()),
            model: Some(self.model.clone()),
            auth_type: Some(self.auth_type.clone()),
            api_format: Some(self.api_format.clone()),
            max_tokens: Some(self.max_tokens),
            skills: if self.skills.is_empty() { None } else { Some(self.skills.clone()) },
        };
        let content = serde_json::to_string_pretty(&raw)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigRaw {
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    auth_type: Option<String>,
    api_format: Option<String>,
    max_tokens: Option<u32>,
    skills: Option<HashMap<String, String>>,
}
