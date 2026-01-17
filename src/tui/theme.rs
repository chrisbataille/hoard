//! Theme support for the TUI
//!
//! Provides multiple color themes including Catppuccin, Dracula, and Nord.
//! Also supports custom user-defined themes via JSON files.

use anyhow::{Context, Result};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;

/// A complete color theme for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    // Base colors
    pub base: Color,     // Main background
    pub surface0: Color, // Slightly elevated surface
    pub surface1: Color, // Borders, separators
    // Text colors
    pub text: Color,     // Primary text
    pub subtext0: Color, // Secondary/dimmed text
    // Accent colors
    pub blue: Color,   // Links, highlights
    pub green: Color,  // Success, installed
    pub yellow: Color, // Warnings, stars
    pub red: Color,    // Errors, destructive
    pub mauve: Color,  // Categories
    pub peach: Color,  // Source badges
    pub teal: Color,   // Sparklines, metrics
}

/// RGB color for JSON serialization
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }
}

/// Custom theme definition for JSON file
///
/// Create a file at `~/.config/hoards/custom-theme.json` with this structure.
/// Colors are specified as RGB objects with r, g, b values (0-255).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    /// JSON Schema reference (optional, for IDE support)
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Theme display name
    pub name: String,

    // Base colors - used for backgrounds
    /// Main background color for the entire TUI
    pub base: RgbColor,
    /// Slightly elevated surface (list items, cards)
    pub surface0: RgbColor,
    /// Borders, separators, inactive elements
    pub surface1: RgbColor,

    // Text colors
    /// Primary text color for main content
    pub text: RgbColor,
    /// Secondary/dimmed text (descriptions, hints)
    pub subtext0: RgbColor,

    // Accent colors - semantic meanings
    /// Links, highlights, selected items, focused elements
    pub blue: RgbColor,
    /// Success states, installed tools, positive indicators
    pub green: RgbColor,
    /// Warnings, stars/favorites, attention needed
    pub yellow: RgbColor,
    /// Errors, destructive actions, uninstall confirmations
    pub red: RgbColor,
    /// Categories, tags, tool classifications
    pub mauve: RgbColor,
    /// Source badges (cargo, apt, npm, etc.)
    pub peach: RgbColor,
    /// Sparklines, metrics, usage graphs
    pub teal: RgbColor,
}

impl CustomTheme {
    /// Convert to runtime Theme
    pub fn to_theme(&self) -> Theme {
        Theme {
            name: "Custom",
            base: self.base.to_color(),
            surface0: self.surface0.to_color(),
            surface1: self.surface1.to_color(),
            text: self.text.to_color(),
            subtext0: self.subtext0.to_color(),
            blue: self.blue.to_color(),
            green: self.green.to_color(),
            yellow: self.yellow.to_color(),
            red: self.red.to_color(),
            mauve: self.mauve.to_color(),
            peach: self.peach.to_color(),
            teal: self.teal.to_color(),
        }
    }

    /// Create default custom theme (based on Catppuccin Mocha)
    pub fn default_template() -> Self {
        Self {
            schema: Some("https://raw.githubusercontent.com/chrisbataille/hoards/main/schema/custom-theme.schema.json".to_string()),
            name: "My Custom Theme".to_string(),
            base: RgbColor { r: 30, g: 30, b: 46 },
            surface0: RgbColor { r: 49, g: 50, b: 68 },
            surface1: RgbColor { r: 69, g: 71, b: 90 },
            text: RgbColor { r: 205, g: 214, b: 244 },
            subtext0: RgbColor { r: 166, g: 173, b: 200 },
            blue: RgbColor { r: 137, g: 180, b: 250 },
            green: RgbColor { r: 166, g: 227, b: 161 },
            yellow: RgbColor { r: 249, g: 226, b: 175 },
            red: RgbColor { r: 243, g: 139, b: 168 },
            mauve: RgbColor { r: 203, g: 166, b: 247 },
            peach: RgbColor { r: 250, g: 179, b: 135 },
            teal: RgbColor { r: 102, g: 178, b: 168 },
        }
    }

    /// Get custom theme file path
    pub fn file_path() -> Result<PathBuf> {
        crate::config::HoardConfig::config_dir().map(|d| d.join("custom-theme.json"))
    }

