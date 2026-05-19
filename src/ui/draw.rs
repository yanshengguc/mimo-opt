use std::sync::OnceLock;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap, Widget},
    Frame,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{
        Color as SynColor, FontStyle, StyleModifier, Theme, ThemeItem,
        ThemeSet, ScopeSelectors,
    },
    parsing::SyntaxSet,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{AppState, ConfirmAction};
use super::Theme as AppTheme;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn syntax_set() -> &'static SyntaxSet {
    static PS: OnceLock<SyntaxSet> = OnceLock::new();
    PS.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syn_color(hex: u32) -> SynColor {
    SynColor {
        r: ((hex >> 16) & 0xFF) as u8,
        g: ((hex >> 8) & 0xFF) as u8,
        b: (hex & 0xFF) as u8,
        a: 0xFF,
    }
}

fn scope_item(selector: &str, hex: u32) -> ThemeItem {
    ThemeItem {
        scope: selector.parse::<ScopeSelectors>().unwrap(),
        style: StyleModifier {
            foreground: Some(syn_color(hex)),
            background: None,
            font_style: Some(FontStyle::empty()),
        },
    }
}

fn build_theme() -> Theme {
    let ts = ThemeSet::load_defaults();
    let mut theme = ts.themes["base16-ocean.dark"].clone();
    theme.name = Some("tokyo-night".into());
    theme.scopes = vec![
        scope_item("comment", 0x565f89),
        scope_item("keyword", 0xbb9af7),
        scope_item("keyword.control", 0xbb9af7),
        scope_item("storage", 0xbb9af7),
        scope_item("constant", 0xff9e64),
        scope_item("constant.numeric", 0xff9e64),
        scope_item("constant.language", 0xff9e64),
        scope_item("string", 0x9ece6a),
        scope_item("string.regexp", 0xff9e64),
        scope_item("entity.name.function", 0x7aa2f7),
        scope_item("entity.name.type", 0x2ac3de),
        scope_item("entity.name.class", 0x2ac3de),
        scope_item("variable", 0xc0caf5),
        scope_item("variable.parameter", 0xc0caf5),
        scope_item("support.function", 0x7aa2f7),
        scope_item("support.type", 0x2ac3de),
        scope_item("punctuation", 0x9aa5ce),
        scope_item("operator", 0x89ddff),
        scope_item("tag", 0xbb9af7),
        scope_item("attribute.name", 0x7aa2f7),
        scope_item("attribute.value", 0x9ece6a),
        scope_item("meta.preprocessor", 0xff9e64),
    ];
    theme
}

fn theme() -> &'static Theme {
    static T: OnceLock<Theme> = OnceLock::new();
    T.get_or_init(build_theme)
}

fn is_code_fence(line: &str) -> Option<Option<String>> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("```") {
        return None;
    }
    let after = &trimmed[3..];
    if after.is_empty() {
        return Some(None);
    }
    if after.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
        return Some(Some(after.to_string()));
    }
    None
}

fn highlight_line<'a>(
    h: &mut HighlightLines,
    ps: &'static SyntaxSet,
    line: &'a str,
    spans: &mut Vec<Span<'a>>,
) {
    match h.highlight_line(line, ps) {
        Ok(regions) => {
            for (style, text) in regions {
                let fg = style.foreground;
                spans.push(Span::styled(
                    text.to_string(),
                    Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b)),
                ));
            }
        }
        Err(_) => {
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(AppTheme::MIMO_MSG),
            ));
        }
    }
}

pub fn draw(f: &mut Frame, state: &AppState) {
    let hint_len = if state.hint_lines.is_empty() { 0u16 } else { 1 };
    let search_len = if state.search_active { 1u16 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),             // 标题栏
            Constraint::Min(8),                // 对话区
            Constraint::Length(hint_len),      // 技能提示
            Constraint::Length(search_len),    // 搜索栏
            Constraint::Length(3),             // 输入框
            Constraint::Length(3),             // 状态栏
        ])
        .split(f.area());

    draw_title_bar(f, chunks[0], state);
    draw_chat_area(f, chunks[1], state);
    draw_hint_area(f, chunks[2], state);
    draw_search_bar(f, chunks[3], state);
    draw_input_area(f, chunks[4], state);
    draw_status_bar(f, chunks[5], state);

    // 确认对话框（覆盖在对话区上方）
    if let Some(ref confirm) = state.pending_confirm {
        draw_confirm_modal(f, confirm);
    }
}

