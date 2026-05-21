use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::{mpsc, oneshot};

use crate::api::{ChatMessage, Content, MiMoClient, StreamResult};
use crate::commands;
use crate::config::Config;
use crate::prompt;
use crate::scanner;
use crate::session::{Session, SessionInfo};
use crate::ui;

const MAX_MESSAGES: usize = 200;
const MESSAGE_WARN_THRESHOLD: usize = 160;

#[derive(Clone)]
pub enum ConfirmAction {
    WriteFile {
        path: String,
        code: String,
        lang: String,
    },
    EditFile {
        path: String,
        old: String,
        new: String,
    },
    SendMessage {
        message: String,
    },
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
    pub error_history: Vec<String>,
    pub should_quit: bool,
    pub api_ok: Option<bool>,
    pub api_error_detail: Option<String>,
    pub system_prompt_text: String,
    pub project_tree: String,
    pub skills: HashMap<String, String>,
    pub input_history: Vec<String>,
    pub history_idx: Option<usize>,
    pub input_draft: String,
    pub hint_lines: Vec<String>,
    pub chat_scroll: usize,
    pub search_active: bool,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub search_match_idx: usize,
    pub pending_confirm: Option<ConfirmState>,
    pub code_blocks: Vec<(String, String)>,
    pub copy_status: Option<String>,
    pub session: Session,
    pub session_list: Vec<SessionInfo>,
    pub cancel_tx: Option<oneshot::Sender<()>>,
    pub balance_info: Option<(String, f64)>,
    pub balance_rx: Option<oneshot::Receiver<Option<(String, f64)>>>,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

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

