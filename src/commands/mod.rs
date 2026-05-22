use std::fmt::Write;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, oneshot};

use crate::api::{ChatMessage, Content, MiMoClient, StreamResult};
use crate::app::{AppState, ConfirmAction, ConfirmState, MAX_MESSAGES};
use crate::config::SkillEntry;
use crate::file_ops;
use crate::prompt;
use crate::util::now_secs;

mod file_cmd;

pub const BUILTIN_COMMANDS: &[&str] = &[
    "read", "write", "edit", "clear", "export", "model", "provider", "search", "skills",
    "addskill", "rmskill", "theme", "errors", "compress", "help",
];

// ── 命令分发 ──

pub fn handle_command(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
    cmd: &str,
    args: &str,
) {
    match cmd {
        "help" => {
            let mut text = String::from("可用命令:\n");
            for c in BUILTIN_COMMANDS {
                let desc = match *c {
                    "read" => "读取文件",
                    "write" => "写入最近代码块（需确认）",
                    "edit" => "编辑文件（需确认）",
                    "export" => "导出会话为 Markdown",
                    "clear" => "清空对话（需确认）",
                    "model" => "管理模型（列出/切换）",
                    "provider" => "切换 API 提供商",
                    "search" => "联网搜索 (DDG/DeepSeek)",
                    "skills" => "查看技能列表",
                    "addskill" => "添加技能",
                    "rmskill" => "删除技能",
                    "theme" => "切换主题 (tokyo-night/nord/catppuccin)",
                    "errors" => "查看错误历史",
                    "compress" => "压缩对话上下文（保留摘要）",
                    "help" => "显示此帮助",
                    _ => unreachable!("unknown builtin: {}", c),
                };
                let _ = writeln!(text, "  /{:<16}{}", c, desc);
            }
            if !state.skills.is_empty() {
                text.push_str("\n技能:\n");
                let mut entries: Vec<(&String, &SkillEntry)> = state.skills.iter().collect();
                entries.sort_by_key(|(k, _)| (*k).clone());
                for (name, entry) in &entries {
                    match entry.desc() {
                        Some(desc) => {
                            let _ = writeln!(text, "  /{:<12}{}", name, desc);
                        }
                        None => {
                            let _ = writeln!(text, "  /{}", name);
                        }
                    }
                }
            }
            text.push_str("\n支持参数: /skill_name args");
            push_assistant_msg(state, text);
        }
        "read" => file_cmd::handle_read_command(state, args, client, token_tx),
        "write" => file_cmd::handle_write_command(state, args, client, token_tx),
        "edit" => file_cmd::handle_edit_command(state, args, client, token_tx),
        "clear" => {
            if !state.messages.is_empty() {
                state.pending_confirm = Some(ConfirmState {
                    action: ConfirmAction::ClearChat,
                    detail: format!("清空 {} 条对话（不可恢复）", state.messages.len()),
                });
            }
        }
        "export" => file_cmd::handle_export_command(state, args),
        "search" => handle_search_command(state, client, token_tx, args),
        "errors" => {
            if state.error_history.is_empty() {
                push_assistant_msg(state, "无错误记录".to_string());
            } else {
                let mut text = format!("最近 {} 条错误:\n", state.error_history.len());
                for (i, err) in state.error_history.iter().enumerate() {
                    text.push_str(&format!("  {}. {}\n", i + 1, err));
                }
                push_assistant_msg(state, text);
            }
        }
        "compress" => {
            let result = compress_messages(state);
            push_assistant_msg(state, result);
        }
        "skills" => {
            let mut text = format!("技能列表 ({} 个):\n", state.skills.len());
            let mut entries: Vec<(&String, &SkillEntry)> = state.skills.iter().collect();
            entries.sort_by_key(|(k, _)| (*k).clone());
            for (name, entry) in &entries {
                let analyze_tag = if entry.should_analyze() {
                    ""
                } else {
                    " [不分析]"
                };
                match entry.desc() {
                    Some(desc) => {
                        text.push_str(&format!("  /{} — {}{}\n", name, desc, analyze_tag));
                        text.push_str(&format!("    cmd: {}\n", entry.cmd()));
                    }
                    None => {
                        text.push_str(&format!("  /{} → {}{}\n", name, entry.cmd(), analyze_tag));
                    }
                }
            }
            text.push_str(
                "\n/addskill <name> <cmd> [;desc]  添加 (可选描述)\n/rmskill <name>                 删除",
            );
            push_assistant_msg(state, text);
        }
        "addskill" => {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            if parts.len() < 2 {
                state.error_message = Some("用法: /addskill <name> <command> [;desc]".into());
            } else {
                let name = parts[0].to_string();
                let rest = parts[1];
                // 支持 "cmd ;desc" 格式
                let (cmd, desc) = if let Some(pos) = rest.find(" ;") {
                    let c = rest[..pos].trim().to_string();
                    let d = rest[pos + 2..].trim().to_string();
                    (c, if d.is_empty() { None } else { Some(d) })
                } else {
                    (rest.to_string(), None)
                };
                let entry = SkillEntry::Detailed {
                    cmd: cmd.clone(),
                    desc,
                    analyze: true,
                };
                state.skills.insert(name.clone(), entry);
                save_skills(state);
                push_assistant_msg(state, format!("✓ 已添加 /{} → {}", name, cmd));
            }
        }
        "rmskill" => {
            if args.is_empty() {
                state.error_message = Some("用法: /rmskill <name>".into());
            } else if state.skills.remove(args).is_some() {
                save_skills(state);
                push_assistant_msg(state, format!("✓ 已删除 /{}", args));
            } else {
                state.error_message = Some(format!("技能 /{} 不存在", args));
            }
        }
        "model" => {
            let preset = state.config.current_preset();
            if args.is_empty() {
                if let Some(p) = preset {
                    let mut list = format!("Provider [{}] 可用模型:\n", p.name);
                    for mi in p.models {
                        let current = if mi.name == state.config.model {
                            " (当前)"
                        } else {
                            ""
                        };
                        list.push_str(&format!(
                            "  {}{}  {}  ↓¥{}/Mtok ↑¥{}/Mtok\n",
                            mi.name,
                            current,
                            mi.desc,
                            mi.input_price_per_mtok,
                            mi.output_price_per_mtok
                        ));
                    }
                    list.push_str("\n用法: /model <name>（已知模型有补全提示）");
                    push_assistant_msg(state, list);
                } else {
                    push_assistant_msg(
                        state,
                        format!(
                            "当前为自定义 provider，模型: {}\n用法: /model <name>",
                            state.config.model
                        ),
                    );
                }
            } else {
                let known = preset.and_then(|p| p.find_model(args));
                state.config.model = args.to_string();
                client.set_model(args.to_string());
                state.config.save().ok();
                if let Some(mi) = known {
                    push_assistant_msg(
                        state,
                        format!(
                            "✓ 已切换到 {} ({})\n  ↓¥{}/Mtok ↑¥{}/Mtok",
                            args, mi.desc, mi.input_price_per_mtok, mi.output_price_per_mtok
                        ),
                    );
                } else {
                    push_assistant_msg(
                        state,
                        format!("✓ 模型已切换为 {} (未知模型，费用按默认估算)", args),
                    );
                }
            }
        }
        "provider" => handle_provider_command(state, client, args),
        "theme" => {
            if args.is_empty() {
                let mut list = String::from("可用主题:\n");
                for name in crate::ui::theme_names() {
                    let current = if *name == state.config.theme {
                        " (当前)"
                    } else {
                        ""
                    };
                    list.push_str(&format!("  {}{}\n", name, current));
                }
                list.push_str("\n用法: /theme <name>");
                push_assistant_msg(state, list);
            } else if crate::ui::theme_names().contains(&args) {
                state.config.theme = args.to_string();
                state.config.save().ok();
                push_assistant_msg(state, format!("✓ 主题已切换为 {}", args));
            } else {
                state.error_message = Some(format!(
                    "未知主题: {}。可用: {}",
                    args,
                    crate::ui::theme_names().join(", ")
                ));
            }
        }
        _ => handle_user_skill(state, client, token_tx, cmd, args),
    }
}

