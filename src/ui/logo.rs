use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};

/// Render the mango-cat startup logo. Returns true if any key was pressed to dismiss.
pub fn draw_logo(
    f: &mut Frame,
    area: Rect,
    version: &str,
    provider: &str,
    model: &str,
    balance: Option<&str>,
) {
    let mango = Color::Rgb(0xFF, 0x8C, 0x00);
    let mango_light = Color::Rgb(0xFF, 0xB3, 0x47);
    let mango_dark = Color::Rgb(0xCC, 0x70, 0x00);
    let cream = Color::Rgb(0xFF, 0xF0, 0xD0);
    let eye = Color::Rgb(0x2A, 0x2A, 0x3E);
    let pink = Color::Rgb(0xFF, 0x99, 0xBB);

    let pixels: &[[(Color, &str); 8]] = &[
        [
            (Color::Reset, "  "),
            (mango, "██"),
            (mango, "██"),
            (Color::Reset, "  "),
            (Color::Reset, "  "),
            (mango, "██"),
            (mango, "██"),
            (Color::Reset, "  "),
        ],
        [
            (pink, "██"),
            (mango, "██"),
            (mango_light, "██"),
            (cream, "██"),
            (cream, "██"),
            (mango_light, "██"),
            (mango, "██"),
            (pink, "██"),
        ],
        [
            (Color::Reset, "  "),
            (mango, "██"),
            (eye, "██"),
            (mango_light, "██"),
            (mango_light, "██"),
            (eye, "██"),
            (mango, "██"),
            (Color::Reset, "  "),
        ],
        [
            (Color::Reset, "  "),
            (mango, "██"),
            (mango_light, "██"),
            (mango_dark, "██"),
            (mango_dark, "██"),
            (mango_light, "██"),
            (mango, "██"),
            (Color::Reset, "  "),
        ],
        [
            (Color::Reset, "  "),
            (Color::Reset, "  "),
            (mango, "██"),
            (mango_light, "██"),
            (mango_light, "██"),
            (mango, "██"),
            (Color::Reset, "  "),
            (Color::Reset, "  "),
        ],
    ];

    let mut logo_lines: Vec<Line> = Vec::new();
    for row in pixels {
        let spans: Vec<Span> = row
            .iter()
            .map(|(color, text)| Span::styled(*text, Style::default().fg(Color::White).bg(*color)))
            .collect();
        logo_lines.push(Line::from(spans));
    }

    let provider_label = match provider {
        "deepseek" => "DeepSeek",
        "openai" => "OpenAI",
        "mimo" => "MiMo",
        _ => provider,
    };

    let mut info_lines = vec![
        Line::from(Span::styled(
            format!("  MiMo-OPT  v{}", version),
            Style::default().fg(mango),
        )),
        Line::from(Span::styled(
            "  芒果猫 · 终端 AI 助手",
            Style::default().fg(mango_light),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            format!("  Provider: {}", provider_label),
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            format!("  Model: {}", model),
            Style::default().fg(Color::Gray),
        )),
    ];

    if let Some(bal) = balance {
        info_lines.push(Line::from(Span::styled(
            format!("  Balance: {}", bal),
            Style::default().fg(Color::Gray),
        )));
    }

    info_lines.push(Line::raw(""));
    info_lines.push(Line::from(Span::styled(
        "  按任意键开始...",
        Style::default().fg(Color::DarkGray),
    )));

    // Center both columns
    let logo_width = 16u16; // 8 chars × 2 wide
    let info_width = 28u16;
    let total_width = logo_width + 3 + info_width;
    let x = (area.width.saturating_sub(total_width)) / 2;
    let y = (area.height.saturating_sub(8)) / 2;

    let logo_area = Rect::new(x, y, logo_width, 5);
    let info_area = Rect::new(x + logo_width + 2, y, info_width, 8);

    // White block behind logo
    let bg_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(mango));
    let outer = Rect::new(x.saturating_sub(2), y.saturating_sub(1), total_width + 4, 9);
    ratatui::widgets::Clear.render(outer, f.buffer_mut());
    f.render_widget(bg_block, outer);

    f.render_widget(Paragraph::new(logo_lines), logo_area);
    f.render_widget(Paragraph::new(info_lines), info_area);
}
