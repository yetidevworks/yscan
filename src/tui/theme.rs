use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub header_fg: Color,
    pub title_fg: Color,
    pub muted: Color,
}

/// Ordered list of available themes for cycling
pub const THEME_NAMES: &[&str] = &["dark", "light", "dracula", "nord"];

impl Theme {
    pub fn by_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            _ => Self::dark(),
        }
    }

    /// Return the next theme name in the cycle
    pub fn next_name(current: &str) -> &'static str {
        let idx = THEME_NAMES.iter().position(|&n| n == current).unwrap_or(0);
        THEME_NAMES[(idx + 1) % THEME_NAMES.len()]
    }

    /// Catppuccin Mocha inspired
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            primary: Color::Rgb(137, 180, 250),
            secondary: Color::Rgb(180, 190, 254),
            accent: Color::Rgb(249, 226, 175),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            border: Color::Rgb(69, 71, 90),
            border_focused: Color::Rgb(137, 180, 250),
            selection_bg: Color::Rgb(49, 50, 68),
            header_fg: Color::Rgb(166, 173, 200),
            title_fg: Color::Rgb(137, 180, 250),
            muted: Color::Rgb(108, 112, 134),
        }
    }

    /// Catppuccin Latte inspired
    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(239, 241, 245),
            fg: Color::Rgb(76, 79, 105),
            primary: Color::Rgb(30, 102, 245),
            secondary: Color::Rgb(114, 135, 253),
            accent: Color::Rgb(223, 142, 29),
            success: Color::Rgb(64, 160, 43),
            warning: Color::Rgb(223, 142, 29),
            error: Color::Rgb(210, 15, 57),
            border: Color::Rgb(188, 192, 204),
            border_focused: Color::Rgb(30, 102, 245),
            selection_bg: Color::Rgb(204, 208, 218),
            header_fg: Color::Rgb(92, 95, 119),
            title_fg: Color::Rgb(30, 102, 245),
            muted: Color::Rgb(140, 143, 161),
        }
    }

    /// Dracula palette
    pub fn dracula() -> Self {
        Self {
            bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            primary: Color::Rgb(189, 147, 249),
            secondary: Color::Rgb(139, 233, 253),
            accent: Color::Rgb(255, 184, 108),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            error: Color::Rgb(255, 85, 85),
            border: Color::Rgb(68, 71, 90),
            border_focused: Color::Rgb(189, 147, 249),
            selection_bg: Color::Rgb(68, 71, 90),
            header_fg: Color::Rgb(98, 114, 164),
            title_fg: Color::Rgb(189, 147, 249),
            muted: Color::Rgb(98, 114, 164),
        }
    }

    /// Nord palette
    pub fn nord() -> Self {
        Self {
            bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(216, 222, 233),
            primary: Color::Rgb(136, 192, 208),
            secondary: Color::Rgb(129, 161, 193),
            accent: Color::Rgb(235, 203, 139),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(191, 97, 106),
            border: Color::Rgb(67, 76, 94),
            border_focused: Color::Rgb(136, 192, 208),
            selection_bg: Color::Rgb(59, 66, 82),
            header_fg: Color::Rgb(143, 150, 163),
            title_fg: Color::Rgb(136, 192, 208),
            muted: Color::Rgb(107, 112, 127),
        }
    }
}
