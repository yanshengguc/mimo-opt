use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::api::{CacheControl, ChatMessage, Content, MiMoClient, StreamResult, SystemContent};
use crate::config::Config;
use crate::file_ops;
use crate::session::{Session, SessionInfo};
use crate::ui;
use crate::util::{now_secs, get_cwd};

#[derive(Clone)]
pub enum ConfirmAction {
    WriteFile { path: String, code: String, lang: String },
    EditFile { path: String, old: String, new: String },
    ClearChat,
    Quit,
}

#[derive(Clone)]
pub struct ConfirmState {
    pub action: ConfirmAction,
    pub detail: String,
}

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
    pub project_tree: String,
    pub skills: HashMap<String, String>,
    // 输入历史
    pub input_history: Vec<String>,
    pub history_idx: Option<usize>,
    pub input_draft: String,
    // 技能提示
    pub hint_lines: Vec<String>,
    // 聊天区滚动
    pub chat_scroll: usize,
    // 搜索
    pub search_active: bool,
    pub search_query: String,
    pub search_matches: Vec<usize>, // 消息 index
    pub search_match_idx: usize,
    // 确认对话框
    pub pending_confirm: Option<ConfirmState>,
    // 代码块（(lang, content) 可见的代码块列表）
    pub code_blocks: Vec<(String, String)>,
    pub copy_status: Option<String>,
    // 会话
    pub session: Session,
    pub session_list: Vec<SessionInfo>,
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
        config.api_format.clone(),
        config.max_tokens,
    ));

    let project_tree = scan_project_tree();
    let skills = config.skills.clone();

    // 加载最近会话或创建新会话
    let session_list = Session::list();
    let session = if let Some(info) = session_list.first() {
        match Session::load(&info.id) {
            Ok(s) => s,
            Err(_) => Session::new(Session::auto_name()),
        }
    } else {
        Session::new(Session::auto_name())
    };
    let messages = session.messages.clone();

    let mut state = AppState {
        config,
        messages,
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
        project_tree,
        skills,
        input_history: Vec::new(),
        history_idx: None,
        input_draft: String::new(),
        hint_lines: Vec::new(),
        chat_scroll: 0,
        search_active: false,
        search_query: String::new(),
        search_matches: Vec::new(),
        search_match_idx: 0,
        pending_confirm: None,
        code_blocks: Vec::new(),
        copy_status: None,
        session,
        session_list,
    };

    // 构建初始 system prompt（静态，不含日期）
    state.system_prompt_text = build_system_prompt_text(&state.project_tree);

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
                            content: Content::text(state.stream_buffer.clone()),
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
                    state.chat_scroll = 0; // 新消息到达，回到底部
                    state.api_ok = Some(true);
                    state.api_error_detail = None;

                    // 自动保存会话（每 5 条消息）
                    if state.messages.len() % 5 == 0 {
                        save_session(state);
                    }
                }
                StreamResult::Error(err) => {
                    state.generating = false;
                    if !state.stream_buffer.is_empty() {
                        state.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: Content::text(state.stream_buffer.clone()),
                            cache_control: None,
                        });
                        state.stream_buffer.clear();
                    }
                    state.error_message = Some(err.clone());
                    state.api_ok = Some(false);
                    state.api_error_detail = Some(err);
                    state.chat_scroll = 0;
                }
            }
        }

        // 处理键盘事件（非阻塞，50ms 超时保证 spinner 刷新）
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // 清除复制状态（下次按键消失）
                if state.copy_status.is_some() {
                    state.copy_status = None;
                }

                match key.code {
                    // 确认对话框 — y 确认 / n/Esc 取消 / d 显示详情
                    KeyCode::Char('y') if state.pending_confirm.is_some() && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        execute_confirm(state, client, token_tx);
                    }
                    KeyCode::Char('n') if state.pending_confirm.is_some() && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.pending_confirm = None;
                        push_cancelled_message(state);
                    }
                    KeyCode::Esc if state.pending_confirm.is_some() => {
                        state.pending_confirm = None;
                        push_cancelled_message(state);
                    }
                    KeyCode::Char('d') if state.pending_confirm.is_some() && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        show_confirm_detail(state);
                    }
                    // Ctrl+N: 新建会话
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) && state.pending_confirm.is_none() => {
                        save_session(state);
                        // 创建新会话
                        state.session = Session::new(Session::auto_name());
                        state.messages.clear();
                        state.stream_buffer.clear();
                        state.chat_scroll = 0;
                        state.total_input_tokens = 0;
                        state.total_output_tokens = 0;
                        state.total_cache_creation_tokens = 0;
                        state.total_cache_read_tokens = 0;
                        state.total_cost = 0.0;
                        state.session_list = Session::list();
                    }
                    // F2: 切换会话
                    KeyCode::F(2) if !state.generating && state.pending_confirm.is_none() => {
                        save_session(state);
                        // 获取会话列表并切换到下一个
                        state.session_list = Session::list();
                        if state.session_list.len() > 1 {
                            // 找到当前会话的位置，切换到下一个
                            let current_idx = state.session_list
                                .iter()
                                .position(|s| s.id == state.session.id)
                                .unwrap_or(0);
                            let next_idx = (current_idx + 1) % state.session_list.len();
                            let next_id = state.session_list[next_idx].id.clone();
                            if let Ok(s) = Session::load(&next_id) {
                                state.messages = s.messages.clone();
                                state.session = s;
                                state.stream_buffer.clear();
                                state.chat_scroll = 0;
                                state.total_input_tokens = 0;
                                state.total_output_tokens = 0;
                                state.total_cache_creation_tokens = 0;
                                state.total_cache_read_tokens = 0;
                                state.total_cost = 0.0;
                            }
                        }
                    }
                    // Ctrl+Q 退出（有消息时确认）
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !state.messages.is_empty() && state.pending_confirm.is_none() {
                            state.pending_confirm = Some(ConfirmState {
                                action: ConfirmAction::Quit,
                                detail: format!("退出将丢失 {} 条对话", state.messages.len()),
                            });
                        } else {
                            save_session(state);
                            state.should_quit = true;
                        }
                    }
                    // Ctrl+C: 中断生成（不退出）
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) && state.pending_confirm.is_none() => {
                        if state.generating {
                            state.generating = false;
                            if !state.stream_buffer.is_empty() {
                                state.messages.push(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: Content::text(state.stream_buffer.clone()),
                                    cache_control: None,
                                });
                                state.stream_buffer.clear();
                            }
                        }
                    }
                    // Ctrl+F: 搜索模式
                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) && state.pending_confirm.is_none() => {
                        if !state.generating && !state.search_active {
                            state.search_active = true;
                            state.search_query.clear();
                            state.search_matches.clear();
                            state.search_match_idx = 0;
                        }
                    }
                    // Ctrl+Y: 复制最后一个代码块到剪贴板
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) && state.pending_confirm.is_none() => {
                        collect_code_blocks(state);
                        if let Some((lang, content)) = state.code_blocks.last() {
                            match cli_clipboard::set_contents(content.clone()) {
                                Ok(()) => {
                                    state.copy_status = Some(format!("已复制 {} 代码块 ({} 行)", lang, content.lines().count()));
                                }
                                Err(e) => {
                                    state.copy_status = Some(format!("复制失败: {}", e));
                                }
                            }
                        } else {
                            state.copy_status = Some("对话中未找到代码块".into());
                        }
                    }
                    // Esc: 退出搜索 或 中断生成
                    KeyCode::Esc if state.search_active => {
                        state.search_active = false;
                        state.search_query.clear();
                        state.search_matches.clear();
                    }
                    KeyCode::Esc if state.generating => {
                        state.generating = false;
                        if !state.stream_buffer.is_empty() {
                            state.messages.push(ChatMessage {
                                role: "assistant".to_string(),
                                content: Content::text(state.stream_buffer.clone()),
                                cache_control: None,
                            });
                            state.stream_buffer.clear();
                        }
                    }
                    // 搜索模式 — 输入/删除/跳转
                    KeyCode::Char(ch) if state.search_active && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.search_query.push(ch);
                        do_search(state);
                    }
                    KeyCode::Backspace if state.search_active => {
                        state.search_query.pop();
                        do_search(state);
                    }
                    KeyCode::Enter if state.search_active => {
                        if !state.search_matches.is_empty() {
                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                if state.search_match_idx > 0 {
                                    state.search_match_idx -= 1;
                                } else {
                                    state.search_match_idx = state.search_matches.len() - 1;
                                }
                            } else {
                                state.search_match_idx = (state.search_match_idx + 1) % state.search_matches.len();
                            }
                        }
                    }
                    // PageUp/PageDown: 聊天区滚动
                    KeyCode::PageUp if !state.generating && !state.search_active => {
                        state.chat_scroll = state.chat_scroll.saturating_add(10);
                    }
                    KeyCode::PageDown if !state.generating && !state.search_active => {
                        state.chat_scroll = state.chat_scroll.saturating_sub(10);
                    }
                    KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.chat_scroll = 0;
                    }
                    // Ctrl+Enter: 发送消息或执行技能
                    KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) && state.pending_confirm.is_none() => {
                        if !state.input.is_empty() && !state.generating {
                            let user_msg = state.input.clone();
                            state.input.clear();
                            state.cursor_pos = 0;
                            state.error_message = None;
                            // 记录到输入历史
                            state.input_history.push(user_msg.clone());
                            state.history_idx = None;
                            state.input_draft.clear();

                            if user_msg.starts_with('/') {
                                // 技能执行
                                let after_slash = user_msg[1..].to_string();
                                let mut parts_iter = after_slash.splitn(2, ' ');
                                let skill_name = parts_iter.next().unwrap_or("").to_string();
                                let args = parts_iter.next().unwrap_or("").to_string();

                                // /help 列出全部技能
                                if skill_name == "help" {
                                    let mut help_text = String::from("可用命令:\n  /read <path>[:start[-end]]  读取文件\n  /write <path>              写入最近代码块（需确认）\n  /edit <path> <old> <new>    编辑文件（需确认）\n  /clear                     清空对话（需确认）\n  /skills                    查看技能列表\n  /addskill <name> <cmd>     添加技能\n  /rmskill <name>            删除技能\n\n技能:\n");
                                    let mut names: Vec<&String> = state.skills.keys().collect();
                                    names.sort();
                                    for name in &names {
                                        help_text.push_str(&format!("  /{}\n", name));
                                    }
                                    help_text.push_str("\n支持参数: /skill_name args");
                                    state.messages.push(ChatMessage {
                                        role: "assistant".to_string(),
                                        content: Content::text(help_text),
                                        cache_control: None,
                                    });
                                } else if skill_name == "read" {
                                    handle_read_command(state, &args, &client, &token_tx);
                                } else if skill_name == "write" {
                                    handle_write_command(state, &args, &client, &token_tx);
                                } else if skill_name == "edit" {
                                    handle_edit_command(state, &args, &client, &token_tx);
                                } else if skill_name == "clear" {
                                    if !state.messages.is_empty() {
                                        state.pending_confirm = Some(ConfirmState {
                                            action: ConfirmAction::ClearChat,
                                            detail: format!("清空 {} 条对话（不可恢复）", state.messages.len()),
                                        });
                                    }
                                } else if skill_name == "skills" {
                                    let mut text = format!("技能列表 ({} 个):\n", state.skills.len());
                                    let mut names: Vec<(&String, &String)> = state.skills.iter().collect();
                                    names.sort_by_key(|(k, _)| (*k).clone());
                                    for (name, cmd) in &names {
                                        text.push_str(&format!("  /{} → {}\n", name, cmd));
                                    }
                                    text.push_str("\n/addskill <name> <cmd>  添加\n/rmskill <name>         删除");
                                    state.messages.push(ChatMessage {
                                        role: "assistant".to_string(),
                                        content: Content::text(text),
                                        cache_control: None,
                                    });
                                } else if skill_name == "addskill" {
                                    let parts: Vec<&str> = args.splitn(2, ' ').collect();
                                    if parts.len() < 2 {
                                        state.error_message = Some("用法: /addskill <name> <command>".into());
                                    } else {
                                        let name = parts[0].to_string();
                                        let cmd = parts[1].to_string();
                                        state.skills.insert(name.clone(), cmd.clone());
                                        state.config.skills = state.skills.clone();
                                        match state.config.save() {
                                            Ok(()) => {
                                                state.messages.push(ChatMessage {
                                                    role: "assistant".to_string(),
                                                    content: Content::text(format!("✓ 已添加 /{} → {}", name, cmd)),
                                                    cache_control: None,
                                                });
                                            }
                                            Err(e) => {
                                                state.error_message = Some(format!("保存配置失败: {}", e));
                                            }
                                        }
                                    }
                                } else if skill_name == "rmskill" {
                                    if args.is_empty() {
                                        state.error_message = Some("用法: /rmskill <name>".into());
                                    } else if state.skills.remove(&args).is_some() {
                                        state.config.skills = state.skills.clone();
                                        match state.config.save() {
                                            Ok(()) => {
                                                state.messages.push(ChatMessage {
                                                    role: "assistant".to_string(),
                                                    content: Content::text(format!("✓ 已删除 /{}", args)),
                                                    cache_control: None,
                                                });
                                            }
                                            Err(e) => {
                                                state.error_message = Some(format!("保存配置失败: {}", e));
                                            }
                                        }
                                    } else {
                                        state.error_message = Some(format!("技能 /{} 不存在", args));
                                    }
                                } else if let Some(cmd_tpl) = state.skills.get(&skill_name).cloned() {
                                    // 参数替换
                                    let cmd = if cmd_tpl.contains("{args}") {
                                        cmd_tpl.replace("{args}", &args)
                                    } else if !args.is_empty() {
                                        format!("{} {}", cmd_tpl, args)
                                    } else {
                                        cmd_tpl
                                    };

                                    state.generating = true;
                                    state.stream_buffer.clear();
                                    state.spinner_tick = 0;

                                    let client = Arc::clone(client);
                                    let tx = token_tx.clone();
                                    let messages_clone = state.messages.clone();
                                    let system_text = state.system_prompt_text.clone();
                                    let skill_name_owned = skill_name.clone();

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

                                        let start = Instant::now();
                                        let output =
                                            tokio::process::Command::new(shell)
                                                .arg(shell_flag)
                                                .arg(&cmd)
                                                .output()
                                                .await;
                                        let elapsed = start.elapsed();

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

                                                // 输出截断
                                                let truncated = result.len() > 2000;
                                                if truncated {
                                                    result.truncate(2000);
                                                    result.push_str("\n... (truncated)");
                                                }

                                                let display = format!(
                                                    "/{} output ({}ms):\n{}",
                                                    skill_name_owned,
                                                    elapsed.as_millis(),
                                                    result
                                                );
                                                let _ = tx.send(StreamResult::Token(display));

                                                // 用技能输出作为用户消息，发送给 MiMo 分析
                                                let mut msgs = messages_clone;
                                                msgs.push(ChatMessage {
                                                    role: "user".to_string(),
                                                    content: Content::text(format!(
                                                        "请分析以下 /{} 命令的输出:\n{}",
                                                        skill_name_owned, result
                                                    )),
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
                                                    skill_name_owned, e
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
                                    content: Content::text(user_msg),
                                    cache_control: None,
                                });

                                state.generating = true;
                                state.stream_buffer.clear();
                                state.spinner_tick = 0;

                                // 首条消息注入日期（替代原来写在 system prompt 的做法）
                                if state.messages.len() == 1 {
                                    let today = chrono_date();
                                    state.messages[0].content.as_mut_str().push_str(
                                        &format!("\n\n[Current date: {}]", today)
                                    );
                                }

                                let system_text = state.system_prompt_text.clone();

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
                    // 输入字符（非搜索模式、非确认状态）
                    KeyCode::Char(ch) if !state.generating && !state.search_active && state.pending_confirm.is_none() && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.input.insert(state.cursor_pos, ch);
                        state.cursor_pos += ch.len_utf8();
                        update_hint_lines(state);
                    }
                    // 退格
                    KeyCode::Backspace if !state.generating && !state.search_active => {
                        if state.cursor_pos > 0 {
                            let prev = state.input[..state.cursor_pos]
                                .chars()
                                .next_back()
                                .map(|c| state.cursor_pos - c.len_utf8())
                                .unwrap_or(0);
                            state.input.remove(prev);
                            state.cursor_pos = prev;
                            update_hint_lines(state);
                        }
                    }
                    // Delete
                    KeyCode::Delete if !state.generating && !state.search_active => {
                        if state.cursor_pos < state.input.len() {
                            state.input.remove(state.cursor_pos);
                            update_hint_lines(state);
                        }
                    }
                    // Ctrl+A
                    KeyCode::Char('a')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !state.generating && !state.search_active =>
                    {
                        state.cursor_pos = 0;
                    }
                    // End / Ctrl+E
                    KeyCode::End if !state.generating && !state.search_active => {
                        state.cursor_pos = state.input.len();
                    }
                    KeyCode::Char('e')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !state.generating && !state.search_active =>
                    {
                        state.cursor_pos = state.input.len();
                    }
                    // 上箭头 — 浏览输入历史
                    KeyCode::Up if !state.generating => {
                        if !state.input_history.is_empty() {
                            match state.history_idx {
                                None => {
                                    state.input_draft = state.input.clone();
                                    state.history_idx = Some(state.input_history.len() - 1);
                                    state.input = state.input_history.last().unwrap().clone();
                                    state.cursor_pos = state.input.len();
                                }
                                Some(0) => {}
                                Some(idx) => {
                                    state.history_idx = Some(idx - 1);
                                    state.input = state.input_history[idx - 1].clone();
                                    state.cursor_pos = state.input.len();
                                }
                            }
                            update_hint_lines(state);
                        }
                    }
                    // 下箭头 — 前进到更新的历史
                    KeyCode::Down if !state.generating => {
                        if let Some(idx) = state.history_idx {
                            if idx + 1 < state.input_history.len() {
                                state.history_idx = Some(idx + 1);
                                state.input = state.input_history[idx + 1].clone();
                            } else {
                                state.history_idx = None;
                                state.input = state.input_draft.clone();
                            }
                            state.cursor_pos = state.input.len();
                            update_hint_lines(state);
                        }
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

/// 保存当前会话（消息 + 时间戳）
fn save_session(state: &mut AppState) {
    state.session.messages = state.messages.clone();
    state.session.updated_at = now_secs();
    let _ = state.session.save();
}

/// 推送一条"已取消"助手消息
fn push_cancelled_message(state: &mut AppState) {
    state.messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: Content::text("已取消"),
        cache_control: None,
    });
}

fn calculate_cost(input_tokens: u64, output_tokens: u64) -> f64 {
    // MiMo Token Plan 粗略费率（具体以官方为准）
    let input_cost = input_tokens as f64 / 1_000_000.0 * 2.0; // ¥2/百万 input tokens
    let output_cost = output_tokens as f64 / 1_000_000.0 * 8.0; // ¥8/百万 output tokens
    input_cost + output_cost
}

// ── 缓存优化 ──

fn chrono_date() -> String {
    let total_secs = now_secs();
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

fn build_system_prompt_text(project_tree: &str) -> String {
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

fn apply_cache_breakpoints(messages: &mut [ChatMessage]) {
    let n = messages.len();
    if n == 0 {
        return;
    }
    messages[0].cache_control = Some(CacheControl {
        cache_type: "ephemeral".to_string(),
    });
    for i in (3..n).step_by(6) {
        messages[i].cache_control = Some(CacheControl {
            cache_type: "ephemeral".to_string(),
        });
    }
}

fn update_hint_lines(state: &mut AppState) {
    if let Some(rest) = state.input.strip_prefix('/') {
        if rest.contains(' ') {
            state.hint_lines.clear();
        } else {
            let prefix = rest.to_lowercase();
            let mut hints: Vec<String> = Vec::new();

            // 内置命令
            for cmd in &["read", "write", "edit", "clear", "skills", "addskill", "rmskill", "help"] {
                if cmd.starts_with(&prefix) && *cmd != prefix {
                    hints.push(format!("/{}", cmd));
                }
            }

            // 用户技能
            let mut skill_hints: Vec<&String> = state.skills.keys()
                .filter(|k| k.to_lowercase().starts_with(&prefix) && **k != prefix)
                .collect();
            skill_hints.sort();
            for k in skill_hints {
                hints.push(format!("/{}", k));
            }

            state.hint_lines = hints;
        }
    } else {
        state.hint_lines.clear();
    }
}

fn do_search(state: &mut AppState) {
    state.search_matches.clear();
    state.search_match_idx = 0;
    if state.search_query.is_empty() {
        return;
    }
    let query = state.search_query.to_lowercase();
    for (i, msg) in state.messages.iter().enumerate() {
        if msg.content.as_str().to_lowercase().contains(&query) {
            state.search_matches.push(i);
        }
    }
}

fn collect_code_blocks(state: &mut AppState) {
    state.code_blocks.clear();
    for msg in &state.messages {
        let text = msg.content.as_str();
        let mut in_block = false;
        let mut lang = String::new();
        let mut lines: Vec<String> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if in_block {
                    let l = if lang.is_empty() { "text".into() } else { lang.clone() };
                    state.code_blocks.push((l, lines.join("\n")));
                    in_block = false;
                    lines.clear();
                } else {
                    in_block = true;
                    lang = trimmed[3..].to_string();
                    lines.clear();
                }
            } else if in_block {
                lines.push(line.to_string());
            }
        }
        if in_block && !lines.is_empty() {
            let l = if lang.is_empty() { "text".into() } else { lang };
            state.code_blocks.push((l, lines.join("\n")));
        }
    }
}

// ── 文件操作命令 ──

fn handle_read_command(
    state: &mut AppState,
    args: &str,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::UnboundedSender<StreamResult>,
) {
    if args.is_empty() {
        state.error_message = Some("用法: /read <path>[:start[-end]]".into());
        return;
    }

    let cwd = match get_cwd() {
        Ok(p) => p,
        Err(e) => {
            state.error_message = Some(e);
            return;
        }
    };

    // 解析 path[:start[-end]]
    let (path_str, line_range) = if let Some(idx) = args.rfind(':') {
        let before = &args[..idx];
        let after = &args[idx + 1..];
        if before.contains('/') || before.contains('\\') {
            // 路径中的冒号（如 Windows 盘符 C:），不拆分
            (args.to_string(), None)
        } else if let Some(dash_idx) = after.find('-') {
            let start: usize = after[..dash_idx].parse().unwrap_or(0);
            let end: usize = after[dash_idx + 1..].parse().unwrap_or(0);
            if start > 0 && end >= start {
                (before.to_string(), Some((start, end)))
            } else {
                (before.to_string(), None)
            }
        } else if let Ok(line_num) = after.parse::<usize>() {
            (before.to_string(), Some((line_num, line_num)))
        } else {
            (args.to_string(), None)
        }
    } else {
        (args.to_string(), None)
    };

    let full_path = match file_ops::validate_path(&path_str, &cwd) {
        Ok(p) => p,
        Err(e) => {
            state.error_message = Some(e);
            return;
        }
    };

    match file_ops::read_file(&full_path, line_range) {
        Ok(content) => {
            let display = format!("/read {}:\n{}", path_str, content);
            state.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: Content::text(display),
                cache_control: None,
            });

            // 发送给 MiMo 分析（截断避免 token 浪费）
            let mut analysis_content = content;
            if analysis_content.len() > 3000 {
                analysis_content.truncate(3000);
                analysis_content.push_str("\n... (truncated)");
            }
            send_to_mimo(
                state,
                client,
                token_tx,
                format!("请简要分析文件 {} 的内容:\n{}", path_str, analysis_content),
            );
        }
        Err(e) => {
            state.error_message = Some(format!("/read: {}", e));
        }
    }
}

