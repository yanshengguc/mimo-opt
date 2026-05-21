use std::sync::OnceLock;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
    Frame,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{
        Color as SynColor, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSet,
    },
    parsing::SyntaxSet,
};
use unicode_width::UnicodeWidthStr;

use super::theme::ThemeColors;
use crate::app::{AppState, ConfirmAction};

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
    if after
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Some(Some(after.to_string()));
    }
    None
}

/// 检测水平分隔线: 整行只有 ---, ***, ___, 或带空格的变体
fn is_hr_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let ch = trimmed.chars().next().unwrap();
    if ch != '-' && ch != '*' && ch != '_' {
        return false;
    }
    trimmed.chars().all(|c| c == ch || c == ' ')
}

/// 检测无序列表项: 以 "- " 或 "* " 开头
fn is_unordered_list(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("- ") {
        Some(rest)
    } else if let Some(rest) = line.strip_prefix("* ") {
        if rest.starts_with(' ') {
            None
        } else {
            Some(rest)
        }
    } else {
        None
    }
}

/// 检测有序列表项: 以 "N. " 开头
fn is_ordered_list(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    if let Some(dot_pos) = trimmed.find(". ") {
        let num_part = &trimmed[..dot_pos];
        if num_part.chars().all(|c| c.is_ascii_digit()) {
            let rest = &trimmed[dot_pos + 2..];
            return Some((indent, rest));
        }
    }
    None
}

fn highlight_line<'a>(
    h: &mut HighlightLines,
    ps: &'static SyntaxSet,
    line: &'a str,
    spans: &mut Vec<Span<'a>>,
    code_bg: Color,
) {
    match h.highlight_line(line, ps) {
        Ok(regions) => {
            for (style, text) in regions {
                let fg = style.foreground;
                spans.push(Span::styled(
                    text.to_string(),
                    Style::default()
                        .fg(Color::Rgb(fg.r, fg.g, fg.b))
                        .bg(code_bg),
                ));
            }
        }
        Err(_) => {
            spans.push(Span::styled(line.to_string(), Style::default().fg(code_bg).bg(code_bg)));
        }
    }
}

/// 渲染单行文本，解析内联 Markdown: **粗体**, *斜体*, `行内代码`, [链接](url)
fn render_markdown_line<'a>(line: &'a str, base_style: Style, t: &ThemeColors) -> Line<'a> {
    let mut spans: Vec<Span> = vec![Span::styled("   ", base_style)];
    let mut remaining = line;
    while !remaining.is_empty() {
        // 行内代码
        if let Some(rest) = remaining.strip_prefix('`') {
            if let Some(end) = rest.find('`') {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    Style::default()
                        .fg(t.model_tag)
                        .bg(t.inline_code),
                ));
                remaining = &rest[end + 1..];
                continue;
            }
        }
        // 链接 [text](url)
        if let Some(rest) = remaining.strip_prefix('[') {
            if let Some(bracket_end) = rest.find("](") {
                let link_text = &rest[..bracket_end];
                let after_bracket = &rest[bracket_end + 2..];
                if let Some(paren_end) = after_bracket.find(')') {
                    spans.push(Span::styled(
                        link_text.to_string(),
                        Style::default()
                            .fg(t.link_color)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    remaining = &after_bracket[paren_end + 1..];
                    continue;
                }
            }
        }
        // 粗体 **
        if let Some(rest) = remaining.strip_prefix("**") {
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    base_style.add_modifier(Modifier::BOLD),
                ));
                remaining = &rest[end + 2..];
                continue;
            }
        }
        // 斜体 *（但不匹配 ** 和列表标记 "- *"）
        if let Some(rest) = remaining.strip_prefix('*') {
            if !rest.starts_with('*') && !rest.starts_with(' ') {
                if let Some(end) = rest.find('*') {
                    spans.push(Span::styled(
                        rest[..end].to_string(),
                        base_style.add_modifier(Modifier::ITALIC),
                    ));
                    remaining = &rest[end + 1..];
                    continue;
                }
            }
        }
        // 普通文字：取到下一个特殊标记
        let next_special = remaining
            .find(['`', '*', '['])
            .unwrap_or(remaining.len());
        if next_special > 0 {
            spans.push(Span::styled(
                remaining[..next_special].to_string(),
                base_style,
            ));
        }
        remaining = &remaining[next_special..];
    }
    Line::from(spans)
}

