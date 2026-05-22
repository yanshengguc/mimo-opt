use std::sync::OnceLock;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use syntect::{
    highlighting::{
        Color as SynColor, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSet,
    },
    parsing::SyntaxSet,
};
use unicode_width::UnicodeWidthStr;

mod chat;
mod modal;
mod sidebar;

use super::theme::ThemeColors;
use crate::app::AppState;

pub(super) const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static SYN_THEME: OnceLock<Theme> = OnceLock::new();

/// 异步预加载语法高亮（由 app.rs 在启动时 spawn_blocking 调用）
pub fn init_syntax_async() {
    std::thread::spawn(|| {
        let ss = SyntaxSet::load_defaults_newlines();
        let tm = build_theme();
        SYNTAX_SET.set(ss).ok();
        SYN_THEME.set(tm).ok();
    });
}

pub(super) fn syntax_set() -> Option<&'static SyntaxSet> {
    SYNTAX_SET.get()
}

pub(super) fn theme() -> Option<&'static Theme> {
    SYN_THEME.get()
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
        scope: selector
            .parse::<ScopeSelectors>()
            .expect("hardcoded scope selector"),
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

// ── 布局入口 ──

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

    // 自适应输入框高度：3 ~ 半屏
    let input_height = if state.search_active || state.generating {
        3u16
    } else {
        let max_input = (area.height / 2).max(3);
        let input_width = area.width.saturating_sub(6) as usize; // borders + prompt
        let text_lines: usize = state.input.lines().count();
        let wrap_lines: usize = state
            .input
            .lines()
            .map(|line| {
                let w = UnicodeWidthStr::width(line);
                if input_width > 0 && w > input_width {
                    w.div_ceil(input_width)
                } else {
                    1
                }
            })
            .sum();
        let needed = (text_lines.max(wrap_lines) + 1) as u16;
        needed.clamp(3, max_input)
    };

    // 侧边栏布局
    let main_area = if state.sidebar_open && area.width > 40 {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(30)])
            .split(area);
        sidebar::draw_sidebar(f, h[0], state, theme);
        h[1]
    } else {
        area
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(hint_len),
            Constraint::Length(search_len),
            Constraint::Length(input_height),
            Constraint::Length(3),
        ])
        .split(main_area);

    draw_title_bar(f, chunks[0], state, theme);
    chat::draw_chat_area(f, chunks[1], state, theme);
    draw_hint_area(f, chunks[2], state, theme);
    draw_search_bar(f, chunks[3], state, theme);
    draw_input_area(f, chunks[4], state, theme);
    draw_status_bar(f, chunks[5], state, theme);

    if let Some(ref confirm) = state.pending_confirm {
        modal::draw_confirm_modal(f, confirm, theme);
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

    let mut right_spans: Vec<Span> =
        vec![Span::styled(model_text, Style::default().fg(t.model_tag))];
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
    let mut spans = vec![Span::styled("  › ", Style::default().fg(t.input_prompt))];

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

    // 费用预估
    let estimate_text = if !state.search_active && !state.generating && !state.input.is_empty() {
        let tokens = crate::util::estimate_tokens(&state.input);
        let cost = crate::util::format_cost_estimate(tokens, state.config.input_price());
        let est = format!(" {}  ~{}tok ", cost, tokens);
        let warn =
            tokens > 20000 || (tokens as f64 / 1_000_000.0 * state.config.input_price()) > 0.5;
        Some((est, warn))
    } else {
        None
    };

    let est_len = estimate_text
        .as_ref()
        .map_or(0, |(s, _)| UnicodeWidthStr::width(s.as_str()));
    let hint_len = UnicodeWidthStr::width(hint);
    let total_width = area.width as usize;
    let content_len: usize = spans
        .iter()
        .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
        .sum();
    let padding = if total_width > content_len + est_len + hint_len + 4 {
        total_width - content_len - est_len - hint_len - 4
    } else {
        1
    };
    spans.push(Span::raw(" ".repeat(padding)));

    if let Some((est, warn)) = estimate_text {
        let color = if warn { t.error } else { t.status_label };
        spans.push(Span::styled(est, Style::default().fg(color)));
    }

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
