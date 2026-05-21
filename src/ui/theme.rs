use ratatui::style::Color;

/// 主题色板 — 所有 UI 颜色集中定义
#[derive(Clone, Debug)]
pub struct ThemeColors {
    pub border_dim: Color,
    pub title: Color,
    pub model_tag: Color,
    pub user_msg: Color,
    pub mimo_msg: Color,
    pub code_border: Color,
    pub code_bg: Color,
    pub status_data: Color,
    pub status_label: Color,
    pub input_prompt: Color,
    pub spinner: Color,
    pub error: Color,
    pub success: Color,
    pub separator: Color,
    pub blockquote_border: Color,
    pub inline_code: Color,
    pub link_color: Color,
    pub list_bullet: Color,
    pub hr_color: Color,
}

pub const TOKYO_NIGHT: ThemeColors = ThemeColors {
    border_dim: Color::Rgb(86, 95, 137),       // #565f89
    title: Color::Rgb(122, 162, 247),           // #7aa2f7
    model_tag: Color::Rgb(125, 207, 255),       // #7dcfff
    user_msg: Color::Rgb(192, 202, 245),        // #c0caf5
    mimo_msg: Color::Rgb(169, 177, 214),        // #a9b1d6
    code_border: Color::Rgb(59, 66, 97),        // #3b4261
    code_bg: Color::Rgb(26, 27, 38),            // #1a1b26
    status_data: Color::Rgb(125, 207, 255),     // #7dcfff
    status_label: Color::Rgb(86, 95, 137),      // #565f89
    input_prompt: Color::Rgb(187, 154, 247),    // #bb9af7
    spinner: Color::Rgb(125, 207, 255),         // #7dcfff
    error: Color::Rgb(247, 118, 142),           // #f7768e
    success: Color::Rgb(158, 206, 106),         // #9ece6a
    separator: Color::Rgb(59, 66, 97),          // #3b4261
    blockquote_border: Color::Rgb(61, 89, 161), // #3d59a1
    inline_code: Color::Rgb(36, 40, 59),        // #24283b
    link_color: Color::Rgb(125, 207, 255),      // #7dcfff
    list_bullet: Color::Rgb(86, 95, 137),       // #565f89
    hr_color: Color::Rgb(59, 66, 97),           // #3b4261
};

pub const NORD: ThemeColors = ThemeColors {
    border_dim: Color::Rgb(76, 86, 106),        // #4c566a
    title: Color::Rgb(136, 192, 208),           // #88c0d0
    model_tag: Color::Rgb(143, 188, 187),       // #8fbcbb
    user_msg: Color::Rgb(236, 239, 244),        // #eceff4
    mimo_msg: Color::Rgb(216, 222, 233),        // #d8dee9
    code_border: Color::Rgb(67, 76, 94),        // #434c5e
    code_bg: Color::Rgb(46, 52, 64),            // #2e3440
    status_data: Color::Rgb(136, 192, 208),     // #88c0d0
    status_label: Color::Rgb(76, 86, 106),      // #4c566a
    input_prompt: Color::Rgb(180, 142, 173),    // #b48ead
    spinner: Color::Rgb(136, 192, 208),         // #88c0d0
    error: Color::Rgb(191, 97, 106),            // #bf616a
    success: Color::Rgb(163, 190, 140),         // #a3be8c
    separator: Color::Rgb(76, 86, 106),         // #4c566a
    blockquote_border: Color::Rgb(94, 129, 172),// #5e81ac
    inline_code: Color::Rgb(59, 66, 82),        // #3b4252
    link_color: Color::Rgb(143, 188, 187),      // #8fbcbb
    list_bullet: Color::Rgb(76, 86, 106),       // #4c566a
    hr_color: Color::Rgb(76, 86, 106),          // #4c566a
};

pub const CATPPUCCIN: ThemeColors = ThemeColors {
    border_dim: Color::Rgb(88, 91, 112),        // #585b70
    title: Color::Rgb(137, 180, 250),           // #89b4fa
    model_tag: Color::Rgb(148, 226, 213),       // #94e2d5
    user_msg: Color::Rgb(205, 214, 244),        // #cdd6f4
    mimo_msg: Color::Rgb(186, 194, 222),        // #bac2de
    code_border: Color::Rgb(69, 71, 90),        // #45475a
    code_bg: Color::Rgb(30, 30, 46),            // #1e1e2e
    status_data: Color::Rgb(148, 226, 213),     // #94e2d5
    status_label: Color::Rgb(88, 91, 112),      // #585b70
    input_prompt: Color::Rgb(203, 166, 247),    // #cba6f7
    spinner: Color::Rgb(148, 226, 213),         // #94e2d5
    error: Color::Rgb(243, 139, 168),           // #f38ba8
    success: Color::Rgb(166, 227, 161),         // #a6e3a1
    separator: Color::Rgb(69, 71, 90),          // #45475a
    blockquote_border: Color::Rgb(137, 180, 250),// #89b4fa
    inline_code: Color::Rgb(49, 50, 68),        // #313244
    link_color: Color::Rgb(148, 226, 213),      // #94e2d5
    list_bullet: Color::Rgb(88, 91, 112),       // #585b70
    hr_color: Color::Rgb(69, 71, 90),           // #45475a
};

/// 根据名称获取主题色板（默认 Tokyo Night）
pub fn get_theme(name: &str) -> &'static ThemeColors {
    match name {
        "nord" => &NORD,
        "catppuccin" => &CATPPUCCIN,
        _ => &TOKYO_NIGHT,
    }
}

/// 所有可用主题名称
pub fn theme_names() -> &'static [&'static str] {
    &["tokyo-night", "nord", "catppuccin"]
}
