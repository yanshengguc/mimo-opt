use std::collections::HashMap;
use std::path::PathBuf;

/// 技能条目：支持 string 和 object 两种格式（向后兼容）
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum SkillEntry {
    /// 旧格式: "name": "command"
    Simple(String),
    /// 新格式: "name": { "cmd": "...", "desc": "...", "analyze": true }
    Detailed {
        cmd: String,
        #[serde(default)]
        desc: Option<String>,
        #[serde(default = "default_true")]
        analyze: bool,
    },
}

fn default_true() -> bool {
    true
}

impl SkillEntry {
    pub fn cmd(&self) -> &str {
        match self {
            SkillEntry::Simple(c) => c,
            SkillEntry::Detailed { cmd, .. } => cmd,
        }
    }

    pub fn desc(&self) -> Option<&str> {
        match self {
            SkillEntry::Simple(_) => None,
            SkillEntry::Detailed { desc, .. } => desc.as_deref(),
        }
    }

    pub fn should_analyze(&self) -> bool {
        match self {
            SkillEntry::Simple(_) => true,
            SkillEntry::Detailed { analyze, .. } => *analyze,
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub provider: String, // "mimo" | "deepseek" | "openai" | "custom"
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub auth_type: String,  // "anthropic" | "bearer"
    pub api_format: String, // "anthropic" | "openai"
    pub max_tokens: u32,
    pub theme: String,             // "tokyo-night" | "nord" | "catppuccin"
    pub proxy_url: Option<String>, // e.g. "http://127.0.0.1:7890" or "socks5://127.0.0.1:1080"
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub web_search: WebSearchConfig,
    pub skills: HashMap<String, SkillEntry>,
}

#[derive(Clone, Debug)]
pub struct WebSearchConfig {
    pub enabled: bool,
    pub engine: String,     // "ddg" | "deepseek"
    pub max_results: usize, // 1-10
    pub timeout_secs: u64,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engine: "ddg".to_string(),
            max_results: 5,
            timeout_secs: 10,
        }
    }
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
            .field("theme", &self.theme)
            .field("proxy_url", &self.proxy_url)
            .field("temperature", &self.temperature)
            .field("top_p", &self.top_p)
            .field("web_search", &self.web_search)
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

/// Per-model pricing info
#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub name: &'static str,
    pub desc: &'static str,         // 用户可读的一句话描述
    pub input_price_per_mtok: f64,  // ¥ per million input tokens
    pub output_price_per_mtok: f64, // ¥ per million output tokens
}

/// Provider presets
pub struct ProviderPreset {
    pub name: &'static str,
    pub base_url: &'static str,
    pub models: &'static [ModelInfo],
    pub default_model_index: usize,
    pub auth_type: &'static str,
    pub api_format: &'static str,
}

impl ProviderPreset {
    pub fn default_model(&self) -> &'static ModelInfo {
        &self.models[self.default_model_index]
    }

    pub fn find_model(&self, model_name: &str) -> Option<&'static ModelInfo> {
        self.models.iter().find(|m| m.name == model_name)
    }
}