pub(crate) fn push_assistant_msg(state: &mut AppState, text: String) {
    Arc::make_mut(&mut state.messages).push(ChatMessage {
        role: "assistant".to_string(),
        content: Content::text(text),
        cache_control: None,
    });
}

fn save_skills(state: &mut AppState) {
    state.config.skills = state.skills.clone();
    if let Err(e) = state.config.save() {
        state.error_message = Some(format!("保存配置失败: {}", e));
    }
}

pub fn handle_provider_command(state: &mut AppState, client: &Arc<MiMoClient>, args: &str) {
    if args.is_empty() {
        let mut list = String::from("可用 provider 预设:\n");
        for p in crate::config::PROVIDERS {
            list.push_str(&format!(
                "  {} — {} ({})\n",
                p.name,
                p.default_model().name,
                p.base_url
            ));
        }
        list.push_str("\n用法: /provider <name>");
        push_assistant_msg(state, list);
    } else if let Some(preset) = crate::config::Config::find_preset(args) {
        let api_key = state.config.api_key.clone();
        state.config.provider = preset.name.to_string();
        state.config.base_url = preset.base_url.to_string();
        state.config.auth_type = preset.auth_type.to_string();
        state.config.api_format = preset.api_format.to_string();
        // Preserve current model if target provider supports it
        if preset.find_model(&state.config.model).is_none() {
            state.config.model = preset.default_model().name.to_string();
        }
        client.update_for_provider(preset, &state.config.model, api_key);
        state.system_stable = prompt::build_system_stable(&state.config.provider);
        state.system_dynamic = prompt::build_system_dynamic(&state.project_tree);
        state.config.save().ok();
        state.balance_info = None;
        if preset.name == "deepseek" {
            let c = Arc::clone(client);
            let (tx, rx) = oneshot::channel();
            state.balance_rx = Some(rx);
            tokio::spawn(async move {
                let bal = c.query_deepseek_balance().await;
                let _ = tx.send(bal);
            });
        }
        push_assistant_msg(
            state,
            format!(
                "✓ 已切换到 {} ({})\nbase_url: {}\nmodel: {}",
                preset.name,
                preset.default_model().name,
                preset.base_url,
                state.config.model
            ),
        );
    } else {
        state.error_message = Some(format!(
            "未知 provider: {}。可用: {}",
            args,
            crate::config::PROVIDERS
                .iter()
                .map(|p| p.name)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}

// ── 用户技能执行 ──

pub fn handle_user_skill(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
    skill_name: &str,
    args: &str,
) {
    let entry = match state.skills.get(skill_name).cloned() {
        Some(e) => e,
        None => {
            state.error_message = Some(format!("未知技能: /{}", skill_name));
            return;
        }
    };

    let cmd_tpl = entry.cmd().to_string();
    let should_analyze = entry.should_analyze();

    let cmd = if cmd_tpl.contains("{args}") {
        cmd_tpl.replace("{args}", &shell_escape(args))
    } else if !args.is_empty() {
        format!("{} {}", cmd_tpl, shell_escape(args))
    } else {
        cmd_tpl
    };

    log::info!("执行技能: /{}", skill_name);

    state.generating = true;
    state.stream_buffer.clear();
    state.spinner_tick = 0;

    let client = Arc::clone(client);
    let tx = token_tx.clone();
    let messages_clone = (*state.messages).clone();
    let stable = state.system_stable.clone();
    let dynamic = state.system_dynamic.clone();
    let skill_name_owned = skill_name.to_string();
    let (cancel_tx, cancel_rx) = oneshot::channel();
    state.cancel_tx = Some(cancel_tx);

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
        let output = tokio::process::Command::new(shell)
            .arg(shell_flag)
            .arg(&cmd)
            .output()
            .await;
        let elapsed = start.elapsed();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let mut result = stdout.to_string();
                if !stderr.is_empty() {
                    result.push_str(&format!("\n[stderr]\n{}", stderr));
                }
                if !out.status.success() {
                    result.push_str(&format!(
                        "\n[exit code: {}]",
                        out.status.code().map_or(-1, |c| c)
                    ));
                }

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
                let _ = tx.send(StreamResult::Token(display)).await;

                if should_analyze {
                    let mut msgs = messages_clone;
                    msgs.push(ChatMessage {
                        role: "user".to_string(),
                        content: Content::text(format!(
                            "请分析以下 /{} 命令的输出:\n{}",
                            skill_name_owned, result
                        )),
                        cache_control: None,
                    });
                    prompt::apply_cache_breakpoints(&mut msgs);
                    let system_content = prompt::build_system_content_split(&stable, &dynamic);

                    tokio::select! {
                        r = client.send_message_stream(&system_content, &msgs, tx.clone()) => {
                            if let Err(e) = r {
                                let _ = tx.send(StreamResult::Error(e.to_string())).await;
                            }
                        }
                        _ = cancel_rx => {}
                    }
                } else {
                    let _ = tx
                        .send(StreamResult::Done {
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_creation_tokens: 0,
                            cache_read_tokens: 0,
                        })
                        .await;
                }
            }
            Err(e) => {
                let _ = tx
                    .send(StreamResult::Error(format!(
                        "/{} 执行失败: {}",
                        skill_name_owned, e
                    )))
                    .await;
            }
        }
    });
}