fn draw_title_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let model_text = format!(" ⚡ {} ", state.config.model);

    // 会话名（截断过长名称）
    let session_name = &state.session.name;
    let short_name = if session_name.len() > 20 {
        format!("{}...", &session_name[..17])
    } else {
        session_name.clone()
    };
    let title = format!(" MiMo-OPT [{}] ", short_name);

    // API 状态显示
    let (api_text, api_color) = match state.api_ok {
        Some(true) => (" ⊛ API OK ".to_string(), AppTheme::SUCCESS),
        Some(false) => {
            let detail = state.api_error_detail.as_deref().unwrap_or("error");
            // 截断过长的错误信息
            let short = if detail.len() > 20 {
                format!("{}...", &detail[..17])
            } else {
                detail.to_string()
            };
            (format!(" ⊛ {} ", short), AppTheme::ERROR)
        }
        None => (" ⊛ ... ".to_string(), AppTheme::STATUS_LABEL),
    };

    // 先计算宽度（用 unicode-width 而非 .len()）
    let total_width = area.width as usize;
    let left_len = UnicodeWidthStr::width(title.as_str());
    let middle_len = UnicodeWidthStr::width(api_text.as_str());
    let right_len = UnicodeWidthStr::width(model_text.as_str());
    let used = left_len + middle_len + right_len + 2;

    let left = Span::styled(title, Style::default().fg(AppTheme::TITLE).add_modifier(Modifier::BOLD));
    let middle = Span::styled(api_text, Style::default().fg(api_color));
    let right = Span::styled(model_text, Style::default().fg(AppTheme::MODEL_TAG));

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
        .border_style(Style::default().fg(AppTheme::BORDER_DIM))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_chat_area(f: &mut Frame, area: Rect, state: &AppState) {
    let ps = syntax_set();
    let syn_theme = theme();

    let mut lines: Vec<Line> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut msg_line_ranges: Vec<(usize, usize)> = Vec::new(); // (start_line, msg_idx)

    for (msg_idx, msg) in state.messages.iter().enumerate() {
        let start_line = lines.len();

        if !lines.is_empty() {
            lines.push(Line::raw(""));
        }

        let default_color = match msg.role.as_str() {
            "user" => AppTheme::USER_MSG,
            _ => AppTheme::MIMO_MSG,
        };

        let is_search_match = state.search_active
            && !state.search_matches.is_empty()
            && state.search_matches.get(state.search_match_idx) == Some(&msg_idx);

        for line in msg.content.as_str().lines() {
            if let Some(lang_opt) = is_code_fence(line) {
                if !in_code_block {
                    in_code_block = true;
                    code_lang = lang_opt;
                    if let Some(ref lang) = code_lang {
                        lines.push(Line::from(Span::styled(
                            format!("   ┌─ {} ──", lang),
                            Style::default().fg(AppTheme::CODE_BORDER),
                        )));
                    } else {
                        lines.push(Line::from(Span::styled(
                            "   ┌──────────",
                            Style::default().fg(AppTheme::CODE_BORDER),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "   └──────────",
                        Style::default().fg(AppTheme::CODE_BORDER),
                    )));
                    in_code_block = false;
                    code_lang = None;
                }
            } else if in_code_block {
                let syntax = if let Some(ref lang) = code_lang {
                    ps.find_syntax_by_token(lang).unwrap_or_else(|| ps.find_syntax_plain_text())
                } else {
                    ps.find_syntax_plain_text()
                };
                let mut h = HighlightLines::new(syntax, syn_theme);
                let mut code_spans = vec![
                    Span::styled("  │ ", Style::default().fg(AppTheme::CODE_BORDER)),
                ];
                highlight_line(&mut h, ps, line, &mut code_spans);
                lines.push(Line::from(code_spans));
            } else {
                let style = if is_search_match {
                    Style::default().fg(default_color).bg(Color::Rgb(59, 66, 97))
                } else {
                    Style::default().fg(default_color)
                };
                lines.push(Line::from(Span::styled(
                    format!("   {}", line),
                    style,
                )));
            }
        }

        if is_search_match && start_line < lines.len() {
            msg_line_ranges.push((start_line, msg_idx));
        }
    }

    if in_code_block {
        lines.push(Line::from(Span::styled(
            "   └──────────",
            Style::default().fg(AppTheme::CODE_BORDER),
        )));
    }

    // 流式生成中
    if state.generating {
        if !lines.is_empty() {
            lines.push(Line::raw(""));
        }

        if state.stream_buffer.is_empty() {
            let spinner_idx = state.spinner_tick % SPINNER.len();
            lines.push(Line::from(Span::styled(
                format!("   {} thinking...", SPINNER[spinner_idx]),
                Style::default().fg(AppTheme::SPINNER),
            )));
        } else {
            let mut stream_code_lang = if in_code_block { code_lang.clone() } else { None };

            for line in state.stream_buffer.lines() {
                if stream_code_lang.is_some() {
                    if is_code_fence(line).is_some() {
                        lines.push(Line::from(Span::styled(
                            "   └──────────",
                            Style::default().fg(AppTheme::CODE_BORDER),
                        )));
                        stream_code_lang = None;
                        continue;
                    }
                    let syntax = if let Some(ref lang) = stream_code_lang {
                        ps.find_syntax_by_token(lang).unwrap_or_else(|| ps.find_syntax_plain_text())
                    } else {
                        ps.find_syntax_plain_text()
                    };
                    let mut h = HighlightLines::new(syntax, syn_theme);
                    let mut code_spans = vec![
                        Span::styled("  │ ", Style::default().fg(AppTheme::CODE_BORDER)),
                    ];
                    highlight_line(&mut h, ps, line, &mut code_spans);
                    lines.push(Line::from(code_spans));
                } else if let Some(lang_opt) = is_code_fence(line) {
                    stream_code_lang = lang_opt.clone();
                    if let Some(ref lang) = lang_opt {
                        lines.push(Line::from(Span::styled(
                            format!("   ┌─ {} ──", lang),
                            Style::default().fg(AppTheme::CODE_BORDER),
                        )));
                    } else {
                        lines.push(Line::from(Span::styled(
                            "   ┌──────────",
                            Style::default().fg(AppTheme::CODE_BORDER),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", line),
                        Style::default().fg(AppTheme::MIMO_MSG),
                    )));
                }
            }
            if stream_code_lang.is_some() {
                lines.push(Line::from(Span::styled(
                    "  │ ▎",
                    Style::default().fg(AppTheme::MODEL_TAG),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "   ▎",
                    Style::default().fg(AppTheme::MODEL_TAG),
                )));
            }
        }
    }

    // 滚动（支持手动偏移）
    let total_lines = lines.len();
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll = if total_lines > visible_height {
        let max_scroll = total_lines - visible_height;
        max_scroll.saturating_sub(state.chat_scroll)
    } else {
        0
    };

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(AppTheme::CODE_BORDER));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn draw_search_bar(f: &mut Frame, area: Rect, state: &AppState) {
    if !state.search_active || area.height == 0 {
        return;
    }
    let match_info = if state.search_matches.is_empty() {
        "no matches".to_string()
    } else {
        format!("{}/{}", state.search_match_idx + 1, state.search_matches.len())
    };
    let text = format!("  / {}  {}", state.search_query, match_info);
    let p = Paragraph::new(Span::styled(text, Style::default().fg(AppTheme::TITLE)));
    f.render_widget(p, area);
}

fn draw_hint_area(f: &mut Frame, area: Rect, state: &AppState) {
    if area.height == 0 || state.hint_lines.is_empty() {
        return;
    }
    let hint_text = state.hint_lines.join("  ");
    let p = Paragraph::new(Span::styled(
        format!("  {}", hint_text),
        Style::default().fg(AppTheme::STATUS_LABEL),
    ));
    f.render_widget(p, area);
}

fn draw_input_area(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans = vec![Span::styled(
        "  › ",
        Style::default().fg(AppTheme::INPUT_PROMPT),
    )];

    if state.search_active {
        spans.push(Span::styled(
            "Searching... (Esc to close)",
            Style::default().fg(AppTheme::STATUS_LABEL),
        ));
    } else if state.generating {
        let spinner_idx = state.spinner_tick % SPINNER.len();
        spans.push(Span::styled(
            format!("{} generating...", SPINNER[spinner_idx]),
            Style::default().fg(AppTheme::SPINNER),
        ));
    } else {
        // 显示输入文本，光标在 cursor_pos 位置
        let before = &state.input[..state.cursor_pos];
        let after = &state.input[state.cursor_pos..];

        spans.push(Span::styled(before, Style::default().fg(AppTheme::USER_MSG)));
        spans.push(Span::styled(
            "▎",
            Style::default().fg(AppTheme::INPUT_PROMPT).add_modifier(Modifier::SLOW_BLINK),
        ));
        if !after.is_empty() {
            spans.push(Span::styled(after, Style::default().fg(AppTheme::USER_MSG)));
        }
    }

    // 右侧快捷键提示
    let hint = if state.search_active { "Enter ↵  Esc ✕" }
        else if state.generating { "Esc" }
        else { "Ctrl+Enter ↵" };
    let hint_len = UnicodeWidthStr::width(hint);
    let total_width = area.width as usize;
    let content_len: usize = spans.iter().map(|s| UnicodeWidthStr::width(s.content.as_ref())).sum();
    let padding = if total_width > content_len + hint_len + 4 {
        total_width - content_len - hint_len - 4
    } else {
        1
    };
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(hint, Style::default().fg(AppTheme::STATUS_LABEL)));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AppTheme::SEPARATOR))
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
    let effective_input = state.total_input_tokens + state.total_cache_read_tokens;
    let cache_text = if effective_input > 0 {
        let hit_rate = state.total_cache_read_tokens as f64 / effective_input as f64 * 100.0;
        format!(" ♻ {:.0}% ", hit_rate)
    } else {
        String::new()
    };

    let mut spans = vec![
        Span::styled(token_text, Style::default().fg(AppTheme::STATUS_DATA)),
        Span::styled(io_text, Style::default().fg(AppTheme::STATUS_LABEL)),
        Span::styled("  ", Style::default()),
        Span::styled(cost_text, Style::default().fg(AppTheme::MODEL_TAG)),
    ];

    if !cache_text.is_empty() {
        spans.push(Span::styled(
            cache_text,
            Style::default().fg(AppTheme::SUCCESS),
        ));
    }

    // 滚动指示器
    if state.chat_scroll > 0 {
        spans.push(Span::styled(
            format!(" [↑{}]", state.chat_scroll),
            Style::default().fg(AppTheme::STATUS_LABEL),
        ));
    }

    // 复制状态
    if let Some(ref copy_msg) = state.copy_status {
        spans.push(Span::styled(
            format!(" ✓ {}", copy_msg),
            Style::default().fg(AppTheme::SUCCESS),
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
        spans.push(Span::styled(err_text, Style::default().fg(AppTheme::ERROR)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AppTheme::CODE_BORDER))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_confirm_modal(f: &mut Frame, confirm: &crate::app::ConfirmState) {
    let area = f.area();

    // 计算居中弹窗（宽度 max 60，高度 max 8）
    let modal_w = 60u16.min(area.width.saturating_sub(4));
    let modal_h = 8u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(modal_w)) / 2;
    let y = (area.height.saturating_sub(modal_h)) / 2;
    let modal_area = Rect::new(x, y, modal_w, modal_h);

    // 清除背景
    ratatui::widgets::Clear.render(modal_area, f.buffer_mut());

    let mut lines: Vec<Line> = Vec::new();

    // 标题
    let title = match &confirm.action {
        ConfirmAction::WriteFile { .. } => " ⚠ 写入确认 ",
        ConfirmAction::EditFile { .. } => " ⚠ 编辑确认 ",
        ConfirmAction::ClearChat => " ⚠ 清空确认 ",
        ConfirmAction::Quit => " ⚠ 退出确认 ",
    };
    lines.push(Line::from(Span::styled(
        title,
        Style::default().fg(AppTheme::ERROR).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    // 详情
    for line in confirm.detail.lines() {
        lines.push(Line::from(Span::styled(
            format!(" {}", line),
            Style::default().fg(AppTheme::USER_MSG),
        )));
    }

    lines.push(Line::raw(""));

    // 操作提示
    lines.push(Line::from(vec![
        Span::styled(" y", Style::default().fg(AppTheme::SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled(" 确认  ", Style::default().fg(AppTheme::USER_MSG)),
        Span::styled("n", Style::default().fg(AppTheme::ERROR).add_modifier(Modifier::BOLD)),
        Span::styled(" 取消  ", Style::default().fg(AppTheme::USER_MSG)),
        Span::styled("d", Style::default().fg(AppTheme::MODEL_TAG).add_modifier(Modifier::BOLD)),
        Span::styled(" 详情", Style::default().fg(AppTheme::USER_MSG)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AppTheme::ERROR))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, modal_area);
}