pub fn draw(f: &mut Frame, state: &AppState, theme: &ThemeColors) {
    // 终端最小尺寸检查
    let area = f.area();
    if area.width < 60 || area.height < 20 {
        let warning = Line::from(Span::styled(
            " 窗口过小，请调整到 60×20 以上 ",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        ));
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.error));
        f.render_widget(
            Paragraph::new(warning)
                .block(block)
                .alignment(ratatui::layout::Alignment::Center),
            area,
        );
        return;
    }

    let hint_len = if state.hint_lines.is_empty() { 0u16 } else { 1 };
    let search_len = if state.search_active { 1u16 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(hint_len),
            Constraint::Length(search_len),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_title_bar(f, chunks[0], state, theme);
    draw_chat_area(f, chunks[1], state, theme);
    draw_hint_area(f, chunks[2], state, theme);
    draw_search_bar(f, chunks[3], state, theme);
    draw_input_area(f, chunks[4], state, theme);
    draw_status_bar(f, chunks[5], state, theme);

    if let Some(ref confirm) = state.pending_confirm {
        draw_confirm_modal(f, confirm, theme);
    }
}

fn draw_title_bar(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    let model_text = format!(" ⚡ {} ", state.config.model);

    let session_name = &state.session.name;
    let short_name = if session_name.len() > 20 {
        format!("{}...", &session_name[..17])
    } else {
        session_name.clone()
    };
    let provider_label = match state.config.provider.as_str() {
        "deepseek" => "DeepSeek",
        "openai" => "OpenAI",
        "mimo" => "MiMo",
        _ => "Custom",
    };
    let title = format!(" {}-OPT [{}] ", provider_label, short_name);

    let (api_text, api_color) = match state.api_ok {
        Some(true) => (" ⊛ API OK ".to_string(), t.success),
        Some(false) => {
            let detail = state.api_error_detail.as_deref().unwrap_or("error");
            let short = if detail.len() > 20 {
                format!("{}...", &detail[..17])
            } else {
                detail.to_string()
            };
            (format!(" ⊛ {} ", short), t.error)
        }
        None => (" ⊛ 发消息检测API ".to_string(), t.status_label),
    };

    let mut right_spans: Vec<Span> = vec![Span::styled(
        model_text,
        Style::default().fg(t.model_tag),
    )];
    if let Some((ref display, amount)) = state.balance_info {
        let bal_color = if amount < 1.0 {
            t.error
        } else if amount < 5.0 {
            Color::Rgb(224, 175, 104)
        } else {
            t.success
        };
        right_spans.push(Span::styled(
            format!(" {} ", display),
            Style::default().fg(bal_color),
        ));
    }

    let total_width = area.width as usize;
    let left_len = UnicodeWidthStr::width(title.as_str());
    let middle_len = UnicodeWidthStr::width(api_text.as_str());
    let right_len: usize = right_spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let used = left_len + middle_len + right_len + 2;

    let left = Span::styled(
        title,
        Style::default().fg(t.title).add_modifier(Modifier::BOLD),
    );
    let middle = Span::styled(api_text, Style::default().fg(api_color));

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

    spans.extend(right_spans);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_dim))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_chat_area(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    let ps = syntax_set();
    let syn_theme = theme();

    let mut lines: Vec<Line> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut msg_line_ranges: Vec<(usize, usize)> = Vec::new();

    for (msg_idx, msg) in state.messages.iter().enumerate() {
        let start_line = lines.len();

        if msg.role == "user" && msg_idx > 0 {
            let sep_width = (area.width as usize).saturating_sub(6);
            let sep: String = " ─".repeat(sep_width / 2);
            lines.push(Line::from(Span::styled(
                format!("   {}", sep),
                Style::default().fg(t.code_border),
            )));
        } else if !lines.is_empty() {
            lines.push(Line::raw(""));
        }

        let default_color = match msg.role.as_str() {
            "user" => t.user_msg,
            _ => t.mimo_msg,
        };

        let is_search_match = state.search_active
            && !state.search_matches.is_empty()
            && state.search_matches.get(state.search_match_idx) == Some(&msg_idx);

        for line in msg.content.as_str().lines() {
            // 代码块边界
            if let Some(lang_opt) = is_code_fence(line) {
                if !in_code_block {
                    in_code_block = true;
                    code_lang = lang_opt;
                    if let Some(ref lang) = code_lang {
                        lines.push(Line::from(Span::styled(
                            format!("   ┌─ {} ──", lang),
                            Style::default().fg(t.code_border).bg(t.code_bg),
                        )));
                    } else {
                        lines.push(Line::from(Span::styled(
                            "   ┌──────────",
                            Style::default().fg(t.code_border).bg(t.code_bg),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "   └──────────",
                        Style::default().fg(t.code_border).bg(t.code_bg),
                    )));
                    in_code_block = false;
                    code_lang = None;
                }
            } else if in_code_block {
                let syntax = if let Some(ref lang) = code_lang {
                    ps.find_syntax_by_token(lang)
                        .unwrap_or_else(|| ps.find_syntax_plain_text())
                } else {
                    ps.find_syntax_plain_text()
                };
                let mut h = HighlightLines::new(syntax, syn_theme);
                let mut code_spans = vec![Span::styled(
                    "  │ ",
                    Style::default().fg(t.code_border).bg(t.code_bg),
                )];
                highlight_line(&mut h, ps, line, &mut code_spans, t.code_bg);
                lines.push(Line::from(code_spans));
            } else if is_hr_line(line) {
                // 水平分隔线
                let hr_width = (area.width as usize).saturating_sub(6);
                let hr: String = "─".repeat(hr_width);
                lines.push(Line::from(Span::styled(
                    format!("   {}", hr),
                    Style::default().fg(t.hr_color),
                )));
            } else if let Some(rest) = is_unordered_list(line) {
                // 无序列表
                let mut spans = vec![
                    Span::styled("  • ", Style::default().fg(t.list_bullet)),
                ];
                spans.extend(render_inline_spans(rest, default_color, t));
                lines.push(Line::from(spans));
            } else if let Some((indent, rest)) = is_ordered_list(line) {
                // 有序列表
                let num_str: String = line.trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                let indent_pad = if indent.is_empty() { "  " } else { indent };
                let mut spans = vec![
                    Span::styled(
                        format!("{}{}. ", indent_pad, num_str),
                        Style::default().fg(t.list_bullet),
                    ),
                ];
                spans.extend(render_inline_spans(rest, default_color, t));
                lines.push(Line::from(spans));
            } else if let Some(rest) = line.strip_prefix('>') {
                // 引用块
                let rest = rest.strip_prefix(' ').unwrap_or(rest);
                let mut spans = vec![Span::styled(
                    "  │ ",
                    Style::default().fg(t.blockquote_border),
                )];
                spans.push(Span::styled(
                    rest,
                    Style::default().fg(t.mimo_msg).add_modifier(Modifier::ITALIC),
                ));
                lines.push(Line::from(spans));
            } else {
                let style = if is_search_match {
                    Style::default()
                        .fg(default_color)
                        .bg(Color::Rgb(86, 95, 137))
                } else {
                    Style::default().fg(default_color)
                };
                lines.push(render_markdown_line(line, style, t));
            }
        }

        if is_search_match && start_line < lines.len() {
            msg_line_ranges.push((start_line, msg_idx));
        }
    }

    if in_code_block {
        lines.push(Line::from(Span::styled(
            "   └──────────",
            Style::default().fg(t.code_border).bg(t.code_bg),
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
                Style::default().fg(t.spinner),
            )));
        } else {
            let mut stream_code_lang = if in_code_block {
                code_lang.clone()
            } else {
                None
            };

            for line in state.stream_buffer.lines() {
                if stream_code_lang.is_some() {
                    if is_code_fence(line).is_some() {
                        lines.push(Line::from(Span::styled(
                            "   └──────────",
                            Style::default().fg(t.code_border).bg(t.code_bg),
                        )));
                        stream_code_lang = None;
                        continue;
                    }
                    let syntax = if let Some(ref lang) = stream_code_lang {
                        ps.find_syntax_by_token(lang)
                            .unwrap_or_else(|| ps.find_syntax_plain_text())
                    } else {
                        ps.find_syntax_plain_text()
                    };
                    let mut h = HighlightLines::new(syntax, syn_theme);
                    let mut code_spans = vec![Span::styled(
                        "  │ ",
                        Style::default().fg(t.code_border).bg(t.code_bg),
                    )];
                    highlight_line(&mut h, ps, line, &mut code_spans, t.code_bg);
                    lines.push(Line::from(code_spans));
                } else if let Some(lang_opt) = is_code_fence(line) {
                    stream_code_lang = lang_opt.clone();
                    if let Some(ref lang) = lang_opt {
                        lines.push(Line::from(Span::styled(
                            format!("   ┌─ {} ──", lang),
                            Style::default().fg(t.code_border).bg(t.code_bg),
                        )));
                    } else {
                        lines.push(Line::from(Span::styled(
                            "   ┌──────────",
                            Style::default().fg(t.code_border).bg(t.code_bg),
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", line),
                        Style::default().fg(t.mimo_msg),
                    )));
                }
            }
            if stream_code_lang.is_some() {
                lines.push(Line::from(Span::styled(
                    "  │ ▎",
                    Style::default().fg(t.model_tag).bg(t.code_bg),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "   ▎",
                    Style::default().fg(t.model_tag),
                )));
            }
        }
    }

    // 滚动
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
        .border_style(Style::default().fg(t.code_border));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(paragraph, area);
}

/// 渲染行内 spans（不含行首前缀），与 render_markdown_line 相同逻辑但不加缩进
fn render_inline_spans<'a>(line: &'a str, base_color: Color, t: &ThemeColors) -> Vec<Span<'a>> {
    let base_style = Style::default().fg(base_color);
    let mut spans: Vec<Span> = Vec::new();
    let mut remaining = line;
    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix('`') {
            if let Some(end) = rest.find('`') {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    Style::default().fg(t.model_tag).bg(t.inline_code),
                ));
                remaining = &rest[end + 1..];
                continue;
            }
        }
        if let Some(rest) = remaining.strip_prefix('[') {
            if let Some(bracket_end) = rest.find("](") {
                let link_text = &rest[..bracket_end];
                let after_bracket = &rest[bracket_end + 2..];
                if let Some(paren_end) = after_bracket.find(')') {
                    spans.push(Span::styled(
                        link_text.to_string(),
                        Style::default()
                            .fg(t.link_color)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    remaining = &after_bracket[paren_end + 1..];
                    continue;
                }
            }
        }
        if let Some(rest) = remaining.strip_prefix("**") {
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    base_style.add_modifier(Modifier::BOLD),
                ));
                remaining = &rest[end + 2..];
                continue;
            }
        }
        if let Some(rest) = remaining.strip_prefix('*') {
            if !rest.starts_with('*') && !rest.starts_with(' ') {
                if let Some(end) = rest.find('*') {
                    spans.push(Span::styled(
                        rest[..end].to_string(),
                        base_style.add_modifier(Modifier::ITALIC),
                    ));
                    remaining = &rest[end + 1..];
                    continue;
                }
            }
        }
        let next_special = remaining
            .find(['`', '*', '['])
            .unwrap_or(remaining.len());
        if next_special > 0 {
            spans.push(Span::styled(remaining[..next_special].to_string(), base_style));
        }
        remaining = &remaining[next_special..];
    }
    spans
}