    let project_tree = scanner::scan_project_tree();
    let skills = config.skills.clone();

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
        total_input_tokens: session.total_input_tokens,
        total_output_tokens: session.total_output_tokens,
        total_cache_creation_tokens: session.total_cache_creation_tokens,
        total_cache_read_tokens: session.total_cache_read_tokens,
        total_cost: session.total_cost,
        error_message: None,
        error_history: Vec::new(),
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
        cancel_tx: None,
        balance_info: None,
        balance_rx: None,
    };

    // DeepSeek 余额查询
    if state.config.provider == "deepseek" {
        match client.query_deepseek_balance().await {
            Some(bal) => state.balance_info = Some(bal),
            None => {
                log::warn!("启动时余额查询失败");
                state.error_message = Some("余额查询失败，请检查网络和 API Key".to_string());
            }
        }
    }

    state.system_prompt_text =
        prompt::build_system_prompt_text(&state.project_tree, &state.config.provider);

    let (token_tx, mut token_rx) = mpsc::channel::<StreamResult>(256);

    let result = run_loop(&mut terminal, &mut state, &client, &token_tx, &mut token_rx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // 退出汇总
    if state.total_input_tokens > 0 || state.total_output_tokens > 0 {
        eprintln!();
        eprintln!("  MiMo-OPT 本次会话汇总");
        eprintln!("  ─────────────────────");
        eprintln!("  消息数:  {}", state.messages.len());
        eprintln!(
            "  Token:   ↓{} ↑{}",
            state.total_input_tokens, state.total_output_tokens
        );
        eprintln!(
            "  命中率:  {}%",
            if state.total_input_tokens + state.total_cache_read_tokens > 0 {
                (state.total_cache_read_tokens as f64
                    / (state.total_input_tokens + state.total_cache_read_tokens) as f64
                    * 100.0) as u32
            } else {
                0
            }
        );
        eprintln!("  费用:    ￥{:.4}", state.total_cost);
        eprintln!();
    }

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
    token_rx: &mut mpsc::Receiver<StreamResult>,
) -> anyhow::Result<()> {
    loop {
        let theme = ui::get_theme(&state.config.theme);
        terminal.draw(|f| ui::draw(f, state, theme))?;

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
                    state.cancel_tx = None;
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
                    state.total_cost = calculate_cost(state);
                    state.stream_buffer.clear();
                    state.chat_scroll = 0;
                    state.api_ok = Some(true);
                    state.api_error_detail = None;
                    log::debug!(
                        "消息完成: input={}, output={}, cache_hit={}",
                        input_tokens,
                        output_tokens,
                        cache_read_tokens
                    );

                    if state.messages.len().is_multiple_of(5) {
                        commands::save_session(state);
                    }
                }
                StreamResult::Error(err) => {
                    state.generating = false;
                    state.cancel_tx = None;
                    if !state.stream_buffer.is_empty() {
                        state.messages.push(ChatMessage {
                            role: "assistant".to_string(),
                            content: Content::text(state.stream_buffer.clone()),
                            cache_control: None,
                        });
                        state.stream_buffer.clear();
                    }
                    state.error_message = Some(err.clone());
                    state.error_history.push(err.clone());
                    if state.error_history.len() > 20 {
                        state.error_history.remove(0);
                    }
                    state.api_ok = Some(false);
                    state.api_error_detail = Some(err);
                    state.chat_scroll = 0;
                }
            }
        }

        // 余额查询结果
        if let Some(mut rx) = state.balance_rx.take() {
            match rx.try_recv() {
                Ok(Some(bal)) => state.balance_info = Some(bal),
                Ok(None) => {
                    log::warn!("余额查询返回空结果");
                    state.error_message = Some("余额查询失败，请检查网络和 API Key".to_string());
                }
                Err(_) => {}
            }
        }

        // 键盘事件
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if state.copy_status.is_some() {
                    state.copy_status = None;
                }

                match key.code {
                    // ── 确认对话框 ──
                    KeyCode::Char('y')
                        if state.pending_confirm.is_some()
                            && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        commands::execute_confirm(state, client, token_tx);
                    }
                    KeyCode::Char('n')
                        if state.pending_confirm.is_some()
                            && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        if let Some(ref c) = state.pending_confirm {
                            if let ConfirmAction::SendMessage { ref message } = c.action {
                                state.input = message.clone();
                                state.cursor_pos = state.input.len();
                            }
                        }
                        state.pending_confirm = None;
                        commands::push_cancelled_message(state);
                    }
                    KeyCode::Esc if state.pending_confirm.is_some() => {
                        if let Some(ref c) = state.pending_confirm {
                            if let ConfirmAction::SendMessage { ref message } = c.action {
                                state.input = message.clone();
                                state.cursor_pos = state.input.len();
                            }
                        }
                        state.pending_confirm = None;
                        commands::push_cancelled_message(state);
                    }
                    KeyCode::Char('d')
                        if state.pending_confirm.is_some()
                            && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        commands::show_confirm_detail(state);
                    }

                    // ── 会话管理 ──
                    KeyCode::Char('n')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.pending_confirm.is_none() =>
                    {
                        commands::save_session(state);
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
                    KeyCode::F(2) if !state.generating && state.pending_confirm.is_none() => {
                        commands::save_session(state);
                        state.session_list = Session::list();
                        if state.session_list.len() > 1 {
                            let current_idx = state
                                .session_list
                                .iter()
                                .position(|s| s.id == state.session.id)
                                .unwrap_or(0);
                            let next_idx = (current_idx + 1) % state.session_list.len();
                            let next_id = state.session_list[next_idx].id.clone();
                            if let Ok(s) = Session::load(&next_id) {
                                log::info!("切换会话: {} → {}", state.session.id, next_id);
                                state.messages = s.messages.clone();
                                state.total_input_tokens = s.total_input_tokens;
                                state.total_output_tokens = s.total_output_tokens;
                                state.total_cache_creation_tokens = s.total_cache_creation_tokens;
                                state.total_cache_read_tokens = s.total_cache_read_tokens;
                                state.total_cost = s.total_cost;
                                state.session = s;
                                state.stream_buffer.clear();
                                state.chat_scroll = 0;
                            }
                        }
                    }

                    // ── 退出 ──
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !state.messages.is_empty() && state.pending_confirm.is_none() {
                            state.pending_confirm = Some(ConfirmState {
                                action: ConfirmAction::Quit,
                                detail: format!("退出将丢失 {} 条对话", state.messages.len()),
                            });
                        } else {
                            commands::save_session(state);
                            state.should_quit = true;
                        }
                    }

                    // ── 中断生成 ──
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.pending_confirm.is_none()
                            && state.generating =>
                    {
                        if let Some(cancel_tx) = state.cancel_tx.take() {
                            let _ = cancel_tx.send(());
                        }
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

                    // ── 粘贴 ──
                    KeyCode::Char('v')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !state.generating
                            && !state.search_active
                            && state.pending_confirm.is_none() =>
                    {
                        match cli_clipboard::get_contents() {
                            Ok(text) => {
                                state.input.insert_str(state.cursor_pos, &text);
                                state.cursor_pos += text.len();
                                commands::update_hint_lines(state);
                            }
                            Err(e) => {
                                state.error_message = Some(format!("粘贴失败: {}", e));
                            }
                        }
                    }

                    // ── 搜索 ──
                    KeyCode::Char('f')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.pending_confirm.is_none()
                            && !state.generating
                            && !state.search_active =>
                    {
                        state.search_active = true;
                        state.search_query.clear();
                        state.search_matches.clear();
                        state.search_match_idx = 0;
                    }

                    // ── 复制代码块 ──
                    KeyCode::Char('y')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.pending_confirm.is_none() =>
                    {
                        commands::collect_code_blocks(state);
                        if let Some((lang, content)) = state.code_blocks.last() {
                            match cli_clipboard::set_contents(content.clone()) {
                                Ok(()) => {
                                    state.copy_status = Some(format!(
                                        "已复制 {} 代码块 ({} 行)",
                                        lang,
                                        content.lines().count()
                                    ));
                                }
                                Err(e) => {
                                    state.copy_status = Some(format!("复制失败: {}", e));
                                }
                            }
                        } else {
                            state.copy_status = Some("对话中未找到代码块".into());
                        }
                    }

                    // ── Esc: 退出搜索 / 中断生成 ──
                    KeyCode::Esc if state.search_active => {
                        state.search_active = false;
                        state.search_query.clear();
                        state.search_matches.clear();
                    }
                    KeyCode::Esc if state.generating => {
                        if let Some(cancel_tx) = state.cancel_tx.take() {
                            let _ = cancel_tx.send(());
                        }
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

                    // ── 搜索模式 ──
                    KeyCode::Char(ch)
                        if state.search_active
                            && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        state.search_query.push(ch);
                        commands::do_search(state);
                    }
                    KeyCode::Backspace if state.search_active => {
                        state.search_query.pop();
                        commands::do_search(state);
                    }
                    KeyCode::Enter if state.search_active && !state.search_matches.is_empty() => {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            if state.search_match_idx > 0 {
                                state.search_match_idx -= 1;
                            } else {
                                state.search_match_idx = state.search_matches.len() - 1;
                            }
                        } else {
                            state.search_match_idx =
                                (state.search_match_idx + 1) % state.search_matches.len();
                        }
                    }

                    // ── 滚动 ──
                    KeyCode::PageUp if !state.generating && !state.search_active => {
                        state.chat_scroll = state.chat_scroll.saturating_add(10);
                    }
                    KeyCode::PageDown if !state.generating && !state.search_active => {
                        state.chat_scroll = state.chat_scroll.saturating_sub(10);
                    }
                    KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.chat_scroll = 0;
                    }

                    // ── 发送消息 ──
                    KeyCode::Enter
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.pending_confirm.is_none()
                            && !state.input.is_empty()
                            && !state.generating =>
                    {
                        if !state.input.starts_with('/') && state.input.lines().count() > 5 {
                            let msg = state.input.clone();
                            state.pending_confirm = Some(ConfirmState {
                                action: ConfirmAction::SendMessage { message: msg },
                                detail: format!(
                                    "{} 行消息，确认发送？",
                                    state.input.lines().count()
                                ),
                            });
                            state.input.clear();
                            state.cursor_pos = 0;
                            break;
                        }
                        let user_msg = state.input.clone();
                        state.input.clear();
                        state.cursor_pos = 0;
                        state.error_message = None;
                        state.input_history.push(user_msg.clone());
                        state.history_idx = None;
                        state.input_draft.clear();

                        if let Some(stripped) = user_msg.strip_prefix('/') {
                            let mut parts_iter = stripped.splitn(2, ' ');
                            let cmd = parts_iter.next().unwrap_or("").to_string();
                            let args = parts_iter.next().unwrap_or("").to_string();
                            commands::handle_command(state, client, token_tx, &cmd, &args);
                        } else {
                            state.messages.push(ChatMessage {
                                role: "user".to_string(),
                                content: Content::text(user_msg),
                                cache_control: None,
                            });

                            commands::maybe_truncate_messages(state);
                            if state.messages.len() >= MESSAGE_WARN_THRESHOLD {
                                state.error_message = Some(format!(
                                    "消息数 {}/{}，接近上限",
                                    state.messages.len(),
                                    MAX_MESSAGES
                                ));
                            }

                            commands::spawn_stream_request(state, client, token_tx, None);
                        }
                    }

                    // ── 文本输入 ──
                    KeyCode::Char(ch)
                        if !state.generating
                            && !state.search_active
                            && state.pending_confirm.is_none()
                            && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        state.input.insert(state.cursor_pos, ch);
                        state.cursor_pos += ch.len_utf8();
                        commands::update_hint_lines(state);
                    }
                    KeyCode::Backspace
                        if !state.generating && !state.search_active && state.cursor_pos > 0 =>
                    {
                        let prev = state.input[..state.cursor_pos]
                            .chars()
                            .next_back()
                            .map(|c| state.cursor_pos - c.len_utf8())
                            .unwrap_or(0);
                        state.input.remove(prev);
                        state.cursor_pos = prev;
                        commands::update_hint_lines(state);
                    }
                    KeyCode::Delete
                        if !state.generating
                            && !state.search_active
                            && state.cursor_pos < state.input.len() =>
                    {
                        state.input.remove(state.cursor_pos);
                        commands::update_hint_lines(state);
                    }
                    KeyCode::Char('a')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !state.generating
                            && !state.search_active =>
                    {
                        state.cursor_pos = 0;
                    }
                    KeyCode::End if !state.generating && !state.search_active => {
                        state.cursor_pos = state.input.len();
                    }
                    KeyCode::Char('e')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !state.generating
                            && !state.search_active =>
                    {
                        state.cursor_pos = state.input.len();
                    }

                    // ── 输入历史 ──
                    KeyCode::Up if !state.generating && !state.input_history.is_empty() => {
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
                        commands::update_hint_lines(state);
                    }
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
                            commands::update_hint_lines(state);
                        }
                    }

                    // ── 光标移动 ──
                    KeyCode::Left if !state.generating && state.cursor_pos > 0 => {
                        let prev = state.input[..state.cursor_pos]
                            .chars()
                            .next_back()
                            .map(|c| state.cursor_pos - c.len_utf8())
                            .unwrap_or(0);
                        state.cursor_pos = prev;
                    }
                    KeyCode::Right if !state.generating && state.cursor_pos < state.input.len() => {
                        let next = state.input[state.cursor_pos..]
                            .chars()
                            .next()
                            .map(|c| state.cursor_pos + c.len_utf8())
                            .unwrap_or(state.input.len());
                        state.cursor_pos = next;
                    }
                    _ => {}
                }
            }
        }

        if state.generating {
            state.spinner_tick = state.spinner_tick.wrapping_add(1);
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

fn calculate_cost(state: &AppState) -> f64 {
    let input_cost = state.total_input_tokens as f64 / 1_000_000.0 * state.config.input_price();
    let output_cost =
        state.total_output_tokens as f64 / 1_000_000.0 * state.config.output_price();
    input_cost + output_cost
}