// ── 联网搜索 ──

fn handle_search_command(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
    args: &str,
) {
    if args.is_empty() {
        state.error_message = Some("用法: /search <关键词>".into());
        return;
    }

    if !state.config.web_search.enabled {
        state.error_message = Some("联网搜索未启用（配置 web_search.enabled = true）".into());
        return;
    }

    log::info!("联网搜索: {}", args);

    push_assistant_msg(state, format!("🔍 正在搜索: {} ...", args));

    let query = args.to_string();
    let web_cfg = state.config.web_search.clone();
    let provider = state.config.provider.clone();

    if provider == "deepseek" && web_cfg.engine == "deepseek" {
        // DeepSeek 原生 web_search tool
        let c = Arc::clone(client);
        let tx = token_tx.clone();
        let msgs = (*state.messages).clone();
        let stable = state.system_stable.clone();
        let dynamic = state.system_dynamic.clone();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        state.cancel_tx = Some(cancel_tx);
        state.generating = true;
        state.stream_buffer.clear();
        state.spinner_tick = 0;

        tokio::spawn(async move {
            let system_content = prompt::build_system_content_split(&stable, &dynamic);
            tokio::select! {
                r = c.send_message_stream_with_search(&system_content, &msgs, &query, tx.clone()) => {
                    if let Err(e) = r {
                        let _ = tx.send(StreamResult::Error(e.to_string())).await;
                    }
                }
                _ = cancel_rx => {}
            }
        });
        return;
    }

    // DDG search — inject results as context and stream AI analysis
    let c = Arc::clone(client);
    let tx = token_tx.clone();
    let msgs = (*state.messages).clone();
    let stable = state.system_stable.clone();
    let dynamic = state.system_dynamic.clone();
    let (cancel_tx, cancel_rx) = oneshot::channel();
    state.cancel_tx = Some(cancel_tx);
    state.generating = true;
    state.stream_buffer.clear();
    state.spinner_tick = 0;

    tokio::spawn(async move {
        match crate::search::execute_search(&query, &web_cfg).await {
            Ok(formatted) => {
                let _ = tx.send(StreamResult::Token(formatted.clone())).await;
                let mut msgs = msgs;
                msgs.push(ChatMessage {
                    role: "user".to_string(),
                    content: Content::text(format!(
                        "请基于以下联网搜索结果回答用户问题: {}\n\n{}",
                        query, formatted
                    )),
                    cache_control: None,
                });
                prompt::apply_cache_breakpoints(&mut msgs);
                let system_content = prompt::build_system_content_split(&stable, &dynamic);
                tokio::select! {
                    r = c.send_message_stream(&system_content, &msgs, tx.clone()) => {
                        if let Err(e) = r {
                            let _ = tx.send(StreamResult::Error(e.to_string())).await;
                        }
                    }
                    _ = cancel_rx => {}
                }
            }
            Err(e) => {
                let _ = tx
                    .send(StreamResult::Error(format!("搜索失败: {}", e)))
                    .await;
            }
        }
    });
}

