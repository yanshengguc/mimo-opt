use ratatui::style::Color;

pub struct Theme;

impl Theme {
    // Tokyo Night 色系
    pub const BORDER_DIM: Color = Color::Rgb(86, 95, 137); // #565f89
    pub const TITLE: Color = Color::Rgb(122, 162, 247); // #7aa2f7 亮紫
    pub const MODEL_TAG: Color = Color::Rgb(125, 207, 255); // #7dcfff 青色
    pub const USER_MSG: Color = Color::Rgb(192, 202, 245); // #c0caf5 亮白
    pub const MIMO_MSG: Color = Color::Rgb(169, 177, 214); // #a9b1d6 浅灰白
    pub const CODE_BORDER: Color = Color::Rgb(59, 66, 97); // #3b4261 深灰
    pub const CODE_BG: Color = Color::Rgb(26, 27, 38); // #1a1b26 深蓝黑
    pub const STATUS_DATA: Color = Color::Rgb(125, 207, 255); // #7dcfff 青色
    pub const STATUS_LABEL: Color = Color::Rgb(86, 95, 137); // #565f89 暗灰
    pub const INPUT_PROMPT: Color = Color::Rgb(187, 154, 247); // #bb9af7 紫色
    pub const SPINNER: Color = Color::Rgb(125, 207, 255); // #7dcfff 青色
    pub const ERROR: Color = Color::Rgb(247, 118, 142); // #f7768e 暗红
    pub const SUCCESS: Color = Color::Rgb(158, 206, 106); // #9ece6a 绿色
    pub const SEPARATOR: Color = Color::Rgb(59, 66, 97); // #3b4261 深灰
}