    /// Load custom theme from file
    pub fn load() -> Result<Self> {
        let path = Self::file_path()?;
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read custom theme from {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse custom theme from {}", path.display()))
    }

    /// Save custom theme to file
    pub fn save(&self) -> Result<()> {
        let path = Self::file_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Check if custom theme file exists
    pub fn exists() -> bool {
        Self::file_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Create default custom theme file if it doesn't exist
    pub fn create_default_if_missing() -> Result<bool> {
        if Self::exists() {
            return Ok(false);
        }
        Self::default_template().save()?;
        Ok(true)
    }
}

/// Global storage for loaded custom theme (supports runtime reloading)
static CUSTOM_THEME: RwLock<Option<Theme>> = RwLock::new(None);

/// Load and cache custom theme
fn get_custom_theme() -> Option<Theme> {
    // Try to read from cache first
    if let Ok(guard) = CUSTOM_THEME.read()
        && let Some(theme) = *guard
    {
        return Some(theme);
    }

    // Load from file and cache
    let theme = CustomTheme::load().ok().map(|ct| ct.to_theme());
    if let Ok(mut guard) = CUSTOM_THEME.write() {
        *guard = theme;
    }
    theme
}

/// Reload custom theme from file (call after file changes)
pub fn reload_custom_theme() -> Option<Theme> {
    let theme = CustomTheme::load().ok().map(|ct| ct.to_theme());
    if let Ok(mut guard) = CUSTOM_THEME.write() {
        *guard = theme;
    }
    theme
}

/// Available theme variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeVariant {
    #[default]
    CatppuccinMocha,
    CatppuccinLatte,
    Dracula,
    Nord,
    TokyoNight,
    Gruvbox,
    Custom,
}

impl ThemeVariant {
    /// Get the theme for this variant
    pub fn theme(&self) -> Theme {
        match self {
            Self::CatppuccinMocha => CATPPUCCIN_MOCHA,
            Self::CatppuccinLatte => CATPPUCCIN_LATTE,
            Self::Dracula => DRACULA,
            Self::Nord => NORD,
            Self::TokyoNight => TOKYO_NIGHT,
            Self::Gruvbox => GRUVBOX,
            Self::Custom => get_custom_theme().unwrap_or(CATPPUCCIN_MOCHA),
        }
    }

    /// Cycle to the next theme
    pub fn next(&self) -> Self {
        match self {
            Self::CatppuccinMocha => Self::CatppuccinLatte,
            Self::CatppuccinLatte => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::TokyoNight,
            Self::TokyoNight => Self::Gruvbox,
            Self::Gruvbox => {
                // Only show Custom option if custom theme file exists
                if CustomTheme::exists() {
                    Self::Custom
                } else {
                    Self::CatppuccinMocha
                }
            }
            Self::Custom => Self::CatppuccinMocha,
        }
    }

    /// Convert from config TuiTheme
    pub fn from_config_theme(theme: crate::config::TuiTheme) -> Self {
        use crate::config::TuiTheme;
        match theme {
            TuiTheme::CatppuccinMocha => Self::CatppuccinMocha,
            TuiTheme::CatppuccinLatte => Self::CatppuccinLatte,
            TuiTheme::Dracula => Self::Dracula,
            TuiTheme::Nord => Self::Nord,
            TuiTheme::TokyoNight => Self::TokyoNight,
            TuiTheme::Gruvbox => Self::Gruvbox,
            TuiTheme::Custom => Self::Custom,
        }
    }

    /// Convert to config TuiTheme
    pub fn to_config_theme(&self) -> crate::config::TuiTheme {
        use crate::config::TuiTheme;
        match self {
            Self::CatppuccinMocha => TuiTheme::CatppuccinMocha,
            Self::CatppuccinLatte => TuiTheme::CatppuccinLatte,
            Self::Dracula => TuiTheme::Dracula,
            Self::Nord => TuiTheme::Nord,
            Self::TokyoNight => TuiTheme::TokyoNight,
            Self::Gruvbox => TuiTheme::Gruvbox,
            Self::Custom => TuiTheme::Custom,
        }
    }

    /// Get all available variants
    pub fn all() -> &'static [ThemeVariant] {
        &[
            Self::CatppuccinMocha,
            Self::CatppuccinLatte,
            Self::Dracula,
            Self::Nord,
            Self::TokyoNight,
            Self::Gruvbox,
        ]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::TokyoNight => "Tokyo Night",
            Self::Gruvbox => "Gruvbox",
            Self::Custom => "Custom",
        }
    }
}

// ============================================================================
// Theme Definitions
// ============================================================================

