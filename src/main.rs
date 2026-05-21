mod api;
mod app;
mod commands;
mod config;
mod file_ops;
mod prompt;
mod scanner;
mod session;
mod ui;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let mut config = config::Config::load()?;
    config.apply_defaults();
    log::info!("配置加载完成 provider={} model={}", config.provider, config.model);

    if config.api_key.is_empty() {
        eprintln!("MiMo-OPT 首次运行，请配置 API");
        eprintln!();
        eprintln!("配置文件: {}", config::Config::config_path()?.display());
        eprintln!();
        eprintln!("方案一：MiMo Token Plan（Anthropic 格式）");
        eprintln!("  \"provider\": \"mimo\"");
        eprintln!("  \"api_key\": \"tp-你的密钥\"");
        eprintln!("  \"auth_type\": \"anthropic\"");
        eprintln!("  \"api_format\": \"anthropic\"");
        eprintln!("  \"base_url\": \"https://token-plan-sgp.xiaomimimo.com/anthropic\"");
        eprintln!("  获取密钥: https://platform.xiaomimimo.com/#/console/subscription");
        eprintln!();
        eprintln!("方案二：DeepSeek API（OpenAI 兼容格式）");
        eprintln!("  \"provider\": \"deepseek\"");
        eprintln!("  \"api_key\": \"sk-你的密钥\"");
        eprintln!("  \"auth_type\": \"bearer\"");
        eprintln!("  \"api_format\": \"openai\"");
        eprintln!("  \"base_url\": \"https://api.deepseek.com\"");
        eprintln!("  \"model\": \"deepseek-chat\"    (或 deepseek-reasoner)");
        eprintln!("  获取密钥: https://platform.deepseek.com/api_keys");
        eprintln!();
        eprintln!("方案三：OpenAI API");
        eprintln!("  \"provider\": \"openai\"");
        eprintln!("  \"api_key\": \"sk-你的密钥\"");
        eprintln!("  \"auth_type\": \"bearer\"");
        eprintln!("  \"api_format\": \"openai\"");
        eprintln!("  \"base_url\": \"https://api.openai.com\"");
        eprintln!("  \"model\": \"gpt-4o-mini\"");
        eprintln!();
        eprintln!("方案四：MiMo API（OpenAI 兼容格式）");
        eprintln!("  \"provider\": \"custom\"");
        eprintln!("  \"api_key\": \"你的密钥\"");
        eprintln!("  \"auth_type\": \"bearer\"");
        eprintln!("  \"api_format\": \"openai\"");
        eprintln!("  \"base_url\": \"https://api.xiaomimimo.com/v1\"");
        eprintln!("  获取密钥: https://platform.xiaomimimo.com");
        eprintln!();
        eprintln!("方案五：其他 OpenAI 兼容 API（如 GLM、通义千问等）");
        eprintln!("  \"provider\": \"custom\"");
        eprintln!("  \"api_key\": \"你的密钥\"");
        eprintln!("  \"auth_type\": \"bearer\"");
        eprintln!("  \"api_format\": \"openai\"");
        eprintln!("  \"base_url\": \"https://api.example.com\"");
        eprintln!("  \"model\": \"your-model-name\"");
        std::process::exit(1);
    }

    app::run(config).await
}
