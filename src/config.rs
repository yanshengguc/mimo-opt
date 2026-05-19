use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub provider: String,    // "mimo" | "deepseek" | "openai" | "custom"
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub auth_type: String,   // "anthropic" | "bearer"
    pub api_format: String,  // "anthropic" | "openai"
    pub max_tokens: u32,
    pub skills: HashMap<String, String>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("provider", &self.provider)
            .field("api_key", &mask_key(&self.api_key))
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("auth_type", &self.auth_type)
            .field("api_format", &self.api_format)
            .field("max_tokens", &self.max_tokens)
            .field("skills", &self.skills)
            .finish()
    }
}

fn mask_key(key: &str) -> String {
    if key.is_empty() {
        return "(empty)".into();
    }
    if key.len() <= 8 {
        return "***".into();
    }
    format!("{}...{}", &key[..4], &key[key.len() - 4..])
}

/// Provider presets
pub struct ProviderPreset {
    pub name: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub auth_type: &'static str,
    pub api_format: &'static str,
    pub input_price_per_mtok: f64,  // ¥ per million tokens
    pub output_price_per_mtok: f64,
}

pub const PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset {
        name: "mimo",
        base_url: "https://token-plan-sgp.xiaomimimo.com/anthropic",
        model: "mimo-v2-flash",
        auth_type: "anthropic",
        api_format: "anthropic",
        input_price_per_mtok: 2.0,
        output_price_per_mtok: 8.0,
    },
    ProviderPreset {
        name: "deepseek",
        base_url: "https://api.deepseek.com",
        model: "deepseek-chat",
        auth_type: "bearer",
        api_format: "openai",
        input_price_per_mtok: 1.0,
        output_price_per_mtok: 2.0,
    },
    ProviderPreset {
        name: "openai",
        base_url: "https://api.openai.com",
        model: "gpt-4o-mini",
        auth_type: "bearer",
        api_format: "openai",
        input_price_per_mtok: 1.25,
        output_price_per_mtok: 5.0,
    },
];

impl Config {
    pub fn find_preset(name: &str) -> Option<&'static ProviderPreset> {
        PROVIDERS.iter().find(|p| p.name == name)
    }

    pub fn current_preset(&self) -> Option<&'static ProviderPreset> {
        Self::find_preset(&self.provider)
    }

    pub fn input_price(&self) -> f64 {
        self.current_preset().map(|p| p.input_price_per_mtok).unwrap_or(2.0)
    }

    pub fn output_price(&self) -> f64 {
        self.current_preset().map(|p| p.output_price_per_mtok).unwrap_or(8.0)
    }

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

        let default_preset = &PROVIDERS[0]; // mimo

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let raw: ConfigRaw = serde_json::from_str(&content)?;
            Ok(Self {
                provider: raw.provider.unwrap_or_else(|| "custom".to_string()),
                api_key: raw.api_key.unwrap_or_default(),
                base_url: raw.base_url.unwrap_or_else(|| default_preset.base_url.to_string()),
                model: raw.model.unwrap_or_else(|| default_preset.model.to_string()),
                auth_type: raw.auth_type.unwrap_or_else(|| default_preset.auth_type.to_string()),
                api_format: raw.api_format.unwrap_or_else(|| default_preset.api_format.to_string()),
                max_tokens: raw.max_tokens.unwrap_or(4096),
                skills: raw.skills.unwrap_or_default(),
            })
        } else {
            // 创建默认配置
            let dir = path.parent().unwrap();
            std::fs::create_dir_all(dir)?;

            let default = ConfigRaw {
                provider: Some(default_preset.name.to_string()),
                api_key: Some(String::new()),
                base_url: Some(default_preset.base_url.to_string()),
                model: Some(default_preset.model.to_string()),
                auth_type: Some(default_preset.auth_type.to_string()),
                api_format: Some(default_preset.api_format.to_string()),
                max_tokens: Some(4096),
                skills: None,
            };

            let content = serde_json::to_string_pretty(&default)?;
            std::fs::write(&path, content)?;

            Ok(Self {
                provider: default_preset.name.to_string(),
                api_key: String::new(),
                base_url: default_preset.base_url.to_string(),
                model: default_preset.model.to_string(),
                auth_type: default_preset.auth_type.to_string(),
                api_format: default_preset.api_format.to_string(),
                max_tokens: 4096,
                skills: HashMap::new(),
            })
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        let raw = ConfigRaw {
            provider: Some(self.provider.clone()),
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigRaw {
    provider: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    auth_type: Option<String>,
    api_format: Option<String>,
    max_tokens: Option<u32>,
    skills: Option<HashMap<String, String>>,
}
