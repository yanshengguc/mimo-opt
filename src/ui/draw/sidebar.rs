use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use super::ThemeColors;
use crate::app::AppState;
use crate::util::format_age;

pub fn draw_sidebar(f: &mut Frame, area: Rect, state: &AppState, t: &ThemeColors) {
    let items: Vec<ListItem> = state
        .session_list
        .iter()
        .enumerate()
        .map(|(i, info)| {
            let is_current = info.id == state.session.id;
            let is_selected = i == state.sidebar_idx;

            let short_name = if info.name.len() > 14 {
                format!("{}...", &info.name[..11])
            } else {
                info.name.clone()
            };

            let age = format_age(info.updated_at);

            let mut spans = vec![];

            if is_selected {
                spans.push(Span::styled("▸ ", Style::default().fg(t.input_prompt)));
            } else {
                spans.push(Span::raw("  "));
            }

            let name_style = if is_current {
                Style::default().fg(t.title).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.user_msg)
            };
            spans.push(Span::styled(short_name, name_style));

            let meta = format!(" {} ·{}", info.msg_count, age);
            spans.push(Span::styled(meta, Style::default().fg(t.status_label)));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = " 会话 (↑↓ 选择, Enter 切换) ".to_string();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.border_dim))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(Span::styled(title, Style::default().fg(t.status_label))),
    );

    f.render_widget(list, area);
}
