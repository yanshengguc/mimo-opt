use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use tokio::sync::mpsc;

use mimo_opt::api::{ChatMessage, Content, MiMoClient, StreamResult};
use mimo_opt::config::{Config, SkillEntry, PROVIDERS};
use mimo_opt::prompt;
use mimo_opt::search;
use mimo_opt::session::Session;
use mimo_opt::util;

// ── 共享状态 ──

pub struct AppState {
    pub config: std::sync::Mutex<Config>,
    pub client: Arc<MiMoClient>,
    pub project_tree: String,
    pub system_stable: String,
    pub system_dynamic: String,
    pub stream_handle: std::sync::Mutex<Option<tokio::task::AbortHandle>>,
}

// ── DTOs ──

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ModelInfoDto {
    pub name: String,
    pub desc: String,
    pub input_price: f64,
    pub output_price: f64,
    pub is_current: bool,
}

#[derive(Serialize)]
pub struct ProviderDto {
    pub name: String,
    pub default_model: String,
    pub base_url: String,
}

#[derive(Serialize)]
pub struct ConfigDto {
    pub provider: String,
    pub api_key_masked: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub theme: String,
    pub proxy_url: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub web_search_enabled: bool,
    pub web_search_engine: String,
}

#[derive(Serialize)]
pub struct SessionInfoDto {
    pub id: String,
    pub name: String,
    pub msg_count: usize,
    pub updated_at: u64,
    pub age: String,
}

#[derive(Serialize)]
pub struct SessionData {
    pub id: String,
    pub name: String,
    pub messages: Vec<ChatMessageDto>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost: f64,
}

// ── 配置 ──

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<ConfigDto, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    Ok(ConfigDto {
        provider: cfg.provider.clone(),
        api_key_masked: util::mask_key(&cfg.api_key),
        base_url: cfg.base_url.clone(),
        model: cfg.model.clone(),
        max_tokens: cfg.max_tokens,
        theme: cfg.theme.clone(),
        proxy_url: cfg.proxy_url.clone(),
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        web_search_enabled: cfg.web_search.enabled,
        web_search_engine: cfg.web_search.engine.clone(),
    })
}

#[tauri::command]
pub fn update_config(
    state: tauri::State<'_, AppState>,
    api_key: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    max_tokens: Option<u32>,
    theme: Option<String>,
    proxy_url: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
) -> Result<(), String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    let mut provider_changed = false;
    if let Some(v) = api_key {
        cfg.api_key = v;
        state.client.set_api_key(cfg.api_key.clone());
    }
    if let Some(v) = provider {
        if let Some(preset) = Config::find_preset(&v) {
            cfg.provider = preset.name.to_string();
            cfg.base_url = preset.base_url.to_string();
            cfg.auth_type = preset.auth_type.to_string();
            cfg.api_format = preset.api_format.to_string();
            if preset.find_model(&cfg.model).is_none() {
                cfg.model = preset.default_model().name.to_string();
            }
            provider_changed = true;
        } else {
            cfg.provider = v;
        }
    }
    if let Some(v) = model {
        cfg.model = v;
        state.client.set_model(cfg.model.clone());
    }
    if let Some(v) = base_url {
        cfg.base_url = v;
    }
    if let Some(v) = max_tokens {
        cfg.max_tokens = v;
    }
    if let Some(v) = theme {
        cfg.theme = v;
    }
    if let Some(v) = proxy_url {
        cfg.proxy_url = if v.is_empty() { None } else { Some(v) };
    }
    cfg.temperature = temperature;
    cfg.top_p = top_p;
    // 同步更新运行中的客户端
    if provider_changed {
        if let Some(preset) = Config::find_preset(&cfg.provider) {
            state.client.update_for_provider(preset, &cfg.model, cfg.api_key.clone());
        }
    }
    cfg.save().map_err(|e| e.to_string())?;
    Ok(())
}

// ── Provider / Model ──