/// Catppuccin Mocha - Dark theme with warm pastels
pub const CATPPUCCIN_MOCHA: Theme = Theme {
    name: "Catppuccin Mocha",
    base: Color::Rgb(30, 30, 46),
    surface0: Color::Rgb(49, 50, 68),
    surface1: Color::Rgb(69, 71, 90),
    text: Color::Rgb(205, 214, 244),
    subtext0: Color::Rgb(166, 173, 200),
    blue: Color::Rgb(137, 180, 250),
    green: Color::Rgb(166, 227, 161),
    yellow: Color::Rgb(249, 226, 175),
    red: Color::Rgb(243, 139, 168),
    mauve: Color::Rgb(203, 166, 247),
    peach: Color::Rgb(250, 179, 135),
    teal: Color::Rgb(102, 178, 168), // Dimmed from 148, 226, 213
};

/// Catppuccin Latte - Light theme with warm pastels
pub const CATPPUCCIN_LATTE: Theme = Theme {
    name: "Catppuccin Latte",
    base: Color::Rgb(239, 241, 245),
    surface0: Color::Rgb(220, 224, 232),
    surface1: Color::Rgb(188, 192, 204),
    text: Color::Rgb(76, 79, 105),
    subtext0: Color::Rgb(108, 111, 133),
    blue: Color::Rgb(30, 102, 245),
    green: Color::Rgb(64, 160, 43),
    yellow: Color::Rgb(223, 142, 29),
    red: Color::Rgb(210, 15, 57),
    mauve: Color::Rgb(136, 57, 239),
    peach: Color::Rgb(254, 100, 11),
    teal: Color::Rgb(23, 146, 153),
};

/// Dracula - Dark theme with vibrant colors
pub const DRACULA: Theme = Theme {
    name: "Dracula",
    base: Color::Rgb(40, 42, 54),
    surface0: Color::Rgb(68, 71, 90),
    surface1: Color::Rgb(98, 114, 164),
    text: Color::Rgb(248, 248, 242),
    subtext0: Color::Rgb(189, 147, 249),
    blue: Color::Rgb(139, 233, 253),
    green: Color::Rgb(80, 250, 123),
    yellow: Color::Rgb(241, 250, 140),
    red: Color::Rgb(255, 85, 85),
    mauve: Color::Rgb(189, 147, 249),
    peach: Color::Rgb(255, 184, 108),
    teal: Color::Rgb(98, 168, 182), // Dimmed from 139, 233, 253
};

/// Nord - Arctic, bluish color palette
pub const NORD: Theme = Theme {
    name: "Nord",
    base: Color::Rgb(46, 52, 64),
    surface0: Color::Rgb(59, 66, 82),
    surface1: Color::Rgb(76, 86, 106),
    text: Color::Rgb(236, 239, 244),
    subtext0: Color::Rgb(216, 222, 233),
    blue: Color::Rgb(136, 192, 208),
    green: Color::Rgb(163, 190, 140),
    yellow: Color::Rgb(235, 203, 139),
    red: Color::Rgb(191, 97, 106),
    mauve: Color::Rgb(180, 142, 173),
    peach: Color::Rgb(208, 135, 112),
    teal: Color::Rgb(143, 188, 187),
};

/// Tokyo Night - Dark theme inspired by Tokyo's night
pub const TOKYO_NIGHT: Theme = Theme {
    name: "Tokyo Night",
    base: Color::Rgb(26, 27, 38),
    surface0: Color::Rgb(36, 40, 59),
    surface1: Color::Rgb(65, 72, 104),
    text: Color::Rgb(192, 202, 245),
    subtext0: Color::Rgb(139, 147, 175),
    blue: Color::Rgb(122, 162, 247),
    green: Color::Rgb(158, 206, 106),
    yellow: Color::Rgb(224, 175, 104),
    red: Color::Rgb(247, 118, 142),
    mauve: Color::Rgb(187, 154, 247),
    peach: Color::Rgb(255, 158, 100),
    teal: Color::Rgb(82, 158, 146), // Dimmed from 115, 218, 202
};

/// Gruvbox - Retro groove color scheme
pub const GRUVBOX: Theme = Theme {
    name: "Gruvbox",
    base: Color::Rgb(40, 40, 40),
    surface0: Color::Rgb(60, 56, 54),
    surface1: Color::Rgb(80, 73, 69),
    text: Color::Rgb(235, 219, 178),
    subtext0: Color::Rgb(189, 174, 147),
    blue: Color::Rgb(131, 165, 152),
    green: Color::Rgb(184, 187, 38),
    yellow: Color::Rgb(250, 189, 47),
    red: Color::Rgb(251, 73, 52),
    mauve: Color::Rgb(211, 134, 155),
    peach: Color::Rgb(254, 128, 25),
    teal: Color::Rgb(142, 192, 124),
};
