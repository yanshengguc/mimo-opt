mod app;
mod config;
mod api;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::load()?;

    if config.api_key.is_empty() {
        eprintln!("MiMo-OPT 首次运行，请配置 API");
        eprintln!();
        eprintln!("配置文件: {}", config::Config::config_path()?.display());
        eprintln!();
        eprintln!("方案一：MiMo Token Plan（Anthropic 格式）");
        eprintln!("  \"api_key\": \"tp-你的密钥\"");
        eprintln!("  \"auth_type\": \"anthropic\"");
        eprintln!("  \"base_url\": \"https://token-plan-sgp.xiaomimimo.com/anthropic\"");
        eprintln!("  获取密钥: https://platform.xiaomimimo.com/#/console/subscription");
        eprintln!();
        eprintln!("方案二：MiMo API（OpenAI Bearer 格式）");
        eprintln!("  \"api_key\": \"你的密钥\"");
        eprintln!("  \"auth_type\": \"bearer\"");
        eprintln!("  \"base_url\": \"https://api.xiaomimimo.com/v1\"");
        eprintln!("  获取密钥: https://platform.xiaomimimo.com");
        std::process::exit(1);
    }

    app::run(config).await
}