// ── 确认对话框处理 ──

pub fn execute_confirm(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
) {
    let confirm = match state.pending_confirm.take() {
        Some(c) => c,
        None => return,
    };

    match confirm.action {
        ConfirmAction::WriteFile { path, code, lang } => {
            let full_path = match file_cmd::resolve_path(state, &path) {
                Some(p) => p,
                None => return,
            };
            match file_ops::write_file(&full_path, &code) {
                Ok(msg) => {
                    push_assistant_msg(state, format!("✓ {} (from {} code block)", msg, lang));
                    send_to_mimo(
                        state,
                        client,
                        token_tx,
                        format!(
                            "代码已写入 {}:\n```{}\n{}\n```\n请简要确认写入的内容。",
                            path, lang, code
                        ),
                    );
                }
                Err(e) => {
                    state.error_message = Some(format!("/write: {}", e));
                }
            }
        }
        ConfirmAction::EditFile { path, old, new } => {
            let full_path = match file_cmd::resolve_path(state, &path) {
                Some(p) => p,
                None => return,
            };
            match file_ops::apply_edit(&full_path, &old, &new) {
                Ok(msg) => {
                    push_assistant_msg(state, format!("✓ {}", msg));
                    send_to_mimo(
                        state,
                        client,
                        token_tx,
                        format!(
                            "文件 {} 已编辑：将 \"{}\" 替换为 \"{}\"。请简要确认。",
                            path, old, new
                        ),
                    );
                }
                Err(e) => {
                    state.error_message = Some(format!("/edit: {}", e));
                }
            }
        }
        ConfirmAction::SendMessage { message } => {
            Arc::make_mut(&mut state.messages).push(ChatMessage {
                role: "user".to_string(),
                content: Content::text(message),
                cache_control: None,
            });
            maybe_truncate_messages(state);
            spawn_stream_request(state, client, token_tx, None);
        }
        ConfirmAction::ClearChat => {
            let count = state.messages.len();
            Arc::make_mut(&mut state.messages).clear();
            state.stream_buffer.clear();
            state.chat_scroll = 0;
            state.search_active = false;
            state.search_query.clear();
            state.search_matches.clear();
            push_assistant_msg(state, format!("已清空 {} 条对话", count));
        }
        ConfirmAction::Quit => {
            save_session(state);
            state.should_quit = true;
        }
    }
}

