use mimo_opt::{app, config, pipe};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args: Vec<String> = std::env::args().collect();
    let (pipe_mode, prompt_arg) = if args.len() >= 2 && (args[1] == "-p" || args[1] == "--prompt")
    {
        (true, args.get(2).map(|s| s.as_str()))
    } else {
        (false, None)
    };

    let mut cfg = config::Config::load()?;
    log::info!("配置加载完成 provider={} model={}", cfg.provider, cfg.model);

    if cfg.api_key.is_empty() {
        if pipe_mode {
            eprintln!("mimo-opt: 未配置 API Key，请先运行交互模式配置");
            std::process::exit(1);
        }
        print_first_run_guide();
        std::process::exit(1);
    }

    cfg.apply_defaults();

    if pipe_mode {
        pipe::run_pipe(&cfg, prompt_arg).await
    } else {
        app::run(cfg).await
    }
}

fn print_first_run_guide() {
    eprintln!("MiMo-OPT 首次运行，请配置 API");
    eprintln!();
    eprintln!(
        "配置文件: {}",
        config::Config::config_path().unwrap_or_default().display()
    );
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
    eprintln!("  \"model\": \"deepseek-chat\"");
    eprintln!("  获取密钥: https://platform.deepseek.com/api_keys");
    eprintln!();
    eprintln!("方案三：OpenAI API");
    eprintln!("  \"provider\": \"openai\"");
    eprintln!("  \"api_key\": \"sk-你的密钥\"");
    eprintln!();
    eprintln!("方案四：自定义 OpenAI 兼容 API");
    eprintln!("  \"provider\": \"custom\"");
    eprintln!("  \"api_key\": \"你的密钥\"");
}