pub const PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset {
        name: "mimo",
        base_url: "https://token-plan-sgp.xiaomimimo.com/anthropic",
        models: &[
            ModelInfo {
                name: "mimo-v2-flash",
                desc: "轻量快速，日常对话",
                input_price_per_mtok: 2.0,
                output_price_per_mtok: 8.0,
            },
            ModelInfo {
                name: "mimo-v2-pro",
                desc: "专业推理，复杂任务",
                input_price_per_mtok: 6.0,
                output_price_per_mtok: 24.0,
            },
        ],
        default_model_index: 0,
        auth_type: "anthropic",
        api_format: "anthropic",
    },
    ProviderPreset {
        name: "deepseek",
        base_url: "https://api.deepseek.com",
        models: &[
            ModelInfo {
                name: "deepseek-chat",
                desc: "V3 标准对话，性价比高",
                input_price_per_mtok: 1.0,
                output_price_per_mtok: 2.0,
            },
            ModelInfo {
                name: "deepseek-reasoner",
                desc: "R1 深度推理，数学/代码/逻辑",
                input_price_per_mtok: 4.0,
                output_price_per_mtok: 16.0,
            },
            ModelInfo {
                name: "deepseek-v4-flash",
                desc: "V4 轻量快速，日常高频",
                input_price_per_mtok: 1.5,
                output_price_per_mtok: 3.0,
            },
            ModelInfo {
                name: "deepseek-v4-pro",
                desc: "V4 旗舰，全能最强",
                input_price_per_mtok: 6.0,
                output_price_per_mtok: 24.0,
            },
        ],
        default_model_index: 0,
        auth_type: "bearer",
        api_format: "openai",
    },
    ProviderPreset {
        name: "openai",
        base_url: "https://api.openai.com",
        models: &[
            ModelInfo {
                name: "gpt-4o-mini",
                desc: "轻量快速，日常使用",
                input_price_per_mtok: 1.25,
                output_price_per_mtok: 5.0,
            },
            ModelInfo {
                name: "gpt-4o",
                desc: "全能旗舰，多模态",
                input_price_per_mtok: 2.5,
                output_price_per_mtok: 10.0,
            },
            ModelInfo {
                name: "gpt-4-turbo",
                desc: "高性能推理",
                input_price_per_mtok: 10.0,
                output_price_per_mtok: 30.0,
            },
        ],
        default_model_index: 0,
        auth_type: "bearer",
        api_format: "openai",
    },
];