fn handle_write_command(
    state: &mut AppState,
    args: &str,
    _client: &Arc<MiMoClient>,
    _token_tx: &mpsc::UnboundedSender<StreamResult>,
) {
    if args.is_empty() {
        state.error_message = Some("用法: /write <path>".into());
        return;
    }

    let cwd = match get_cwd() {
        Ok(p) => p,
        Err(e) => {
            state.error_message = Some(e);
            return;
        }
    };

    let full_path = match file_ops::validate_path(args, &cwd) {
        Ok(p) => p,
        Err(e) => {
            state.error_message = Some(e);
            return;
        }
    };

    // 从对话历史中提取最后一个代码块
    let all_messages: Vec<&ChatMessage> = state.messages.iter().collect();
    let code = extract_last_code_block(&all_messages, &state.stream_buffer);

    match code {
        Some((lang, content)) => {
            let exists = full_path.exists();
            let action = ConfirmAction::WriteFile {
                path: args.to_string(),
                code: content,
                lang,
            };
            let detail = if exists {
                format!("覆盖已存在的文件: {}（将自动备份）", args)
            } else {
                format!("写入新文件: {}", args)
            };
            state.pending_confirm = Some(ConfirmState { action, detail });
        }
        None => {
            state.error_message = Some("对话中未找到代码块".into());
        }
    }
}

