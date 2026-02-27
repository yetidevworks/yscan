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
pub const THEME_NAMES: &[&str] = &[
    "dark",
    "light",
    "dracula",
    "nord",
    "onedark",
    "monokai-pro",
    "tokyo-night",
    "synthwave",
];

impl Theme {
    pub fn by_name(name: &str) -> Self {
        match name {
            "light" => Self::light(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "onedark" => Self::onedark(),
            "monokai-pro" => Self::monokai_pro(),
            "tokyo-night" => Self::tokyo_night(),
            "synthwave" => Self::synthwave(),
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

    /// Atom One Dark palette
    pub fn onedark() -> Self {
        Self {
            bg: Color::Rgb(40, 44, 52),
            fg: Color::Rgb(171, 178, 191),
            primary: Color::Rgb(97, 175, 239),
            secondary: Color::Rgb(198, 120, 221),
            accent: Color::Rgb(229, 192, 123),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            error: Color::Rgb(224, 108, 117),
            border: Color::Rgb(62, 68, 81),
            border_focused: Color::Rgb(97, 175, 239),
            selection_bg: Color::Rgb(50, 56, 66),
            header_fg: Color::Rgb(130, 137, 151),
            title_fg: Color::Rgb(97, 175, 239),
            muted: Color::Rgb(92, 99, 112),
        }
    }

    /// Monokai Pro palette
    pub fn monokai_pro() -> Self {
        Self {
            bg: Color::Rgb(45, 42, 46),
            fg: Color::Rgb(252, 252, 250),
            primary: Color::Rgb(120, 220, 232),
            secondary: Color::Rgb(171, 157, 242),
            accent: Color::Rgb(255, 216, 102),
            success: Color::Rgb(169, 220, 118),
            warning: Color::Rgb(255, 216, 102),
            error: Color::Rgb(255, 97, 136),
            border: Color::Rgb(73, 70, 75),
            border_focused: Color::Rgb(120, 220, 232),
            selection_bg: Color::Rgb(56, 53, 58),
            header_fg: Color::Rgb(144, 140, 147),
            title_fg: Color::Rgb(120, 220, 232),
            muted: Color::Rgb(114, 111, 117),
        }
    }

    /// Tokyo Night palette
    pub fn tokyo_night() -> Self {
        Self {
            bg: Color::Rgb(26, 27, 38),
            fg: Color::Rgb(169, 177, 214),
            primary: Color::Rgb(122, 162, 247),
            secondary: Color::Rgb(187, 154, 247),
            accent: Color::Rgb(224, 175, 104),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            border: Color::Rgb(41, 46, 66),
            border_focused: Color::Rgb(122, 162, 247),
            selection_bg: Color::Rgb(33, 35, 49),
            header_fg: Color::Rgb(125, 131, 160),
            title_fg: Color::Rgb(122, 162, 247),
            muted: Color::Rgb(86, 95, 137),
        }
    }

    /// Synthwave '84 palette
    pub fn synthwave() -> Self {
        Self {
            bg: Color::Rgb(38, 25, 52),
            fg: Color::Rgb(230, 225, 236),
            primary: Color::Rgb(255, 110, 199),
            secondary: Color::Rgb(114, 242, 249),
            accent: Color::Rgb(254, 215, 102),
            success: Color::Rgb(114, 242, 249),
            warning: Color::Rgb(254, 215, 102),
            error: Color::Rgb(254, 80, 96),
            border: Color::Rgb(58, 42, 75),
            border_focused: Color::Rgb(255, 110, 199),
            selection_bg: Color::Rgb(48, 33, 65),
            header_fg: Color::Rgb(148, 130, 168),
            title_fg: Color::Rgb(255, 110, 199),
            muted: Color::Rgb(118, 100, 138),
        }
    }
}
