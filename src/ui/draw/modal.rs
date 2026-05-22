use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};

use crate::app::ConfirmAction;
use crate::ui::theme::ThemeColors;

pub fn draw_confirm_modal(f: &mut Frame, confirm: &crate::app::ConfirmState, t: &ThemeColors) {
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
        if let Some(rest) = line.strip_prefix("+ ") {
            lines.push(Line::from(Span::styled(
                format!("+ {}", rest),
                Style::default().fg(t.success),
            )));
        } else if let Some(rest) = line.strip_prefix("- ") {
            lines.push(Line::from(Span::styled(
                format!("- {}", rest),
                Style::default().fg(t.error),
            )));
        } else {
            let trimmed = line.strip_prefix("  ").unwrap_or(line);
            lines.push(Line::from(Span::styled(
                format!("  {}", trimmed),
                Style::default().fg(t.user_msg),
            )));
        }
    }

    lines.push(Line::raw(""));

    lines.push(Line::from(vec![
        Span::styled(
            " y",
            Style::default().fg(t.success).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 确认  ", Style::default().fg(t.user_msg)),
        Span::styled(
            "n",
            Style::default().fg(t.error).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 取消  ", Style::default().fg(t.user_msg)),
        Span::styled(
            "d",
            Style::default()
                .fg(t.model_tag)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 详情", Style::default().fg(t.user_msg)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.error))
        .border_type(ratatui::widgets::BorderType::Rounded);

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, modal_area);
}