#[tauri::command]
pub fn get_providers() -> Vec<ProviderDto> {
    PROVIDERS
        .iter()
        .map(|p| ProviderDto {
            name: p.name.to_string(),
            default_model: p.default_model().name.to_string(),
            base_url: p.base_url.to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn get_models(state: tauri::State<'_, AppState>) -> Result<Vec<ModelInfoDto>, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    match cfg.current_preset() {
        Some(p) => Ok(p
            .models
            .iter()
            .map(|m| ModelInfoDto {
                name: m.name.to_string(),
                desc: m.desc.to_string(),
                input_price: m.input_price_per_mtok,
                output_price: m.output_price_per_mtok,
                is_current: m.name == cfg.model,
            })
            .collect()),
        None => Ok(vec![ModelInfoDto {
            name: cfg.model.clone(),
            desc: "自定义模型".into(),
            input_price: 2.0,
            output_price: 8.0,
            is_current: true,
        }]),
    }
}

#[tauri::command]
pub fn get_models_for_provider(state: tauri::State<'_, AppState>, provider: String) -> Vec<ModelInfoDto> {
    let current_model = state.config.lock().ok().map(|c| c.model.clone());
    match Config::find_preset(&provider) {
        Some(p) => p
            .models
            .iter()
            .map(|m| ModelInfoDto {
                name: m.name.to_string(),
                desc: m.desc.to_string(),
                input_price: m.input_price_per_mtok,
                output_price: m.output_price_per_mtok,
                is_current: current_model.as_deref() == Some(m.name),
            })
            .collect(),
        None => vec![ModelInfoDto {
            name: "custom".into(),
            desc: "自定义模型".into(),
            input_price: 2.0,
            output_price: 8.0,
            is_current: true,
        }],
    }
}

// ── 会话管理 ──

#[tauri::command]
pub fn list_sessions() -> Vec<SessionInfoDto> {
    Session::list()
        .into_iter()
        .map(|s| SessionInfoDto {
            id: s.id.clone(),
            name: s.name,
            msg_count: s.msg_count,
            updated_at: s.updated_at,
            age: util::format_age(s.updated_at),
        })
        .collect()
}

#[tauri::command]
pub fn load_session(session_id: String) -> Result<SessionData, String> {
    let s = Session::load(&session_id).map_err(|e| e.to_string())?;
    Ok(SessionData {
        id: s.id,
        name: s.name,
        messages: s
            .messages
            .iter()
            .map(|m| ChatMessageDto {
                role: m.role.clone(),
                content: m.content.as_str().to_string(),
            })
            .collect(),
        total_input_tokens: s.total_input_tokens,
        total_output_tokens: s.total_output_tokens,
        total_cost: s.total_cost,
    })
}

#[tauri::command]
pub fn create_session() -> SessionData {
    let s = Session::new(Session::auto_name());
    let _ = s.save();
    SessionData {
        id: s.id,
        name: s.name,
        messages: Vec::new(),
        total_input_tokens: 0,
        total_output_tokens: 0,
        total_cost: 0.0,
    }
}

#[tauri::command]
pub fn delete_session(session_id: String) -> Result<(), String> {
    let dir = dirs::config_dir()
        .ok_or("无法获取配置目录")?
        .join("mimo-opt")
        .join("sessions");
    let path = dir.join(format!("{}.json", session_id));
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_session_data(
    session_id: String,
    name: String,
    messages: Vec<ChatMessageDto>,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_cost: f64,
) -> Result<(), String> {
    let chat_messages: Arc<Vec<ChatMessage>> = Arc::new(
        messages
            .into_iter()
            .map(|m| ChatMessage {
                role: m.role,
                content: Content::text(m.content),
                cache_control: None,
            })
            .collect(),
    );
    let mut session = Session::load(&session_id).unwrap_or_else(|_| Session::new(name.clone()));
    session.name = name;
    session.messages = chat_messages;
    session.total_input_tokens = total_input_tokens;
    session.total_output_tokens = total_output_tokens;
    session.total_cost = total_cost;
    session.updated_at = util::now_secs();
    session.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_session(
    session_id: String,
    output_path: Option<String>,
) -> Result<String, String> {
    let session = Session::load(&session_id).map_err(|e| e.to_string())?;
    let path = output_path.unwrap_or_else(|| format!("{}.md", session.name));
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", session.name));
    md.push_str(&format!(
        "> Tokens: ↓{} ↑{} | Cost: ￥{:.4}\n\n---\n\n",
        session.total_input_tokens, session.total_output_tokens, session.total_cost,
    ));
    for msg in session.messages.iter() {
        let label = if msg.role == "user" { "User" } else { "Assistant" };
        md.push_str(&format!("## {}\n\n{}\n\n", label, msg.content.as_str()));
    }
    std::fs::write(&path, &md).map_err(|e| e.to_string())?;
    Ok(format!("已导出到 {} ({} 条消息)", path, session.messages.len()))
}

// ── 聊天（流式） ──

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    messages: Vec<ChatMessageDto>,
) -> Result<(), String> {
    let mut chat_msgs: Vec<ChatMessage> = messages
        .into_iter()
        .map(|m| ChatMessage {
            role: m.role,
            content: Content::text(m.content),
            cache_control: None,
        })
        .collect();

    prompt::apply_cache_breakpoints(&mut chat_msgs);

    let system_content =
        prompt::build_system_content_split(&state.system_stable, &state.system_dynamic);
    let client = Arc::clone(&state.client);

    let (tx, mut rx) = mpsc::channel::<StreamResult>(256);

    let app1 = app.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = client.send_message_stream(&system_content, &chat_msgs, tx).await {
            let _ = app1.emit_all(
                "stream_error",
                serde_json::json!({ "error": e.to_string() }),
            );
        }
    });
    // Store abort handle for cancellation
    if let Ok(mut h) = state.stream_handle.lock() {
        *h = Some(handle.abort_handle());
    }

    tokio::spawn(async move {
        while let Some(result) = rx.recv().await {
            match result {
                StreamResult::Token(text) => {
                    let _ = app.emit_all("stream_token", serde_json::json!({ "token": text }));
                }
                StreamResult::Done {
                    input_tokens,
                    output_tokens,
                    ..
                } => {
                    let _ = app.emit_all(
                        "stream_done",
                        serde_json::json!({
                            "input_tokens": input_tokens,
                            "output_tokens": output_tokens,
                        }),
                    );
                    break;
                }
                StreamResult::Error(err) => {
                    let _ =
                        app.emit_all("stream_error", serde_json::json!({ "error": err }));
                    break;
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn stop_generation(state: tauri::State<'_, AppState>) {
    if let Ok(mut h) = state.stream_handle.lock() {
        if let Some(handle) = h.take() {
            handle.abort();
        }
    }
}

// ── 联网搜索 ──

#[tauri::command]
pub async fn web_search(state: tauri::State<'_, AppState>, query: String) -> Result<String, String> {
    let web_cfg = {
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        if !cfg.web_search.enabled {
            return Err("联网搜索未启用".into());
        }
        cfg.web_search.clone()
    };
    search::execute_search(&query, &web_cfg)
        .await
        .map_err(|e| e.to_string())
}

// ── 技能 ──

#[tauri::command]
pub fn get_skills(state: tauri::State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    Ok(cfg
        .skills
        .iter()
        .map(|(k, v)| (k.clone(), v.desc().unwrap_or(v.cmd()).to_string()))
        .collect())
}

#[tauri::command]
pub fn add_skill(
    state: tauri::State<'_, AppState>,
    name: String,
    cmd: String,
    desc: Option<String>,
) -> Result<(), String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.skills.insert(
        name,
        SkillEntry::Detailed {
            cmd,
            desc,
            analyze: true,
        },
    );
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_skill(state: tauri::State<'_, AppState>, name: String) -> Result<(), String> {
    let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
    cfg.skills.remove(&name);
    cfg.save().map_err(|e| e.to_string())
}

// ── 文件操作 ──

#[tauri::command]
pub fn read_file(path: String, start_line: Option<usize>, end_line: Option<usize>) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let full_path = mimo_opt::file_ops::validate_path(&path, &cwd)?;
    let range = match (start_line, end_line) {
        (Some(s), Some(e)) => Some((s, e)),
        (Some(s), None) => Some((s, s)),
        _ => None,
    };
    mimo_opt::file_ops::read_file(&full_path, range)
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let full_path = mimo_opt::file_ops::validate_path(&path, &cwd)?;
    mimo_opt::file_ops::write_file(&full_path, &content)
}

#[tauri::command]
pub fn edit_file(path: String, old_text: String, new_text: String) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let full_path = mimo_opt::file_ops::validate_path(&path, &cwd)?;
    mimo_opt::file_ops::apply_edit(&full_path, &old_text, &new_text)
}

// ── 工具 ──

#[tauri::command]
pub fn estimate_tokens(text: String) -> usize {
    util::estimate_tokens(&text)
}

#[tauri::command]
pub fn get_model_prices(state: tauri::State<'_, AppState>) -> (f64, f64) {
    state.client.get_model_prices()
}

#[tauri::command]
pub fn get_themes() -> Vec<String> {
    mimo_opt::ui::theme_names()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[tauri::command]
pub fn get_project_tree(state: tauri::State<'_, AppState>) -> String {
    state.project_tree.clone()
}