pub(crate) fn show_confirm_detail(state: &mut AppState) {
    let detail = match &state.pending_confirm {
        Some(c) => match &c.action {
            ConfirmAction::WriteFile { path, code, lang } => {
                let preview: String = code.lines().take(15).collect::<Vec<_>>().join("\n");
                let truncated = code.lines().count() > 15;
                format!(
                    "将写入 {}\n语言: {}\n大小: {} B\n\n预览:\n{}\n{}",
                    path,
                    lang,
                    code.len(),
                    preview,
                    if truncated {
                        "\n... (更多内容已省略)"
                    } else {
                        ""
                    }
                )
            }
            ConfirmAction::EditFile { path, old, new, .. } => {
                format!("文件: {}\n\n查找:\n{}\n\n替换为:\n{}", path, old, new)
            }
            ConfirmAction::ClearChat => {
                format!("将清空 {} 条对话消息（不可恢复）", state.messages.len())
            }
            ConfirmAction::Quit => c.detail.clone(),
            ConfirmAction::SendMessage { message } => {
                let preview: String = message.lines().take(10).collect::<Vec<_>>().join("\n");
                format!("{} 行消息:\n{}", message.lines().count(), preview)
            }
        },
        None => return,
    };

    push_assistant_msg(state, format!("详情:\n{}", detail));
}

// ── 流式请求 ──

/// 启动流式请求：克隆消息、注入日期、设置缓存断点、spawn 异步任务
pub fn spawn_stream_request(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
    extra_user_msg: Option<String>,
) {
    state.generating = true;
    state.stream_buffer.clear();
    state.spinner_tick = 0;

    let mut msgs = (*state.messages).clone();
    if let Some(msg_text) = extra_user_msg {
        msgs.push(ChatMessage {
            role: "user".to_string(),
            content: Content::text(msg_text),
            cache_control: None,
        });
    }
    if msgs.len() > MAX_MESSAGES {
        let excess = msgs.len() - KEEP_RECENT;
        if excess > 0 {
            msgs.drain(0..excess);
        }
    }

    prompt::apply_cache_breakpoints(&mut msgs);

    let system_content =
        prompt::build_system_content_split(&state.system_stable, &state.system_dynamic);
    let client = Arc::clone(client);
    let tx = token_tx.clone();
    let (cancel_tx, cancel_rx) = oneshot::channel();
    state.cancel_tx = Some(cancel_tx);

    tokio::spawn(async move {
        tokio::select! {
            result = client.send_message_stream(&system_content, &msgs, tx.clone()) => {
                if let Err(e) = result {
                    let _ = tx.send(StreamResult::Error(e.to_string())).await;
                }
            }
            _ = cancel_rx => {}
        }
    });
}

/// 将内容发送给 MiMo 分析
pub fn send_to_mimo(
    state: &mut AppState,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
    analysis_msg: String,
) {
    spawn_stream_request(state, client, token_tx, Some(analysis_msg));
}