fn draw_search_bar(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    if !state.search_active || area.height == 0 {
        return;
    }
    let match_info = if state.search_matches.is_empty() {
        "no matches".to_string()
    } else {
        format!(
            "{}/{}",
            state.search_match_idx + 1,
            state.search_matches.len()
        )
    };
    let text = format!("  / {}  {}", state.search_query, match_info);
    let p = Paragraph::new(Span::styled(text, Style::default().fg(t.title)));
    f.render_widget(p, area);
}

fn draw_hint_area(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    if area.height == 0 || state.hint_lines.is_empty() {
        return;
    }
    let hint_text = state.hint_lines.join("  ");
    let p = Paragraph::new(Span::styled(
        format!("  {}", hint_text),
        Style::default().fg(t.status_label),
    ));
    f.render_widget(p, area);
}

fn draw_input_area(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    let mut spans = vec![Span::styled(
        "  › ",
        Style::default().fg(t.input_prompt),
    )];

    if state.search_active {
        spans.push(Span::styled(
            "Searching... (Esc to close)",
            Style::default().fg(t.status_label),
        ));
    } else if state.generating {
        let spinner_idx = state.spinner_tick % SPINNER.len();
        spans.push(Span::styled(
            format!("{} generating...", SPINNER[spinner_idx]),
            Style::default().fg(t.spinner),
        ));
    } else {
        let before = &state.input[..state.cursor_pos];
        let after = &state.input[state.cursor_pos..];

        spans.push(Span::styled(before, Style::default().fg(t.user_msg)));
        spans.push(Span::styled(
            "▎",
            Style::default()
                .fg(t.input_prompt)
                .add_modifier(Modifier::SLOW_BLINK),
        ));
        if !after.is_empty() {
            spans.push(Span::styled(after, Style::default().fg(t.user_msg)));
        }
    }

    let hint = if state.search_active {
        "Enter ↵  Esc ✕"
    } else if state.generating {
        "Esc"
    } else {
        "Ctrl+Enter ↵"
    };
    let hint_len = UnicodeWidthStr::width(hint);
    let total_width = area.width as usize;
    let content_len: usize = spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let padding = if total_width > content_len + hint_len + 4 {
        total_width - content_len - hint_len - 4
    } else {
        1
    };
    spans.push(Span::raw(" ".repeat(padding)));
    spans.push(Span::styled(hint, Style::default().fg(t.status_label)));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.separator))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    let total_tok = state.total_input_tokens + state.total_output_tokens;
    let token_text = format!(" ◉ {:.1}K tok ", total_tok as f64 / 1000.0);
    let io_text = format!(
        " ↓{} ↑{} ",
        state.total_input_tokens, state.total_output_tokens
    );
    let cost_text = format!(" ￥{:.4} ", state.total_cost);

    let msg_text = format!(" ✉ {} ", state.messages.len());

    let effective_input = state.total_input_tokens + state.total_cache_read_tokens;
    let cache_text = if effective_input > 0 {
        let hit_rate = state.total_cache_read_tokens as f64 / effective_input as f64 * 100.0;
        format!(" ♻ {:.0}% ", hit_rate)
    } else {
        String::new()
    };

    let mut spans = vec![
        Span::styled(msg_text, Style::default().fg(t.status_label)),
        Span::styled(token_text, Style::default().fg(t.status_data)),
        Span::styled(io_text, Style::default().fg(t.status_label)),
        Span::styled("  ", Style::default()),
        Span::styled(cost_text, Style::default().fg(t.model_tag)),
    ];

    if !cache_text.is_empty() {
        spans.push(Span::styled(cache_text, Style::default().fg(t.success)));
    }

    // 滚动指示器
    if state.chat_scroll > 0 {
        spans.push(Span::styled(
            format!(" [↑{}]", state.chat_scroll),
            Style::default().fg(t.status_label),
        ));
    }

    // 复制状态
    if let Some(ref copy_msg) = state.copy_status {
        spans.push(Span::styled(
            format!(" ✓ {}", copy_msg),
            Style::default().fg(t.success),
        ));
    }

    // 错误信息
    if let Some(ref err) = state.error_message {
        let err_text = format!(" ✗ {} ", err);
        let total_width = area.width as usize;
        let content_len: usize = spans
            .iter()
            .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
            .sum();
        let padding = if total_width > content_len + UnicodeWidthStr::width(err_text.as_str()) + 2 {
            total_width - content_len - UnicodeWidthStr::width(err_text.as_str()) - 2
        } else {
            1
        };
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(err_text, Style::default().fg(t.error)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.code_border))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}

