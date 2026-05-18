use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::api::{CacheControl, ChatMessage, MiMoClient, StreamResult, SystemContent};
use crate::config::Config;
use crate::ui;

pub struct AppState {
    pub config: Config,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub cursor_pos: usize,
    pub generating: bool,
    pub stream_buffer: String,
    pub spinner_tick: usize,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cost: f64,
    pub error_message: Option<String>,
    pub should_quit: bool,
    pub api_ok: Option<bool>, // None=未检测, Some(true)=正常, Some(false)=异常
    pub api_error_detail: Option<String>,
    // 缓存
    pub system_prompt_text: String,
    pub system_prompt_cached_date: String,
    pub project_tree: String,
    pub skills: HashMap<String, String>,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // scopeguard 确保终端清理
    let _guard = scopeguard::guard((), |_| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    });

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let client = Arc::new(MiMoClient::new(
        config.base_url.clone(),
        config.api_key.clone(),
        config.model.clone(),
        config.auth_type.clone(),
    ));

    let project_tree = scan_project_tree();
    let today = chrono_date();
    let skills = config.skills.clone();

    let mut state = AppState {
        config,
        messages: Vec::new(),
        input: String::new(),
        cursor_pos: 0,
        generating: false,
        stream_buffer: String::new(),
        spinner_tick: 0,
        total_input_tokens: 0,
        total_output_tokens: 0,
        total_cache_creation_tokens: 0,
        total_cache_read_tokens: 0,
        total_cost: 0.0,
        error_message: None,
        should_quit: false,
        api_ok: None,
        api_error_detail: None,
        system_prompt_text: String::new(),
        system_prompt_cached_date: String::new(),
        project_tree,
        skills,
    };

    // 构建初始 system prompt
    state.system_prompt_text = build_system_prompt_text(&state.project_tree, &today);
    state.system_prompt_cached_date = today;

    // token 流 channel
    let (token_tx, mut token_rx) = mpsc::unbounded_channel::<StreamResult>();

    // 启动时探测 API（发一个极短请求）
    match client.check_api().await {
        Ok(()) => {
            state.api_ok = Some(true);
        }
        Err(e) => {
            state.api_ok = Some(false);
            state.api_error_detail = Some(e.to_string());
        }
    }

    let result = run_loop(&mut terminal, &mut state, &client, &token_tx, &mut token_rx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::UnboundedSender<StreamResult>,
    token_rx: &mut mpsc::UnboundedReceiver<StreamResult>,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, state))?;

        // 处理流式结果
        while let Ok(result) = token_rx.try_recv() {
            match result {
                StreamResult::Token(text) => {
                    state.stream_buffer.push_str(&text);
                }
                StreamResult::Done {
                    input_tokens,
                    output_tokens,
                    cache_creation_tokens,
                    cache_read_tokens,
                } => {
                    state.generating = false;
                    if !state.stream_buffer.is_empty() {
                        state.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: state.stream_buffer.clone(),
                            cache_control: None,
                        });
                    }
                    state.total_input_tokens += input_tokens;
                    state.total_output_tokens += output_tokens;
                    state.total_cache_creation_tokens += cache_creation_tokens;
                    state.total_cache_read_tokens += cache_read_tokens;
                    state.total_cost =
                        calculate_cost(state.total_input_tokens, state.total_output_tokens);
                    state.stream_buffer.clear();
                    // 生成完成，API 正常
                    state.api_ok = Some(true);
                    state.api_error_detail = None;
                }
                StreamResult::Error(err) => {
                    state.generating = false;
                    if !state.stream_buffer.is_empty() {
                        state.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: state.stream_buffer.clone(),
                            cache_control: None,
                        });
                        state.stream_buffer.clear();
                    }
                    state.error_message = Some(err.clone());
                    state.api_ok = Some(false);
                    state.api_error_detail = Some(err);
                }
            }
        }

        // 处理键盘事件（非阻塞，50ms 超时保证 spinner 刷新）
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    // Ctrl+Q 退出
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.should_quit = true;
                    }
                    // Ctrl+C: 中断生成（不退出）
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if state.generating {
                            state.generating = false;
                            if !state.stream_buffer.is_empty() {
                                state.messages.push(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: state.stream_buffer.clone(),
                                    cache_control: None,
                                });
                                state.stream_buffer.clear();
                            }
                        }
                    }
                    // Esc: 中断生成
                    KeyCode::Esc if state.generating => {
                        state.generating = false;
                        if !state.stream_buffer.is_empty() {
                            state.messages.push(ChatMessage {
                                role: "assistant".to_string(),
                                content: state.stream_buffer.clone(),
                                cache_control: None,
                            });
                            state.stream_buffer.clear();
                        }
                    }
                    // Ctrl+Enter: 发送消息或执行技能
                    KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !state.input.is_empty() && !state.generating {
                            let user_msg = state.input.clone();
                            state.input.clear();
                            state.cursor_pos = 0;
                            state.error_message = None;

                            if user_msg.starts_with('/') {
                                // 技能执行 — 提取 owned strings 避免借用问题
                                let after_slash = user_msg[1..].to_string();
                                let mut parts_iter = after_slash.splitn(2, ' ');
                                let skill_name = parts_iter.next().unwrap_or("").to_string();
                                let cmd = state.skills.get(&skill_name).cloned();

                                if let Some(cmd) = cmd {
                                    state.generating = true;
                                    state.stream_buffer.clear();
                                    state.spinner_tick = 0;

                                    let client = Arc::clone(client);
                                    let tx = token_tx.clone();
                                    let messages_clone = state.messages.clone();
                                    let system_text = state.system_prompt_text.clone();

                                    tokio::spawn(async move {
                                        let shell = if cfg!(target_os = "windows") {
                                            "cmd"
                                        } else {
                                            "sh"
                                        };
                                        let shell_flag = if cfg!(target_os = "windows") {
                                            "/C"
                                        } else {
                                            "-c"
                                        };

                                        let output =
                                            tokio::process::Command::new(shell)
                                                .arg(shell_flag)
                                                .arg(&cmd)
                                                .output()
                                                .await;

                                        match output {
                                            Ok(out) => {
                                                let stdout =
                                                    String::from_utf8_lossy(&out.stdout);
                                                let stderr =
                                                    String::from_utf8_lossy(&out.stderr);
                                                let mut result = stdout.to_string();
                                                if !stderr.is_empty() {
                                                    result
                                                        .push_str(&format!("\n[stderr]\n{}", stderr));
                                                }
                                                if !out.status.success() {
                                                    result.push_str(&format!(
                                                        "\n[exit code: {}]",
                                                        out.status
                                                            .code()
                                                            .map_or(-1, |c| c)
                                                    ));
                                                }

                                                let _ = tx.send(StreamResult::Token(format!(
                                                    "/{} output:\n{}",
                                                    skill_name, result
                                                )));

                                                // 用技能输出作为用户消息，发送给 MiMo 分析
                                                let mut msgs = messages_clone;
                                                msgs.push(ChatMessage {
                                                    role: "user".to_string(),
                                                    content: format!(
                                                        "请分析以下 /{} 命令的输出:\n{}",
                                                        skill_name, result
                                                    ),
                                                    cache_control: None,
                                                });
                                                apply_cache_breakpoints(&mut msgs);
                                                let system_content =
                                                    build_system_content(&system_text);

                                                let result = client
                                                    .send_message_stream(
                                                        &system_content,
                                                        &msgs,
                                                        tx.clone(),
                                                    )
                                                    .await;
                                                if let Err(e) = result {
                                                    let _ =
                                                        tx.send(StreamResult::Error(e.to_string()));
                                                }
                                            }
                                            Err(e) => {
                                                let _ = tx.send(StreamResult::Error(format!(
                                                    "/{} 执行失败: {}",
                                                    skill_name, e
                                                )));
                                            }
                                        }
                                    });
                                } else {
                                    state.error_message =
                                        Some(format!("未知技能: /{}", skill_name));
                                }
                            } else {
                                // 普通消息发送
                                state.messages.push(ChatMessage {
                                    role: "user".to_string(),
                                    content: user_msg,
                                    cache_control: None,
                                });

                                state.generating = true;
                                state.stream_buffer.clear();
                                state.spinner_tick = 0;

                                // 检查日期是否变化，需要重建 system prompt
                                let today = chrono_date();
                                let system_text = if today != state.system_prompt_cached_date {
                                    state.system_prompt_text =
                                        build_system_prompt_text(&state.project_tree, &today);
                                    state.system_prompt_cached_date = today.clone();
                                    state.system_prompt_text.clone()
                                } else {
                                    state.system_prompt_text.clone()
                                };

                                // 构建带缓存断点的消息列表
                                let mut messages = state.messages.clone();
                                apply_cache_breakpoints(&mut messages);

                                // 构建 system prompt
                                let system_content = build_system_content(&system_text);

                                let client = Arc::clone(client);
                                let tx = token_tx.clone();

                                tokio::spawn(async move {
                                    let result = client
                                        .send_message_stream(&system_content, &messages, tx.clone())
                                        .await;
                                    if let Err(e) = result {
                                        let _ = tx.send(StreamResult::Error(e.to_string()));
                                    }
                                });
                            }
                        }
                    }
                    // 输入字符
                    KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !state.generating {
                            state.input.insert(state.cursor_pos, ch);
                            state.cursor_pos += ch.len_utf8();
                        }
                    }
                    // 退格
                    KeyCode::Backspace if !state.generating => {
                        if state.cursor_pos > 0 {
                            // 找到前一个字符的边界
                            let prev = state.input[..state.cursor_pos]
                                .chars()
                                .next_back()
                                .map(|c| state.cursor_pos - c.len_utf8())
                                .unwrap_or(0);
                            state.input.remove(prev);
                            state.cursor_pos = prev;
                        }
                    }
                    // Delete
                    KeyCode::Delete if !state.generating => {
                        if state.cursor_pos < state.input.len() {
                            state.input.remove(state.cursor_pos);
                        }
                    }
                    // Home / Ctrl+A
                    KeyCode::Home if !state.generating => {
                        state.cursor_pos = 0;
                    }
                    KeyCode::Char('a')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !state.generating =>
                    {
                        state.cursor_pos = 0;
                    }
                    // End / Ctrl+E
                    KeyCode::End if !state.generating => {
                        state.cursor_pos = state.input.len();
                    }
                    KeyCode::Char('e')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !state.generating =>
                    {
                        state.cursor_pos = state.input.len();
                    }
                    // 左箭头
                    KeyCode::Left if !state.generating => {
                        if state.cursor_pos > 0 {
                            let prev = state.input[..state.cursor_pos]
                                .chars()
                                .next_back()
                                .map(|c| state.cursor_pos - c.len_utf8())
                                .unwrap_or(0);
                            state.cursor_pos = prev;
                        }
                    }
                    // 右箭头
                    KeyCode::Right if !state.generating => {
                        if state.cursor_pos < state.input.len() {
                            let next = state.input[state.cursor_pos..]
                                .chars()
                                .next()
                                .map(|c| state.cursor_pos + c.len_utf8())
                                .unwrap_or(state.input.len());
                            state.cursor_pos = next;
                        }
                    }
                    _ => {}
                }
            }
        }

        // spinner
        if state.generating {
            state.spinner_tick = state.spinner_tick.wrapping_add(1);
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

fn calculate_cost(input_tokens: u64, output_tokens: u64) -> f64 {
    // MiMo Token Plan 粗略费率（具体以官方为准）
    let input_cost = input_tokens as f64 / 1_000_000.0 * 2.0; // ¥2/百万 input tokens
    let output_cost = output_tokens as f64 / 1_000_000.0 * 8.0; // ¥8/百万 output tokens
    input_cost + output_cost
}

// ── 缓存优化 ──

fn chrono_date() -> String {
    // 简单日期格式，不引入 chrono 依赖
    // 使用系统时间计算年月日
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();
    // 粗略计算（不考虑闰秒，足够用于日期显示）
    let days_since_epoch = total_secs / 86400;
    let date = chrono_from_days(days_since_epoch);
    format!("{:04}/{:02}/{:02}", date.0, date.1, date.2)
}

fn chrono_from_days(days: u64) -> (u64, u64, u64) {
    // 从 1970-01-01 开始的天数计算年月日
    let mut y = 1970u64;
    let mut d = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let months = [31, if is_leap(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0u64;
    for (i, &days_in_month) in months.iter().enumerate() {
        if d < days_in_month {
            m = i as u64 + 1;
            d += 1;
            break;
        }
        d -= days_in_month;
    }
    (y, m, d)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn build_system_prompt_text(project_tree: &str, date: &str) -> String {
    let mut text = String::with_capacity(512);
    text.push_str("You are MiMo, a helpful AI assistant created by Xiaomi's LLM-Core team. ");
    text.push_str("You can help with coding, analysis, writing, math, and general Q&A.\n\n");
    text.push_str("## Output Format\n");
    text.push_str("- Use markdown for formatting\n");
    text.push_str("- Use code blocks with language tags for code\n");
    text.push_str("- Be concise and direct\n\n");
    text.push_str("## Current Working Directory\n");
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    text.push_str(&cwd);
    text.push('\n');
    if !project_tree.is_empty() {
        text.push_str("\n## Project Files\n");
        text.push_str(project_tree);
    }
    text.push_str("\n\n## Current Date\n");
    text.push_str(date);
    text
}

fn build_system_content(system_text: &str) -> Vec<SystemContent> {
    vec![SystemContent {
        content_type: "text".to_string(),
        text: system_text.to_string(),
        cache_control: CacheControl {
            cache_type: "ephemeral".to_string(),
        },
    }]
}

/// 对消息列表应用缓存断点策略
/// - system prompt 始终在 build_system_content 中标记缓存
/// - 消息 >= 4 条时，第 4 条消息（index 3）标记为缓存断点
fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    if messages.len() >= 4 {
        messages[3].cache_control = Some(CacheControl {
            cache_type: "ephemeral".to_string(),
        });
    }
}

fn scan_project_tree() -> String {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    // 检查是否有 .git 目录（判断是否为 git 仓库）
    let is_git_repo = cwd.join(".git").exists();

    // 默认排除的目录
    let default_excludes: &[&str] = &[
        ".git", "node_modules", "target", "__pycache__", ".venv", "venv", "vendor", ".idea",
        ".vscode", ".DS_Store", "dist", "build",
    ];

    let mut entries: Vec<String> = Vec::new();
    let max_files = 100;

    scan_dir_recursive(&cwd, &cwd, default_excludes, is_git_repo, &mut entries, max_files, 0);

    if entries.is_empty() {
        return String::new();
    }

    entries.sort();
    entries.join("\n")
}

fn scan_dir_recursive(
    dir: &std::path::Path,
    base: &std::path::Path,
    excludes: &[&str],
    is_git_repo: bool,
    entries: &mut Vec<String>,
    max_files: usize,
    depth: usize,
) {
    if entries.len() >= max_files || depth > 5 {
        return;
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut dirs_to_scan: Vec<std::path::PathBuf> = Vec::new();

    for entry in read_dir.flatten() {
        if entries.len() >= max_files {
            break;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // 跳过排除的目录/文件
        if excludes.iter().any(|&e| e == name_str.as_ref()) {
            continue;
        }

        // 跳过隐藏文件/目录（. 开头）
        if name_str.starts_with('.') {
            continue;
        }

        let path = entry.path();

        if path.is_dir() {
            dirs_to_scan.push(path);
        } else if path.is_file() {
            // 添加相对路径
            let rel = path.strip_prefix(base).unwrap_or(&path);
            entries.push(rel.display().to_string());
        }
    }

    // 递归扫描子目录
    for subdir in dirs_to_scan {
        if entries.len() >= max_files {
            break;
        }
        scan_dir_recursive(&subdir, base, excludes, is_git_repo, entries, max_files, depth + 1);
    }
}