// ── 消息管理 ──

const COMPRESS_THRESHOLD: usize = 180;
const KEEP_RECENT: usize = 100;

/// 检查消息数是否超限，优先压缩旧消息为摘要，超出安全上限才截断
pub fn maybe_truncate_messages(state: &mut AppState) {
    let len = state.messages.len();
    if len <= COMPRESS_THRESHOLD {
        return;
    }
    // 尝试压缩：保留最近 KEEP_RECENT 条，旧消息合成摘要
    let keep = len.min(KEEP_RECENT);
    let split = len - keep;
    if split > 0 {
        let summary = summarize_messages(&state.messages[..split]);
        let mut new_msgs = vec![ChatMessage {
            role: "system".to_string(),
            content: Content::text(summary),
            cache_control: None,
        }];
        new_msgs.extend_from_slice(&state.messages[split..]);
        state.messages = Arc::new(new_msgs);
        log::info!("上下文已压缩: {} → {} 条", len, state.messages.len());
        // 压缩后自动保存，摘要持久化
        save_session(state);
    }
}

/// 手动压缩命令
pub fn compress_messages(state: &mut AppState) -> String {
    let len = state.messages.len();
    if len <= 20 {
        return format!("消息数 {}，无需压缩", len);
    }
    let keep = len / 2;
    let split = len - keep;
    let summary = summarize_messages(&state.messages[..split]);
    let mut new_msgs = vec![ChatMessage {
        role: "system".to_string(),
        content: Content::text(summary),
        cache_control: None,
    }];
    new_msgs.extend_from_slice(&state.messages[split..]);
    state.messages = Arc::new(new_msgs);
    save_session(state);
    format!(
        "已压缩: {} → {} 条消息（已保存）",
        len,
        state.messages.len()
    )
}