impl Config {
    pub fn find_preset(name: &str) -> Option<&'static ProviderPreset> {
        PROVIDERS.iter().find(|p| p.name == name)
    }

    pub fn current_preset(&self) -> Option<&'static ProviderPreset> {
        Self::find_preset(&self.provider)
    }

    /// Look up pricing for the currently configured model.
    /// Falls back: exact model match → provider default → hardcoded 2.0/8.0.
    pub fn model_info(&self) -> Option<&'static ModelInfo> {
        self.current_preset()
            .and_then(|p| p.find_model(&self.model))
    }

    pub fn input_price(&self) -> f64 {
        self.model_info()
            .map(|m| m.input_price_per_mtok)
            .or_else(|| {
                self.current_preset()
                    .map(|p| p.default_model().input_price_per_mtok)
            })
            .unwrap_or(2.0)
    }

    pub fn output_price(&self) -> f64 {
        self.model_info()
            .map(|m| m.output_price_per_mtok)
            .or_else(|| {
                self.current_preset()
                    .map(|p| p.default_model().output_price_per_mtok)
            })
            .unwrap_or(8.0)
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

    fn default_skills() -> HashMap<String, SkillEntry> {
        let mut m = HashMap::new();
        m.insert(
            "lint".into(),
            SkillEntry::Detailed {
                cmd: "cargo clippy 2>&1".into(),
                desc: Some("Rust 代码检查".into()),
                analyze: true,
            },
        );
        m.insert(
            "test".into(),
            SkillEntry::Detailed {
                cmd: "cargo test 2>&1".into(),
                desc: Some("运行单元测试".into()),
                analyze: true,
            },
        );
        m.insert(
            "build".into(),
            SkillEntry::Detailed {
                cmd: "cargo build 2>&1".into(),
                desc: Some("编译项目".into()),
                analyze: true,
            },
        );
        m.insert(
            "git".into(),
            SkillEntry::Detailed {
                cmd: "git log --oneline -10".into(),
                desc: Some("最近 10 条提交".into()),
                analyze: false,
            },
        );
        m.insert(
            "diff".into(),
            SkillEntry::Detailed {
                cmd: "git diff".into(),
                desc: Some("查看代码差异".into()),
                analyze: false,
            },
        );
        m
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;

        let default_preset = &PROVIDERS[0]; // mimo

        if path.exists() {
            log::info!("加载配置: {}", path.display());
            let content = std::fs::read_to_string(&path)?;
            let raw: ConfigRaw = serde_json::from_str(&content)?;
            Ok(Self {
                provider: raw.provider.unwrap_or_else(|| "custom".to_string()),
                api_key: raw.api_key.unwrap_or_default(),
                base_url: raw
                    .base_url
                    .unwrap_or_else(|| default_preset.base_url.to_string()),
                model: raw
                    .model
                    .unwrap_or_else(|| default_preset.default_model().name.to_string()),
                auth_type: raw
                    .auth_type
                    .unwrap_or_else(|| default_preset.auth_type.to_string()),
                api_format: raw
                    .api_format
                    .unwrap_or_else(|| default_preset.api_format.to_string()),
                max_tokens: raw.max_tokens.unwrap_or(4096),
                theme: raw.theme.unwrap_or_else(|| "tokyo-night".to_string()),
                proxy_url: raw.proxy_url,
                temperature: raw.temperature,
                top_p: raw.top_p,
                web_search: raw
                    .web_search
                    .map(|w| WebSearchConfig {
                        enabled: w.enabled.unwrap_or(true),
                        engine: w.engine.unwrap_or_else(|| "ddg".to_string()),
                        max_results: w.max_results.map(|n| n.clamp(1, 10)).unwrap_or(5),
                        timeout_secs: w.timeout_secs.unwrap_or(10),
                    })
                    .unwrap_or_default(),
                skills: raw
                    .skills
                    .map(|m| {
                        m.into_iter()
                            .map(|(k, v)| match v {
                                SkillEntry::Simple(cmd) => (
                                    k,
                                    SkillEntry::Detailed {
                                        cmd,
                                        desc: None,
                                        analyze: true,
                                    },
                                ),
                                other => (k, other),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        } else {
            // 创建默认配置
            let dir = path.parent().expect("config path has no parent");
            std::fs::create_dir_all(dir)?;

            let default = ConfigRaw {
                provider: Some(default_preset.name.to_string()),
                api_key: Some(String::new()),
                base_url: Some(default_preset.base_url.to_string()),
                model: Some(default_preset.default_model().name.to_string()),
                auth_type: Some(default_preset.auth_type.to_string()),
                api_format: Some(default_preset.api_format.to_string()),
                max_tokens: Some(4096),
                theme: Some("tokyo-night".to_string()),
                proxy_url: None,
                temperature: None,
                top_p: None,
                web_search: None,
                skills: None,
            };

            let content = serde_json::to_string_pretty(&default)?;
            std::fs::write(&path, content)?;

            Ok(Self {
                provider: default_preset.name.to_string(),
                api_key: String::new(),
                base_url: default_preset.base_url.to_string(),
                model: default_preset.default_model().name.to_string(),
                auth_type: default_preset.auth_type.to_string(),
                api_format: default_preset.api_format.to_string(),
                max_tokens: 4096,
                theme: "tokyo-night".to_string(),
                proxy_url: None,
                temperature: None,
                top_p: None,
                web_search: WebSearchConfig::default(),
                skills: HashMap::new(),
            })
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        log::info!("保存配置: {}", path.display());
        let raw = ConfigRaw {
            provider: Some(self.provider.clone()),
            api_key: Some(self.api_key.clone()),
            base_url: Some(self.base_url.clone()),
            model: Some(self.model.clone()),
            auth_type: Some(self.auth_type.clone()),
            api_format: Some(self.api_format.clone()),
            max_tokens: Some(self.max_tokens),
            theme: Some(self.theme.clone()),
            proxy_url: self.proxy_url.clone(),
            temperature: self.temperature,
            top_p: self.top_p,
            web_search: Some(WebSearchConfigRaw {
                enabled: Some(self.web_search.enabled),
                engine: Some(self.web_search.engine.clone()),
                max_results: Some(self.web_search.max_results),
                timeout_secs: Some(self.web_search.timeout_secs),
            }),
            skills: if self.skills.is_empty() {
                None
            } else {
                Some(self.skills.clone())
            },
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
    theme: Option<String>,
    proxy_url: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    web_search: Option<WebSearchConfigRaw>,
    skills: Option<HashMap<String, SkillEntry>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct WebSearchConfigRaw {
    enabled: Option<bool>,
    engine: Option<String>,
    max_results: Option<usize>,
    timeout_secs: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_key_empty() {
        assert_eq!(mask_key(""), "(empty)");
    }

    #[test]
    fn mask_key_short() {
        assert_eq!(mask_key("abc"), "***");
        assert_eq!(mask_key("12345678"), "***");
    }

    #[test]
    fn mask_key_normal() {
        let masked = mask_key("tp-1234567890abcdef");
        assert!(masked.starts_with("tp-1"));
        assert!(masked.ends_with("cdef"));
        assert!(!masked.contains("12345678"));
        assert!(masked.contains("..."));
    }

    #[test]
    fn mask_key_exactly_9_chars() {
        let masked = mask_key("123456789");
        assert_eq!(masked, "1234...6789");
    }

    #[test]
    fn default_skills_non_empty() {
        let skills = Config::default_skills();
        assert!(!skills.is_empty());
        assert!(skills.contains_key("lint"));
        assert!(skills.contains_key("test"));
        assert!(skills.contains_key("build"));
    }

    #[test]
    fn find_preset_known() {
        assert!(Config::find_preset("mimo").is_some());
        assert!(Config::find_preset("deepseek").is_some());
        assert!(Config::find_preset("openai").is_some());
    }

    #[test]
    fn find_preset_unknown() {
        assert!(Config::find_preset("nonexistent").is_none());
    }

    #[test]
    fn provider_preset_default_model() {
        let mimo = Config::find_preset("mimo").unwrap();
        let default = mimo.default_model();
        assert_eq!(default.name, "mimo-v2-flash");
    }

    #[test]
    fn provider_preset_find_model() {
        let ds = Config::find_preset("deepseek").unwrap();
        assert!(ds.find_model("deepseek-chat").is_some());
        assert!(ds.find_model("deepseek-reasoner").is_some());
        assert!(ds.find_model("nonexistent").is_none());
    }

    #[test]
    fn model_info_pricing() {
        let ds = Config::find_preset("deepseek").unwrap();
        let chat = ds.find_model("deepseek-chat").unwrap();
        assert_eq!(chat.input_price_per_mtok, 1.0);
        assert_eq!(chat.output_price_per_mtok, 2.0);

        let reasoner = ds.find_model("deepseek-reasoner").unwrap();
        assert_eq!(reasoner.input_price_per_mtok, 4.0);
        assert_eq!(reasoner.output_price_per_mtok, 16.0);
    }

    #[test]
    fn web_search_config_default() {
        let ws = WebSearchConfig::default();
        assert!(ws.enabled);
        assert_eq!(ws.engine, "ddg");
        assert_eq!(ws.max_results, 5);
        assert_eq!(ws.timeout_secs, 10);
    }

    #[test]
    fn config_debug_masks_key() {
        let config = Config {
            provider: "mimo".into(),
            api_key: "tp-1234567890abcdef".into(),
            base_url: "https://example.com".into(),
            model: "mimo-v2-flash".into(),
            auth_type: "anthropic".into(),
            api_format: "anthropic".into(),
            max_tokens: 4096,
            theme: "tokyo-night".into(),
            proxy_url: None,
            temperature: None,
            top_p: None,
            web_search: WebSearchConfig::default(),
            skills: HashMap::new(),
        };
        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("1234567890"));
        assert!(debug_str.contains("tp-1"));
        assert!(debug_str.contains("cdef"));
    }

    #[test]
    fn skill_entry_simple_cmd() {
        let entry = SkillEntry::Simple("echo hi".into());
        assert_eq!(entry.cmd(), "echo hi");
        assert!(entry.desc().is_none());
        assert!(entry.should_analyze());
    }

    #[test]
    fn skill_entry_detailed_cmd() {
        let entry = SkillEntry::Detailed {
            cmd: "cargo test".into(),
            desc: Some("run tests".into()),
            analyze: false,
        };
        assert_eq!(entry.cmd(), "cargo test");
        assert_eq!(entry.desc(), Some("run tests"));
        assert!(!entry.should_analyze());
    }

    #[test]
    fn skill_entry_serde_roundtrip() {
        let entry = SkillEntry::Detailed {
            cmd: "git log".into(),
            desc: Some("view log".into()),
            analyze: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: SkillEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cmd(), "git log");
        assert_eq!(back.desc(), Some("view log"));
        assert!(!back.should_analyze());
    }

    #[test]
    fn skill_entry_deserialize_simple_string() {
        let json = "\"cargo build 2>&1\"";
        let entry: SkillEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.cmd(), "cargo build 2>&1");
        assert!(entry.desc().is_none());
        assert!(entry.should_analyze());
    }

    #[test]
    fn default_skills_have_descriptions() {
        let skills = Config::default_skills();
        for (name, entry) in &skills {
            assert!(
                entry.desc().is_some(),
                "default skill '{}' should have a description",
                name
            );
        }
    }
}
