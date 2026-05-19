mod app;
mod config;
mod api;
mod file_ops;
mod session;
mod ui;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = config::Config::load()?;
    config.apply_defaults();

    if config.api_key.is_empty() {
        eprintln!("MiMo-OPT 首次运行，请配置 API");
        eprintln!();
        eprintln!("配置文件: {}", config::Config::config_path()?.display());
        eprintln!();
        eprintln!("方案一：MiMo Token Plan（Anthropic 格式）");
        eprintln!("  \"api_key\": \"tp-你的密钥\"");
        eprintln!("  \"auth_type\": \"anthropic\"");
        eprintln!("  \"api_format\": \"anthropic\"");
        eprintln!("  \"base_url\": \"https://token-plan-sgp.xiaomimimo.com/anthropic\"");
        eprintln!("  获取密钥: https://platform.xiaomimimo.com/#/console/subscription");
        eprintln!();
        eprintln!("方案二：MiMo API（OpenAI 兼容格式）");
        eprintln!("  \"api_key\": \"你的密钥\"");
        eprintln!("  \"auth_type\": \"bearer\"");
        eprintln!("  \"api_format\": \"openai\"");
        eprintln!("  \"base_url\": \"https://api.xiaomimimo.com/v1\"");
        eprintln!("  获取密钥: https://platform.xiaomimimo.com");
        eprintln!();
        eprintln!("方案三：其他 OpenAI 兼容 API（如 DeepSeek、GLM 等）");
        eprintln!("  \"api_key\": \"你的密钥\"");
        eprintln!("  \"auth_type\": \"bearer\"");
        eprintln!("  \"api_format\": \"openai\"");
        eprintln!("  \"base_url\": \"https://api.example.com\"");
        eprintln!("  \"model\": \"deepseek-chat\"");
        std::process::exit(1);
    }

    app::run(config).await
}
