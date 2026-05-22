use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc;

use crate::api::{ChatMessage, Content, MiMoClient, StreamResult};
use crate::app::{AppState, ConfirmAction, ConfirmState};
use crate::file_ops;
use crate::util::get_cwd;

use super::send_to_mimo;

// ── 文件操作命令 ──

pub fn handle_read_command(
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
            Arc::make_mut(&mut state.messages).push(ChatMessage {
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

pub fn handle_write_command(
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
    let code = super::extract_last_code_block(&all_messages, &state.stream_buffer);

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

pub fn handle_edit_command(
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
            let diff = compute_edit_diff(&content, old_text, new_text);
            let action = ConfirmAction::EditFile {
                path: path_str.to_string(),
                old: old_text.to_string(),
                new: new_text.to_string(),
            };
            let detail = format!("{} — 替换 {} 处\n{}", path_str, count, diff);
            state.pending_confirm = Some(ConfirmState { action, detail });
        }
        Err(e) => {
            state.error_message = Some(format!("/edit: 读取失败: {}", e));
        }
    }
}

pub fn handle_export_command(state: &mut AppState, args: &str) {
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
    for msg in state.messages.iter() {
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
            let msg_count = state.messages.len();
            let md_len = md.len();
            Arc::make_mut(&mut state.messages).push(ChatMessage {
                role: "assistant".to_string(),
                content: Content::text(format!(
                    "✓ 已导出到 {} ({} 条消息, {} B)",
                    path, msg_count, md_len
                )),
                cache_control: None,
            });
        }
        Err(e) => {
            state.error_message = Some(format!("/export: {}", e));
        }
    }
}

// ── 确认对话框辅助 ──

pub fn resolve_path(state: &mut AppState, path: &str) -> Option<PathBuf> {
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

/// Generate a diff preview for /edit: old_text → new_text in file content.
/// Returns lines prefixed with "  " (context), "- " (deleted), "+ " (added).
fn compute_edit_diff(file_content: &str, old_text: &str, new_text: &str) -> String {
    let mut result = String::new();
    for line in file_content.lines() {
        if line.contains(old_text) {
            let replaced = line.replace(old_text, new_text);
            if line != replaced {
                result.push_str(&format!("- {}\n", line));
                result.push_str(&format!("+ {}\n", replaced));
                continue;
            }
        }
        result.push_str(&format!("  {}\n", line));
    }
    result.trim_end().to_string()
}