fn draw_confirm_modal(f: &mut Frame, confirm: &crate::app::ConfirmState, t: &ThemeColors) {
    let area = f.area();

    let modal_w = 60u16.min(area.width.saturating_sub(4));
    let modal_h = 8u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(modal_w)) / 2;
    let y = (area.height.saturating_sub(modal_h)) / 2;
    let modal_area = Rect::new(x, y, modal_w, modal_h);

    ratatui::widgets::Clear.render(modal_area, f.buffer_mut());

    let mut lines: Vec<Line> = Vec::new();

    let title = match &confirm.action {
        ConfirmAction::WriteFile { .. } => " ⚠ 写入确认 ",
        ConfirmAction::EditFile { .. } => " ⚠ 编辑确认 ",
        ConfirmAction::SendMessage { .. } => " ⚠ 发送确认 ",
        ConfirmAction::ClearChat => " ⚠ 清空确认 ",
        ConfirmAction::Quit => " ⚠ 退出确认 ",
    };
    lines.push(Line::from(Span::styled(
        title,
        Style::default().fg(t.error).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    for line in confirm.detail.lines() {
        lines.push(Line::from(Span::styled(
            format!(" {}", line),
            Style::default().fg(t.user_msg),
        )));
    }

    lines.push(Line::raw(""));

    lines.push(Line::from(vec![
        Span::styled(" y", Style::default().fg(t.success).add_modifier(Modifier::BOLD)),
        Span::styled(" 确认  ", Style::default().fg(t.user_msg)),
        Span::styled("n", Style::default().fg(t.error).add_modifier(Modifier::BOLD)),
        Span::styled(" 取消  ", Style::default().fg(t.user_msg)),
        Span::styled("d", Style::default().fg(t.model_tag).add_modifier(Modifier::BOLD)),
        Span::styled(" 详情", Style::default().fg(t.user_msg)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.error))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, modal_area);
}
