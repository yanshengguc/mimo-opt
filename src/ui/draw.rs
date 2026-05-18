use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::AppState;
use super::Theme;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题栏
            Constraint::Min(10),   // 对话区
            Constraint::Length(3), // 输入框
            Constraint::Length(3), // 状态栏
        ])
        .split(f.area());

    draw_title_bar(f, chunks[0], state);
    draw_chat_area(f, chunks[1], state);
    draw_input_area(f, chunks[2], state);
    draw_status_bar(f, chunks[3], state);
}

fn draw_title_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let model_text = format!(" ⚡ {} ", state.config.model);
    let title = " MiMo-OPT ";

    // API 状态显示
    let (api_text, api_color) = match state.api_ok {
        Some(true) => (" ⊛ API OK ".to_string(), Theme::SUCCESS),
        Some(false) => {
            let detail = state.api_error_detail.as_deref().unwrap_or("error");
            // 截断过长的错误信息
            let short = if detail.len() > 20 {
                format!("{}...", &detail[..17])
            } else {
                detail.to_string()
            };
            (format!(" ⊛ {} ", short), Theme::ERROR)
        }
        None => (" ⊛ ... ".to_string(), Theme::STATUS_LABEL),
    };

    // 先计算宽度（用 unicode-width 而非 .len()）
    let total_width = area.width as usize;
    let left_len = UnicodeWidthStr::width(title);
    let middle_len = UnicodeWidthStr::width(api_text.as_str());
    let right_len = UnicodeWidthStr::width(model_text.as_str());
    let used = left_len + middle_len + right_len + 2;

    let left = Span::styled(title, Style::default().fg(Theme::TITLE).add_modifier(Modifier::BOLD));
    let middle = Span::styled(api_text, Style::default().fg(api_color));
    let right = Span::styled(model_text, Style::default().fg(Theme::MODEL_TAG));

    let mut spans = vec![left];

    if total_width > used {
        let pad_mid = (total_width - used) / 2;
        let pad_right = total_width - used - pad_mid;
        spans.push(Span::raw(" ".repeat(pad_mid)));
        spans.push(middle);
        spans.push(Span::raw(" ".repeat(pad_right)));
    } else {
        let pad = total_width.saturating_sub(left_len + right_len + 2);
        spans.push(Span::raw(" ".repeat(pad)));
    }

    spans.push(right);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::BORDER_DIM))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_chat_area(f: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &state.messages {
        if !lines.is_empty() {
            lines.push(Line::raw(""));
        }

        let color = match msg.role.as_str() {
            "user" => Theme::USER_MSG,
            _ => Theme::MIMO_MSG,
        };

        for line in msg.content.lines() {
            lines.push(Line::from(Span::styled(
                format!("   {}", line),
                Style::default().fg(color),
                    )));
        }
    }

    // 流式生成中
    if state.generating {
        if !lines.is_empty() {
            lines.push(Line::raw(""));
        }

        if state.stream_buffer.is_empty() {
            // 还没收到 token，显示等待提示
            let spinner_idx = state.spinner_tick % SPINNER.len();
            lines.push(Line::from(Span::styled(
                format!("   {} thinking...", SPINNER[spinner_idx]),
                Style::default().fg(Theme::SPINNER),
            )));
        } else {
            for line in state.stream_buffer.lines() {
                lines.push(Line::from(Span::styled(
                    format!("   {}", line),
                    Style::default().fg(Theme::MIMO_MSG),
                )));
            }
            // 光标
            lines.push(Line::from(Span::styled(
                "   ▎",
                Style::default().fg(Theme::MODEL_TAG),
            )));
        }
    }

    // 滚动
    let total_lines = lines.len();
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll = if total_lines > visible_height {
        total_lines - visible_height
    } else {
        0
    };

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Theme::CODE_BORDER));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn draw_input_area(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans = vec![Span::styled(
        "  › ",
        Style::default().fg(Theme::INPUT_PROMPT),
    )];

    if state.generating {
        let spinner_idx = state.spinner_tick % SPINNER.len();
        spans.push(Span::styled(
            format!("{} generating...", SPINNER[spinner_idx]),
            Style::default().fg(Theme::SPINNER),
        ));
    } else {
        // 显示输入文本，光标在 cursor_pos 位置
        let before = &state.input[..state.cursor_pos];
        let after = &state.input[state.cursor_pos..];

        spans.push(Span::styled(before, Style::default().fg(Theme::USER_MSG)));
        spans.push(Span::styled(
            "▎",
            Style::default().fg(Theme::INPUT_PROMPT).add_modifier(Modifier::SLOW_BLINK),
        ));
        if !after.is_empty() {
            spans.push(Span::styled(after, Style::default().fg(Theme::USER_MSG)));
        }
    }

    // 右侧快捷键提示
    let hint = if state.generating { "Esc" } else { "Ctrl+Enter ↵" };
    let hint_len = UnicodeWidthStr::width(hint);
    let total_width = area.width as usize;
    let content_len: usize = spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
    let padding = if total_width > content_len + hint_len + 4 {
        total_width - content_len - hint_len - 4
    } else {
        1
    };
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(hint, Style::default().fg(Theme::STATUS_LABEL)));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::SEPARATOR))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let total_tok = state.total_input_tokens + state.total_output_tokens;
    let token_text = format!(" ◉ {:.1}K tok ", total_tok as f64 / 1000.0);
    let io_text = format!(" ↓{} ↑{} ", state.total_input_tokens, state.total_output_tokens);
    let cost_text = format!(" ￥{:.4} ", state.total_cost);

    // 缓存命中率
    let cache_total = state.total_cache_creation_tokens + state.total_cache_read_tokens;
    let cache_text = if cache_total > 0 {
        let hit_rate = state.total_cache_read_tokens as f64 / cache_total as f64 * 100.0;
        format!(" ♻ {:.0}% ", hit_rate)
    } else {
        String::new()
    };

    let mut spans = vec![
        Span::styled(token_text, Style::default().fg(Theme::STATUS_DATA)),
        Span::styled(io_text, Style::default().fg(Theme::STATUS_LABEL)),
        Span::styled("  ", Style::default()),
        Span::styled(cost_text, Style::default().fg(Theme::MODEL_TAG)),
    ];

    if !cache_text.is_empty() {
        spans.push(Span::styled(
            cache_text,
            Style::default().fg(Theme::SUCCESS),
        ));
    }

    // 错误信息
    if let Some(ref err) = state.error_message {
        let err_text = format!(" ✗ {} ", err);
        let total_width = area.width as usize;
        let content_len: usize = spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
        let padding = if total_width > content_len + UnicodeWidthStr::width(err_text.as_str()) + 2 {
            total_width - content_len - UnicodeWidthStr::width(err_text.as_str()) - 2
        } else {
            1
        };
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(err_text, Style::default().fg(Theme::ERROR)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Theme::CODE_BORDER))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