fn handle_edit_command(
    state: &mut AppState,
    args: &str,
    _client: &Arc<MiMoClient>,
    _token_tx: &mpsc::UnboundedSender<StreamResult>,
) {
    // 格式: /edit <path> <old> <new>
    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    if parts.len() < 3 {
        state.error_message = Some("用法: /edit <path> <old> <new>".into());
        return;
    }

    let path_str = parts[0];
    let old_text = parts[1];
    let new_text = parts[2];

    let cwd = match get_cwd() {
        Ok(p) => p,
        Err(e) => {
            state.error_message = Some(e);
            return;
        }
    };

    let full_path = match file_ops::validate_path(path_str, &cwd) {
        Ok(p) => p,
        Err(e) => {
            state.error_message = Some(e);
            return;
        }
    };

    // 预检查：文件是否存在、文本是否匹配
    match std::fs::read_to_string(&full_path) {
        Ok(content) => {
            if !content.contains(old_text) {
                state.error_message = Some("/edit: 未找到匹配文本".into());
                return;
            }
            let count = content.matches(old_text).count();
            let action = ConfirmAction::EditFile {
                path: path_str.to_string(),
                old: old_text.to_string(),
                new: new_text.to_string(),
            };
            let detail = format!(
                "编辑 {}：替换 {} 处 \"{}\" → \"{}\"",
                path_str, count, old_text, new_text
            );
            state.pending_confirm = Some(ConfirmState { action, detail });
        }
        Err(e) => {
            state.error_message = Some(format!("/edit: 读取失败: {}", e));
        }
    }
}