/// 将消息列表压缩为结构化摘要
fn summarize_messages(messages: &[ChatMessage]) -> String {
    let mut user_requests: Vec<String> = Vec::new();
    let mut topics: Vec<String> = Vec::new();
    let mut code_langs: Vec<String> = Vec::new();
    let mut assistant_key_points: Vec<String> = Vec::new();

    for msg in messages {
        let text = msg.content.as_str();
        match msg.role.as_str() {
            "user" => {
                // 取前 80 字作为请求摘要
                let preview: String = text.chars().take(80).collect();
                if text.len() > 80 {
                    user_requests.push(format!("{}...", preview));
                } else {
                    user_requests.push(preview);
                }
                // 提取关键词（>3 字的中文词或英文词）
                for word in text.split_whitespace() {
                    let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
                    if clean.len() > 3 && !topics.contains(&clean) && topics.len() < 20 {
                        topics.push(clean);
                    }
                }
            }
            "assistant" => {
                // 提取代码语言
                let mut in_fence = false;
                for line in text.lines() {
                    if line.trim_start().starts_with("```") {
                        if !in_fence {
                            in_fence = true;
                            if let Some(lang) = line.trim_start().strip_prefix("```") {
                                let lang = lang.trim();
                                if !lang.is_empty() && !code_langs.contains(&lang.to_string()) {
                                    code_langs.push(lang.to_string());
                                }
                            }
                        } else {
                            in_fence = false;
                        }
                    }
                }
                // 取第一行非空文本作为关键点
                if assistant_key_points.len() < 5 {
                    if let Some(first_line) = text.lines().find(|l| !l.trim().is_empty()) {
                        let preview: String = first_line.chars().take(60).collect();
                        if first_line.len() > 60 {
                            assistant_key_points.push(format!("{}...", preview));
                        } else {
                            assistant_key_points.push(preview);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let mut summary = String::from("[上下文摘要 — 早期对话已压缩]\n");
    summary.push_str(&format!(
        "共 {} 条消息 ({} 轮对话)\n",
        messages.len(),
        user_requests.len()
    ));

    if !topics.is_empty() {
        summary.push_str(&format!("涉及主题: {}\n", topics.join(", ")));
    }
    if !code_langs.is_empty() {
        summary.push_str(&format!("涉及语言: {}\n", code_langs.join(", ")));
    }
    if !user_requests.is_empty() {
        summary.push_str("用户请求:\n");
        for (i, req) in user_requests.iter().enumerate().take(10) {
            summary.push_str(&format!("  {}. {}\n", i + 1, req));
        }
        if user_requests.len() > 10 {
            summary.push_str(&format!("  ...等 {} 个请求\n", user_requests.len()));
        }
    }
    if !assistant_key_points.is_empty() {
        summary.push_str("关键回复:\n");
        for point in &assistant_key_points {
            summary.push_str(&format!("  - {}\n", point));
        }
    }

    summary
}

/// 保存当前会话（消息 + 时间戳 + token 统计）
pub fn save_session(state: &mut AppState) {
    state.session.messages = Arc::clone(&state.messages);
    state.session.updated_at = now_secs();
    state.session.total_input_tokens = state.total_input_tokens;
    state.session.total_output_tokens = state.total_output_tokens;
    state.session.total_cache_creation_tokens = state.total_cache_creation_tokens;
    state.session.total_cache_read_tokens = state.total_cache_read_tokens;
    state.session.total_cost = state.total_cost;
    let _ = state.session.save();
}

/// 推送一条"已取消"助手消息
pub(crate) fn push_cancelled_message(state: &mut AppState) {
    push_assistant_msg(state, "已取消".to_string());
}

// ── 搜索 + 代码块 ──

pub(crate) fn update_hint_lines(state: &mut AppState) {
    if let Some(rest) = state.input.strip_prefix('/') {
        if let Some((cmd, partial_arg)) = rest.split_once(' ') {
            // Second-level hints for commands with argument completion
            if cmd == "model" && !partial_arg.is_empty() {
                if let Some(preset) = state.config.current_preset() {
                    let lower = partial_arg.to_lowercase();
                    state.hint_lines = preset
                        .models
                        .iter()
                        .filter(|m| {
                            m.name.to_lowercase().starts_with(&lower) && m.name != partial_arg
                        })
                        .map(|m| m.name.to_string())
                        .collect();
                    return;
                }
            }
            if cmd == "provider" && !partial_arg.is_empty() {
                let lower = partial_arg.to_lowercase();
                state.hint_lines = crate::config::PROVIDERS
                    .iter()
                    .filter(|p| p.name.to_lowercase().starts_with(&lower) && p.name != partial_arg)
                    .map(|p| p.name.to_string())
                    .collect();
                return;
            }
            state.hint_lines.clear();
        } else {
            let prefix = rest.to_lowercase();
            let mut hints: Vec<String> = Vec::new();

            for cmd in BUILTIN_COMMANDS {
                if cmd.starts_with(&prefix) && *cmd != prefix {
                    hints.push(format!("/{}", cmd));
                }
            }

            let mut skill_hints: Vec<&String> = state
                .skills
                .keys()
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

pub(crate) fn do_search(state: &mut AppState) {
    state.search_matches.clear();
    state.search_match_idx = 0;
    if state.search_query.is_empty() {
        return;
    }
    let query = state.search_query.to_lowercase();
    for (i, msg) in state.messages.iter().enumerate() {
        if contains_ignore_case(msg.content.as_str(), &query) {
            state.search_matches.push(i);
        }
    }
}

fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let needle: Vec<char> = needle.chars().flat_map(|c| c.to_lowercase()).collect();
    let mut buf: Vec<char> = Vec::with_capacity(needle.len() + 4);
    for hc in haystack.chars() {
        buf.extend(hc.to_lowercase());
        while buf.len() > needle.len() {
            buf.remove(0);
        }
        if buf.len() == needle.len() && *buf == *needle {
            return true;
        }
    }
    false
}

pub fn collect_code_blocks(state: &mut AppState) {
    state.code_blocks.clear();

    if !state.stream_buffer.is_empty() {
        state
            .code_blocks
            .extend(extract_code_blocks_from_text(&state.stream_buffer));
    }

    for msg in state.messages.iter() {
        state
            .code_blocks
            .extend(extract_code_blocks_from_text(msg.content.as_str()));
    }
}

pub(crate) fn extract_last_code_block<'a>(
    messages: &[&'a ChatMessage],
    stream_buffer: &'a str,
) -> Option<(String, String)> {
    if !stream_buffer.is_empty() {
        if let Some(result) = extract_code_blocks_from_text(stream_buffer).pop() {
            return Some(result);
        }
    }

    for msg in messages.iter().rev() {
        if let Some(result) = extract_code_blocks_from_text(msg.content.as_str()).pop() {
            return Some(result);
        }
    }
    None
}

fn extract_code_blocks_from_text(text: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut in_block = false;
    let mut current_lang = String::new();
    let mut current_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(stripped) = trimmed.strip_prefix("```") {
            if in_block {
                let l = if current_lang.is_empty() {
                    "text".to_string()
                } else {
                    current_lang.clone()
                };
                results.push((l, current_lines.join("\n")));
                in_block = false;
                current_lines.clear();
            } else {
                in_block = true;
                current_lang = stripped.to_string();
                current_lines.clear();
            }
        } else if in_block {
            current_lines.push(line.to_string());
        }
    }

    if in_block && !current_lines.is_empty() {
        let l = if current_lang.is_empty() {
            "text".to_string()
        } else {
            current_lang
        };
        results.push((l, current_lines.join("\n")));
    }

    results
}

/// Shell-escape user-provided args to prevent command injection via `{args}`.
fn shell_escape(args: &str) -> String {
    if cfg!(target_os = "windows") {
        let mut s = String::with_capacity(args.len() * 2);
        for ch in args.chars() {
            match ch {
                '%' | '^' | '&' | '<' | '>' | '|' | '"' | '(' | ')' | '!' => {
                    s.push('^');
                    s.push(ch);
                }
                _ => s.push(ch),
            }
        }
        s
    } else {
        let escaped = args.replace('\'', "'\\''");
        format!("'{}'", escaped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ChatMessage, Content};

    fn make_msg(role: &str, text: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: Content::text(text),
            cache_control: None,
        }
    }

    #[test]
    fn summarize_empty() {
        let summary = summarize_messages(&[]);
        assert!(summary.contains("共 0 条消息"));
    }

    #[test]
    fn summarize_user_requests() {
        let msgs = vec![
            make_msg("user", "帮我写一个排序算法"),
            make_msg("assistant", "好的，这是快速排序"),
        ];
        let summary = summarize_messages(&msgs);
        assert!(summary.contains("共 2 条消息"));
        assert!(summary.contains("1 轮对话"));
        assert!(summary.contains("帮我写一个排序算法"));
    }

    #[test]
    fn summarize_extracts_code_langs() {
        let msgs = vec![
            make_msg("user", "写 Rust 代码"),
            make_msg("assistant", "```rust\nfn main() {}\n```"),
        ];
        let summary = summarize_messages(&msgs);
        assert!(summary.contains("rust"));
    }

    #[test]
    fn summarize_truncates_long_request() {
        let long = "a".repeat(200);
        let msgs = vec![make_msg("user", &long)];
        let summary = summarize_messages(&msgs);
        // Preview should show 80 chars + "..."
        assert!(summary.contains(&"a".repeat(80)));
        assert!(summary.contains("..."));
    }

    #[test]
    fn summarize_max_10_requests() {
        let msgs: Vec<ChatMessage> = (0..20)
            .map(|i| make_msg("user", &format!("request {}", i)))
            .collect();
        let summary = summarize_messages(&msgs);
        assert!(summary.contains("等 20 个请求"));
    }

    #[test]
    fn summarize_skips_system_messages() {
        let msgs = vec![
            make_msg("system", "you are helpful"),
            make_msg("user", "hello"),
        ];
        let summary = summarize_messages(&msgs);
        assert_eq!(summary.matches("轮对话").count(), 1);
        assert!(summary.contains("1 轮对话"));
    }

    #[test]
    fn contains_ignore_case_basic() {
        assert!(contains_ignore_case("Hello World", "hello"));
        assert!(contains_ignore_case("HELLO", "hello"));
        assert!(contains_ignore_case("HeLLo WoRlD", "world"));
        assert!(!contains_ignore_case("Hello", "xyz"));
    }

    #[test]
    fn contains_ignore_case_empty() {
        assert!(contains_ignore_case("anything", ""));
        assert!(!contains_ignore_case("", "something"));
    }

    #[test]
    fn contains_ignore_case_multibyte() {
        assert!(contains_ignore_case("你好世界", "好世"));
        assert!(!contains_ignore_case("你好世界", "好界")); // not adjacent
        assert!(!contains_ignore_case("你好世界", "abc"));
    }

    #[test]
    fn contains_ignore_case_at_end() {
        assert!(contains_ignore_case("abcdef", "def"));
        assert!(!contains_ignore_case("abcdef", "deg"));
    }
}
