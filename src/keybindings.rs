use std::collections::HashMap;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyModifiers};

/// 一个按键组合: key + modifiers
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn matches(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.code == code && self.modifiers == modifiers
    }
}

/// 按键绑定配置
#[derive(Clone)]
pub struct KeyBindings {
    bindings: HashMap<String, KeyCombo>,
}

impl KeyBindings {
    /// 从配置文件加载，文件不存在时使用默认值
    pub fn load() -> Self {
        let defaults = Self::defaults();
        let path = match Self::config_path() {
            Ok(p) => p,
            Err(_) => return defaults,
        };
        if !path.exists() {
            return defaults;
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<HashMap<String, String>>(&content) {
                Ok(map) => {
                    let mut bindings = defaults.bindings.clone();
                    for (action, key_str) in map {
                        if let Some(combo) = parse_key_combo(&key_str) {
                            bindings.insert(action, combo);
                        } else {
                            log::warn!("快捷键解析失败: {} = {}", action, key_str);
                        }
                    }
                    Self { bindings }
                }
                Err(e) => {
                    log::warn!("keybindings.json 解析失败: {}", e);
                    defaults
                }
            },
            Err(e) => {
                log::warn!("keybindings.json 读取失败: {}", e);
                defaults
            }
        }
    }

    pub fn config_path() -> anyhow::Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取配置目录"))?
            .join("mimo-opt");
        Ok(dir.join("keybindings.json"))
    }

    fn defaults() -> Self {
        let mut m = HashMap::new();
        m.insert(
            "toggle_sidebar".into(),
            KeyCombo::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        );
        m.insert(
            "new_session".into(),
            KeyCombo::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
        );
        m.insert(
            "next_session".into(),
            KeyCombo::new(KeyCode::F(2), KeyModifiers::NONE),
        );
        m.insert(
            "send".into(),
            KeyCombo::new(KeyCode::Enter, KeyModifiers::CONTROL),
        );
        m.insert(
            "quit".into(),
            KeyCombo::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
        );
        m.insert(
            "cancel".into(),
            KeyCombo::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        m.insert(
            "paste".into(),
            KeyCombo::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
        );
        m.insert(
            "search".into(),
            KeyCombo::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        );
        m.insert(
            "copy_code".into(),
            KeyCombo::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        );
        m.insert(
            "undo".into(),
            KeyCombo::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
        );
        m.insert(
            "cursor_home".into(),
            KeyCombo::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        );
        m.insert(
            "cursor_end".into(),
            KeyCombo::new(KeyCode::Char('e'), KeyModifiers::CONTROL),
        );
        m.insert(
            "scroll_top".into(),
            KeyCombo::new(KeyCode::Home, KeyModifiers::CONTROL),
        );
        Self { bindings: m }
    }

    /// 检查某个动作的按键是否匹配
    pub fn is(&self, action: &str, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.bindings
            .get(action)
            .is_some_and(|combo| combo.matches(code, modifiers))
    }

    /// 获取某个动作的按键描述（用于 UI 显示）
    #[allow(dead_code)]
    pub fn describe(&self, action: &str) -> String {
        self.bindings
            .get(action)
            .map_or_else(|| "???".to_string(), format_combo)
    }
}

#[allow(dead_code)]
fn format_combo(combo: &KeyCombo) -> String {
    let mut parts = Vec::new();
    if combo.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if combo.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }
    if combo.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }
    let key_str = match combo.code {
        KeyCode::Char(c) => c.to_uppercase().to_string(),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::Backspace => "Bksp".into(),
        KeyCode::Delete => "Del".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PgUp".into(),
        KeyCode::PageDown => "PgDn".into(),
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        _ => "?".into(),
    };
    parts.push(&key_str);
    parts.join("+")
}