// ── 确认对话框处理 ──

fn execute_confirm(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::UnboundedSender<StreamResult>,
) {
    let confirm = match state.pending_confirm.take() {
        Some(c) => c,
        None => return,
    };

    match confirm.action {
        ConfirmAction::WriteFile { path, code, lang } => {
            let cwd = match std::env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    state.error_message = Some(format!("获取工作目录失败: {}", e));
                    return;
                }
            };
            let full_path = match file_ops::validate_path(&path, &cwd) {
                Ok(p) => p,
                Err(e) => {
                    state.error_message = Some(e);
                    return;
                }
            };
            match file_ops::write_file(&full_path, &code) {
                Ok(msg) => {
                    state.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: Content::text(format!("✓ {} (from {} code block)", msg, lang)),
                        cache_control: None,
                    });
                    send_to_mimo(
                        state,
                        client,
                        token_tx,
                        format!("代码已写入 {}:\n```{}\n{}\n```\n请简要确认写入的内容。", path, lang, code),
                    );
                }
                Err(e) => {
                    state.error_message = Some(format!("/write: {}", e));
                }
            }
        }
        ConfirmAction::EditFile { path, old, new } => {
            let cwd = match std::env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    state.error_message = Some(format!("获取工作目录失败: {}", e));
                    return;
                }
            };
            let full_path = match file_ops::validate_path(&path, &cwd) {
                Ok(p) => p,
                Err(e) => {
                    state.error_message = Some(e);
                    return;
                }
            };
            match file_ops::apply_edit(&full_path, &old, &new) {
                Ok(msg) => {
                    state.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: Content::text(format!("✓ {}", msg)),
                        cache_control: None,
                    });
                    send_to_mimo(
                        state,
                        client,
                        token_tx,
                        format!("文件 {} 已编辑：将 \"{}\" 替换为 \"{}\"。请简要确认。", path, old, new),
                    );
                }
                Err(e) => {
                    state.error_message = Some(format!("/edit: {}", e));
                }
            }
        }
        ConfirmAction::ClearChat => {
            let count = state.messages.len();
            state.messages.clear();
            state.stream_buffer.clear();
            state.chat_scroll = 0;
            state.search_active = false;
            state.search_query.clear();
            state.search_matches.clear();
            state.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: Content::text(format!("已清空 {} 条对话", count)),
                cache_control: None,
            });
        }
        ConfirmAction::Quit => {
            save_session(state);
            state.should_quit = true;
        }
    }
}

