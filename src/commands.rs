use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, oneshot};

use crate::api::{ChatMessage, Content, MiMoClient, StreamResult};
use crate::app::{AppState, ConfirmAction, ConfirmState};
use crate::file_ops;
use crate::prompt;
use crate::util::{get_cwd, now_secs};

pub const BUILTIN_COMMANDS: &[&str] = &[
    "read", "write", "edit", "clear", "export", "model", "provider", "skills", "addskill",
    "rmskill", "theme", "errors", "help",
];

const MAX_MESSAGES: usize = 200;

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
                    "model" => "切换模型",
                    "provider" => "切换 API 提供商",
                    "skills" => "查看技能列表",
                    "addskill" => "添加技能",
                    "rmskill" => "删除技能",
                    "theme" => "切换主题 (tokyo-night/nord/catppuccin)",
                    "errors" => "查看错误历史",
                    "help" => "显示此帮助",
                    _ => continue,
                };
                text.push_str(&format!("  /{:<16}{}\n", c, desc));
            }
            if !state.skills.is_empty() {
                text.push_str("\n技能:\n");
                let mut names: Vec<&String> = state.skills.keys().collect();
                names.sort();
                for name in &names {
                    text.push_str(&format!("  /{}\n", name));
                }
            }
            text.push_str("\n支持参数: /skill_name args");
            push_assistant_msg(state, text);
        }
        "read" => handle_read_command(state, args, client, token_tx),
        "write" => handle_write_command(state, args, client, token_tx),
        "edit" => handle_edit_command(state, args, client, token_tx),
        "clear" => {
            if !state.messages.is_empty() {
                state.pending_confirm = Some(ConfirmState {
                    action: ConfirmAction::ClearChat,
                    detail: format!("清空 {} 条对话（不可恢复）", state.messages.len()),
                });
            }
        }
        "export" => handle_export_command(state, args),
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
        "skills" => {
            let mut text = format!("技能列表 ({} 个):\n", state.skills.len());
            let mut names: Vec<(&String, &String)> = state.skills.iter().collect();
            names.sort_by_key(|(k, _)| (*k).clone());
            for (name, cmd) in &names {
                text.push_str(&format!("  /{} → {}\n", name, cmd));
            }
            text.push_str("\n/addskill <name> <cmd>  添加\n/rmskill <name>         删除");
            push_assistant_msg(state, text);
        }
        "addskill" => {
            let parts: Vec<&str> = args.splitn(2, ' ').collect();
            if parts.len() < 2 {
                state.error_message = Some("用法: /addskill <name> <command>".into());
            } else {
                let name = parts[0].to_string();
                let cmd = parts[1].to_string();
                state.skills.insert(name.clone(), cmd.clone());
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
            if args.is_empty() {
                state.error_message = Some("用法: /model <model_name>".into());
            } else {
                state.config.model = args.to_string();
                client.set_model(args.to_string());
                state.config.save().ok();
                push_assistant_msg(state, format!("✓ 模型已切换为 {}", args));
            }
        }
        "provider" => handle_provider_command(state, client, args),
        "theme" => {
            if args.is_empty() {
                let mut list = String::from("可用主题:\n");
                for name in crate::ui::theme_names() {
                    let current = if *name == state.config.theme { " (当前)" } else { "" };
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

pub fn push_assistant_msg(state: &mut AppState, text: String) {
    state.messages.push(ChatMessage {
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
            list.push_str(&format!("  {} — {} ({})\n", p.name, p.model, p.base_url));
        }
        list.push_str("\n用法: /provider <name>");
        push_assistant_msg(state, list);
    } else if let Some(preset) = crate::config::Config::find_preset(args) {
        let api_key = state.config.api_key.clone();
        state.config.provider = preset.name.to_string();
        state.config.base_url = preset.base_url.to_string();
        state.config.model = preset.model.to_string();
        state.config.auth_type = preset.auth_type.to_string();
        state.config.api_format = preset.api_format.to_string();
        client.update_for_provider(preset, api_key);
        state.system_prompt_text =
            prompt::build_system_prompt_text(&state.project_tree, &state.config.provider);
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
                preset.name, preset.model, preset.base_url, state.config.model
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
    let cmd_tpl = match state.skills.get(skill_name).cloned() {
        Some(t) => t,
        None => {
            state.error_message = Some(format!("未知技能: /{}", skill_name));
            return;
        }
    };

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
    let messages_clone = state.messages.clone();
    let system_text = state.system_prompt_text.clone();
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
                let system_content = prompt::build_system_content(&system_text);

                tokio::select! {
                    r = client.send_message_stream(&system_content, &msgs, tx.clone()) => {
                        if let Err(e) = r {
                            let _ = tx.send(StreamResult::Error(e.to_string())).await;
                        }
                    }
                    _ = cancel_rx => {}
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

// ── 文件操作命令 ──

fn handle_read_command(
    state: &mut AppState,
    args: &str,
    client: &Arc<MiMoClient>,
    token_tx: &mpsc::Sender<StreamResult>,
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

    let (path_str, line_range) = if let Some(idx) = args.rfind(':') {
        let before = &args[..idx];
        let after = &args[idx + 1..];
        if before.contains('/') || before.contains('\\') {
            (args.to_string(), None)
        } else if let Some(dash_idx) = after.find('-') {
            let start_str = &after[..dash_idx];
            let end_str = &after[dash_idx + 1..];
            let start: usize = match start_str.parse() {
                Ok(v) if v > 0 => v,
                _ => {
                    state.error_message = Some(format!("/read: 无效起始行号 '{}'", start_str));
                    return;
                }
            };
            let end: usize = match end_str.parse() {
                Ok(v) if v >= start => v,
                _ => {
                    state.error_message = Some(format!("/read: 无效结束行号 '{}'", end_str));
                    return;
                }
            };
            (before.to_string(), Some((start, end)))
        } else if let Ok(line_num) = after.parse::<usize>() {
            if line_num == 0 {
                state.error_message = Some("/read: 行号必须大于0".to_string());
                return;
            }
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
    _token_tx: &mpsc::Sender<StreamResult>,
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
    _token_tx: &mpsc::Sender<StreamResult>,
) {
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

fn handle_export_command(state: &mut AppState, args: &str) {
    if state.messages.is_empty() {
        state.error_message = Some("对话为空，无法导出".to_string());
        return;
    }
    let path = if args.is_empty() {
        format!("{}.md", state.session.name)
    } else {
        args.to_string()
    };
    let mut md = String::new();
    md.push_str(&format!("# MiMo-OPT 会话: {}\n", state.session.name));
    md.push_str(&format!(
        "> 模型: {} | Token: ↓{} ↑{} | ￥{:.4}\n\n",
        state.config.model, state.total_input_tokens, state.total_output_tokens, state.total_cost,
    ));
    md.push_str("---\n\n");
    for msg in &state.messages {
        match msg.role.as_str() {
            "user" => {
                md.push_str("## User\n\n");
                md.push_str(msg.content.as_str());
                md.push_str("\n\n");
            }
            _ => {
                md.push_str(&format!("## {}\n\n", state.config.model));
                md.push_str(msg.content.as_str());
                md.push_str("\n\n");
            }
        }
    }
    match std::fs::write(&path, &md) {
        Ok(()) => {
            state.messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: Content::text(format!(
                    "✓ 已导出到 {} ({} 条消息, {} B)",
                    path,
                    state.messages.len(),
                    md.len()
                )),
                cache_control: None,
            });
        }
        Err(e) => {
            state.error_message = Some(format!("/export: {}", e));
        }
    }
}

// ── 确认对话框处理 ──

fn resolve_path(state: &mut AppState, path: &str) -> Option<PathBuf> {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            state.error_message = Some(format!("获取工作目录失败: {}", e));
            return None;
        }
    };
    match file_ops::validate_path(path, &cwd) {
        Ok(p) => Some(p),
        Err(e) => {
            state.error_message = Some(e);
            None
        }
    }
}

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
            let full_path = match resolve_path(state, &path) {
                Some(p) => p,
                None => return,
            };
            match file_ops::write_file(&full_path, &code) {
                Ok(msg) => {
                    push_assistant_msg(
                        state,
                        format!("✓ {} (from {} code block)", msg, lang),
                    );
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
            let full_path = match resolve_path(state, &path) {
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
            state.messages.push(ChatMessage {
                role: "user".to_string(),
                content: Content::text(message),
                cache_control: None,
            });
            maybe_truncate_messages(state);
            spawn_stream_request(state, client, token_tx, None);
        }
        ConfirmAction::ClearChat => {
            let count = state.messages.len();
            state.messages.clear();
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

pub fn show_confirm_detail(state: &mut AppState) {
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

    let mut msgs = state.messages.clone();
    if let Some(msg_text) = extra_user_msg {
        msgs.push(ChatMessage {
            role: "user".to_string(),
            content: Content::text(msg_text),
            cache_control: None,
        });
    }
    if msgs.len() > MAX_MESSAGES {
        let excess = msgs.len() - 150;
        if excess > 0 {
            msgs.drain(0..excess);
        }
    }

    prompt::inject_date_if_needed(&mut msgs);
    prompt::apply_cache_breakpoints(&mut msgs);

    let system_text = state.system_prompt_text.clone();
    let system_content = prompt::build_system_content(&system_text);
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

/// 检查消息数是否超限，自动截断最早的消息对（保留最近 150 条）
pub fn maybe_truncate_messages(state: &mut AppState) {
    if state.messages.len() > MAX_MESSAGES {
        let excess = state.messages.len() - 150;
        if excess > 0 {
            state.messages.drain(0..excess);
            if let Some(first) = state.messages.first_mut() {
                if !first.content.as_str().contains("[Current date:") {
                    let today = prompt::chrono_date();
                    first
                        .content
                        .as_mut_str()
                        .push_str(&format!("\n\n[Current date: {}]", today));
                }
            }
        }
    }
}

/// 保存当前会话（消息 + 时间戳 + token 统计）
pub fn save_session(state: &mut AppState) {
    state.session.messages = state.messages.clone();
    state.session.updated_at = now_secs();
    state.session.total_input_tokens = state.total_input_tokens;
    state.session.total_output_tokens = state.total_output_tokens;
    state.session.total_cache_creation_tokens = state.total_cache_creation_tokens;
    state.session.total_cache_read_tokens = state.total_cache_read_tokens;
    state.session.total_cost = state.total_cost;
    let _ = state.session.save();
}

/// 推送一条"已取消"助手消息
pub fn push_cancelled_message(state: &mut AppState) {
    push_assistant_msg(state, "已取消".to_string());
}

// ── 搜索 + 代码块 ──

pub fn update_hint_lines(state: &mut AppState) {
    if let Some(rest) = state.input.strip_prefix('/') {
        if rest.contains(' ') {
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

pub fn do_search(state: &mut AppState) {
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

pub fn collect_code_blocks(state: &mut AppState) {
    state.code_blocks.clear();

    if !state.stream_buffer.is_empty() {
        state
            .code_blocks
            .extend(extract_code_blocks_from_text(&state.stream_buffer));
    }

    for msg in &state.messages {
        state
            .code_blocks
            .extend(extract_code_blocks_from_text(msg.content.as_str()));
    }
}

fn extract_last_code_block<'a>(
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
pub fn shell_escape(args: &str) -> String {
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