/// 解析 "Ctrl+B" 格式的按键描述
fn parse_key_combo(s: &str) -> Option<KeyCombo> {
    let parts: Vec<&str> = s.split('+').collect();
    let mut modifiers = KeyModifiers::NONE;
    let mut key_part = "";

    for (i, part) in parts.iter().enumerate() {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "alt" => modifiers |= KeyModifiers::ALT,
            _ => {
                if i != parts.len() - 1 {
                    return None; // 非修饰键不在最后位置
                }
                key_part = part;
            }
        }
    }

    if key_part.is_empty() {
        return None;
    }

    let code = if key_part.len() == 1 {
        KeyCode::Char(key_part.to_lowercase().chars().next().unwrap())
    } else {
        match key_part.to_uppercase().as_str() {
            s if s.starts_with('F') && s.len() > 1 => {
                let n: u8 = s[1..].parse().ok()?;
                if n == 0 || n > 12 {
                    return None;
                }
                KeyCode::F(n)
            }
            "ENTER" | "RETURN" => KeyCode::Enter,
            "ESC" | "ESCAPE" => KeyCode::Esc,
            "TAB" => KeyCode::Tab,
            "BACKSPACE" | "BKSP" => KeyCode::Backspace,
            "DELETE" | "DEL" => KeyCode::Delete,
            "HOME" => KeyCode::Home,
            "END" => KeyCode::End,
            "PAGEUP" | "PGUP" => KeyCode::PageUp,
            "PAGEDOWN" | "PGDN" => KeyCode::PageDown,
            "UP" => KeyCode::Up,
            "DOWN" => KeyCode::Down,
            "LEFT" => KeyCode::Left,
            "RIGHT" => KeyCode::Right,
            "SPACE" => KeyCode::Char(' '),
            _ => return None,
        }
    };

    Some(KeyCombo { code, modifiers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_char() {
        let combo = parse_key_combo("Ctrl+B").unwrap();
        assert_eq!(combo.code, KeyCode::Char('b'));
        assert!(combo.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn parse_function_key() {
        let combo = parse_key_combo("F2").unwrap();
        assert_eq!(combo.code, KeyCode::F(2));
        assert_eq!(combo.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn parse_shift_enter() {
        let combo = parse_key_combo("Shift+Enter").unwrap();
        assert_eq!(combo.code, KeyCode::Enter);
        assert!(combo.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn parse_case_insensitive() {
        let a = parse_key_combo("ctrl+b").unwrap();
        let b = parse_key_combo("CTRL+B").unwrap();
        let c = parse_key_combo("Ctrl+B").unwrap();
        assert_eq!(a.code, b.code);
        assert_eq!(b.code, c.code);
    }

    #[test]
    fn parse_special_keys() {
        assert!(parse_key_combo("Esc").is_some());
        assert!(parse_key_combo("Home").is_some());
        assert!(parse_key_combo("End").is_some());
        assert!(parse_key_combo("PgUp").is_some());
        assert!(parse_key_combo("Space").is_some());
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_key_combo("").is_none());
        assert!(parse_key_combo("Ctrl+").is_none());
        assert!(parse_key_combo("F0").is_none());
        assert!(parse_key_combo("F13").is_none());
        assert!(parse_key_combo("Ctrl+Ctrl").is_none());
        assert!(parse_key_combo("FooBar").is_none());
    }

    #[test]
    fn format_combo_display() {
        let combo = KeyCombo::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        assert_eq!(format_combo(&combo), "Ctrl+B");

        let combo = KeyCombo::new(KeyCode::F(2), KeyModifiers::NONE);
        assert_eq!(format_combo(&combo), "F2");
    }

    #[test]
    fn defaults_contain_expected_actions() {
        let kb = KeyBindings::defaults();
        assert!(kb.is("toggle_sidebar", KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert!(kb.is("send", KeyCode::Enter, KeyModifiers::CONTROL));
        assert!(kb.is("quit", KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(!kb.is("send", KeyCode::Char('q'), KeyModifiers::CONTROL));
    }

    #[test]
    fn describe_returns_readable() {
        let kb = KeyBindings::defaults();
        assert_eq!(kb.describe("toggle_sidebar"), "Ctrl+B");
        assert_eq!(kb.describe("send"), "Ctrl+Enter");
        assert_eq!(kb.describe("nonexistent"), "???");
    }
}