fn show_confirm_detail(state: &mut AppState) {
    let detail = match &state.pending_confirm {
        Some(c) => match &c.action {
            ConfirmAction::WriteFile { path, code, lang } => {
                let preview: String = code.lines().take(15).collect::<Vec<_>>().join("\n");
                let truncated = code.lines().count() > 15;
                format!(
                    "将写入 {}\n语言: {}\n大小: {} B\n\n预览:\n{}\n{}",
                    path, lang, code.len(),
                    preview,
                    if truncated { "\n... (更多内容已省略)" } else { "" }
                )
            }
            ConfirmAction::EditFile { path, old, new, .. } => {
                format!("文件: {}\n\n查找:\n{}\n\n替换为:\n{}", path, old, new)
            }
            ConfirmAction::ClearChat => {
                format!("将清空 {} 条对话消息（不可恢复）", state.messages.len())
            }
            ConfirmAction::Quit => c.detail.clone(),
        },
        None => return,
    };

    state.messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: Content::text(format!("详情:\n{}", detail)),
        cache_control: None,
    });
}

/// 从对话历史中提取最后一个代码块
fn extract_last_code_block<'a>(
    messages: &[&'a ChatMessage],
    stream_buffer: &'a str,
) -> Option<(String, String)> {
    // 先从 stream_buffer 找（如果有生成中的内容）
    if !stream_buffer.is_empty() {
        if let Some(result) = extract_code_from_text(stream_buffer) {
            return Some(result);
        }
    }

    // 从后往前搜 messages
    for msg in messages.iter().rev() {
        if let Some(result) = extract_code_from_text(msg.content.as_str()) {
            return Some(result);
        }
    }
    None
}

