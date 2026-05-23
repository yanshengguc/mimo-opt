use std::sync::Arc;

use mimo_opt::{config, scanner, prompt};
mod tauri_cmds;
use tauri_cmds::AppState;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut cfg = config::Config::load().expect("配置加载失败");
    cfg.apply_defaults();

    if cfg.api_key.is_empty() {
        eprintln!("MiMo-OPT 桌面端：未配置 API Key");
        eprintln!("请编辑配置文件后重新启动");
        eprintln!(
            "配置文件: {}",
            config::Config::config_path().unwrap_or_default().display()
        );
        std::process::exit(1);
    }

    let client = Arc::new(mimo_opt::api::MiMoClient::new(
        cfg.base_url.clone(),
        cfg.api_key.clone(),
        cfg.model.clone(),
        cfg.auth_type.clone(),
        cfg.api_format.clone(),
        cfg.max_tokens,
        cfg.proxy_url.clone(),
        cfg.temperature,
        cfg.top_p,
    ));

    let project_tree = scanner::scan_project_tree();
    let system_stable = prompt::build_system_stable(&cfg.provider);
    let system_dynamic = prompt::build_system_dynamic(&project_tree);

    let app_state = AppState {
        config: std::sync::Mutex::new(cfg),
        client,
        project_tree,
        system_stable,
        system_dynamic,
        stream_handle: std::sync::Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            tauri_cmds::get_config,
            tauri_cmds::update_config,
            tauri_cmds::get_providers,
            tauri_cmds::get_models,
            tauri_cmds::get_models_for_provider,
            tauri_cmds::list_sessions,
            tauri_cmds::load_session,
            tauri_cmds::create_session,
            tauri_cmds::delete_session,
            tauri_cmds::save_session_data,
            tauri_cmds::send_message,
            tauri_cmds::stop_generation,
            tauri_cmds::web_search,
            tauri_cmds::get_skills,
            tauri_cmds::add_skill,
            tauri_cmds::remove_skill,
            tauri_cmds::read_file,
            tauri_cmds::write_file,
            tauri_cmds::edit_file,
            tauri_cmds::estimate_tokens,
            tauri_cmds::get_model_prices,
            tauri_cmds::get_themes,
            tauri_cmds::get_project_tree,
            tauri_cmds::export_session,
        ])
        .run(tauri::generate_context!())
        .expect("启动桌面端失败");
}
