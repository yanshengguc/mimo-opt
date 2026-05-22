use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use syntect::{easy::HighlightLines, parsing::SyntaxSet};

use crate::app::AppState;
use crate::ui::theme::ThemeColors;

pub fn is_code_fence(line: &str) -> Option<Option<String>> {
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
pub fn is_hr_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() < 3 {
        return false;
    }
    let ch = trimmed.chars().next().expect("len >= 3");
    if ch != '-' && ch != '*' && ch != '_' {
        return false;
    }
    trimmed.chars().all(|c| c == ch || c == ' ')
}

/// 检测无序列表项: 以 "- " 或 "* " 开头
pub fn is_unordered_list(line: &str) -> Option<&str> {
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
pub fn is_ordered_list(line: &str) -> Option<(&str, &str)> {
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

fn render_opening_fence<'a>(lang: &Option<String>, code_bg: Color, code_border: Color) -> Line<'a> {
    if let Some(ref l) = lang {
        Line::from(Span::styled(
            format!("   ┌─ {} ──", l),
            Style::default().fg(code_border).bg(code_bg),
        ))
    } else {
        Line::from(Span::styled(
            "   ┌──────────",
            Style::default().fg(code_border).bg(code_bg),
        ))
    }
}

fn render_closing_fence<'a>(code_bg: Color, code_border: Color) -> Line<'a> {
    Line::from(Span::styled(
        "   └──────────",
        Style::default().fg(code_border).bg(code_bg),
    ))
}

fn render_code_line<'a>(
    line: &'a str,
    code_lang: &Option<String>,
    ps: Option<&'static SyntaxSet>,
    syn_theme: Option<&'static syntect::highlighting::Theme>,
    code_bg: Color,
    code_border: Color,
    fallback_color: Color,
) -> Line<'a> {
    let mut code_spans = vec![Span::styled(
        "  │ ",
        Style::default().fg(code_border).bg(code_bg),
    )];
    if let (Some(ps), Some(syn_theme)) = (ps, syn_theme) {
        let syntax = if let Some(ref lang) = code_lang {
            ps.find_syntax_by_token(lang)
                .unwrap_or_else(|| ps.find_syntax_plain_text())
        } else {
            ps.find_syntax_plain_text()
        };
        let mut h = HighlightLines::new(syntax, syn_theme);
        highlight_line(&mut h, ps, line, &mut code_spans, code_bg);
    } else {
        code_spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(fallback_color).bg(code_bg),
        ));
    }
    Line::from(code_spans)
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
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(code_bg).bg(code_bg),
            ));
        }
    }
}

/// Parse inline Markdown spans: `code`, [links](url), **bold**, *italic*, plain text
fn parse_inline_spans<'a>(line: &'a str, base_style: Style, t: &ThemeColors) -> Vec<Span<'a>> {
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
        let next_special = remaining.find(['`', '*', '[']).unwrap_or(remaining.len());
        if next_special > 0 {
            spans.push(Span::styled(
                remaining[..next_special].to_string(),
                base_style,
            ));
        }
        remaining = &remaining[next_special..];
    }
    spans
}

/// 渲染单行文本（带行首缩进）
fn render_markdown_line<'a>(line: &'a str, base_style: Style, t: &ThemeColors) -> Line<'a> {
    let mut spans = vec![Span::styled("   ", base_style)];
    spans.extend(parse_inline_spans(line, base_style, t));
    Line::from(spans)
}

fn render_inline_spans<'a>(line: &'a str, base_color: Color, t: &ThemeColors) -> Vec<Span<'a>> {
    parse_inline_spans(line, Style::default().fg(base_color), t)
}

use super::SPINNER;

pub fn draw_chat_area(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    let ps = super::syntax_set();
    let syn_theme = super::theme();

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
                    lines.push(render_opening_fence(&code_lang, t.code_bg, t.code_border));
                } else {
                    lines.push(render_closing_fence(t.code_bg, t.code_border));
                    in_code_block = false;
                    code_lang = None;
                }
            } else if in_code_block {
                lines.push(render_code_line(
                    line,
                    &code_lang,
                    ps,
                    syn_theme,
                    t.code_bg,
                    t.code_border,
                    t.user_msg,
                ));
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
                let mut spans = vec![Span::styled("  • ", Style::default().fg(t.list_bullet))];
                spans.extend(render_inline_spans(rest, default_color, t));
                lines.push(Line::from(spans));
            } else if let Some((indent, rest)) = is_ordered_list(line) {
                // 有序列表
                let num_str: String = line
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                let indent_pad = if indent.is_empty() { "  " } else { indent };
                let mut spans = vec![Span::styled(
                    format!("{}{}. ", indent_pad, num_str),
                    Style::default().fg(t.list_bullet),
                )];
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
                    Style::default()
                        .fg(t.mimo_msg)
                        .add_modifier(Modifier::ITALIC),
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
                        lines.push(render_closing_fence(t.code_bg, t.code_border));
                        stream_code_lang = None;
                        continue;
                    }
                    lines.push(render_code_line(
                        line,
                        &stream_code_lang,
                        ps,
                        syn_theme,
                        t.code_bg,
                        t.code_border,
                        t.user_msg,
                    ));
                } else if let Some(lang_opt) = is_code_fence(line) {
                    stream_code_lang = lang_opt;
                    lines.push(render_opening_fence(
                        &stream_code_lang,
                        t.code_bg,
                        t.code_border,
                    ));
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

    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::LEFT | ratatui::widgets::Borders::RIGHT)
        .border_style(Style::default().fg(t.code_border));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(paragraph, area);
}