fn extract_code_from_text(text: &str) -> Option<(String, String)> {
    let mut last_code: Option<(String, Vec<String>)> = None;
    let mut in_block = false;
    let mut current_lang = String::new();
    let mut current_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            if in_block {
                last_code = Some((current_lang.clone(), current_lines.clone()));
                in_block = false;
                current_lines.clear();
            } else {
                in_block = true;
                current_lang = trimmed[3..].to_string();
                current_lines.clear();
            }
        } else if in_block {
            current_lines.push(line.to_string());
        }
    }

    // 处理未闭合的代码块（流式场景）
    if in_block && !current_lines.is_empty() {
        last_code = Some((current_lang, current_lines));
    }

    last_code.map(|(lang, lines)| {
        let l = if lang.is_empty() { "text".to_string() } else { lang };
        (l, lines.join("\n"))
    })
}

/// 将内容发送给 MiMo 分析（共享逻辑）
fn send_to_mimo(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::UnboundedSender<StreamResult>,
    analysis_msg: String,
) {
    state.generating = true;
    state.stream_buffer.clear();
    state.spinner_tick = 0;

    // 首条消息注入日期
    if state.messages.len() == 1 {
        let today = chrono_date();
        state.messages[0]
            .content
            .as_mut_str()
            .push_str(&format!("\n\n[Current date: {}]", today));
    }

    let mut msgs = state.messages.clone();
    msgs.push(ChatMessage {
        role: "user".to_string(),
        content: Content::text(analysis_msg),
        cache_control: None,
    });
    apply_cache_breakpoints(&mut msgs);

    let system_text = state.system_prompt_text.clone();
    let system_content = build_system_content(&system_text);

    let client = Arc::clone(client);
    let tx = token_tx.clone();

    tokio::spawn(async move {
        let result = client
            .send_message_stream(&system_content, &msgs, tx.clone())
            .await;
        if let Err(e) = result {
            let _ = tx.send(StreamResult::Error(e.to_string()));
        }
    });
}

fn scan_project_tree() -> String {
    let cwd = match get_cwd() {
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
