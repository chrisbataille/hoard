//! Application state for the TUI

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::Update;
use crate::config::{AiProvider, HoardConfig, SourcesConfig, TuiTheme, UsageMode};
use crate::db::{Database, GitHubInfo, ToolUsage};
use crate::models::{Bundle, InstallSource, Tool};

/// A search result from the Discover tab
#[derive(Debug, Clone)]
pub struct DiscoverResult {
    pub name: String,
    pub description: Option<String>,
    pub source: DiscoverSource,
    pub stars: Option<u64>,
    pub url: Option<String>,
}

/// Source of a discover result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverSource {
    GitHub,
    CratesIo,
    PyPI,
    Npm,
    Apt,
    Homebrew,
    AI,
}

impl DiscoverSource {
    pub fn to_install_source(&self) -> InstallSource {
        match self {
            DiscoverSource::GitHub => InstallSource::Unknown,
            DiscoverSource::CratesIo => InstallSource::Cargo,
            DiscoverSource::PyPI => InstallSource::Pip,
            DiscoverSource::Npm => InstallSource::Npm,
            DiscoverSource::Apt => InstallSource::Apt,
            DiscoverSource::Homebrew => InstallSource::Brew,
            DiscoverSource::AI => InstallSource::Unknown,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DiscoverSource::GitHub => "\u{f09b}", //
            DiscoverSource::CratesIo => "🦀",
            DiscoverSource::PyPI => "🐍",
            DiscoverSource::Npm => "\u{e71e}", //
            DiscoverSource::Apt => "📦",
            DiscoverSource::Homebrew => "🍺",
            DiscoverSource::AI => "🤖",
        }
    }
}

/// Section of the config menu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigSection {
    #[default]
    AiProvider,
    Theme,
    Sources,
    UsageMode,
    Buttons, // Save/Cancel
}

impl ConfigSection {
    pub fn all() -> &'static [ConfigSection] {
        &[
            ConfigSection::AiProvider,
            ConfigSection::Theme,
            ConfigSection::Sources,
            ConfigSection::UsageMode,
            ConfigSection::Buttons,
        ]
    }

    pub fn next(&self) -> ConfigSection {
        match self {
            ConfigSection::AiProvider => ConfigSection::Theme,
            ConfigSection::Theme => ConfigSection::Sources,
            ConfigSection::Sources => ConfigSection::UsageMode,
            ConfigSection::UsageMode => ConfigSection::Buttons,
            ConfigSection::Buttons => ConfigSection::AiProvider,
        }
    }

    pub fn prev(&self) -> ConfigSection {
        match self {
            ConfigSection::AiProvider => ConfigSection::Buttons,
            ConfigSection::Theme => ConfigSection::AiProvider,
            ConfigSection::Sources => ConfigSection::Theme,
            ConfigSection::UsageMode => ConfigSection::Sources,
            ConfigSection::Buttons => ConfigSection::UsageMode,
        }
    }

    /// Get the starting line number for this section in the config menu.
    /// Used for click detection and auto-scroll.
    ///
    /// Layout (without custom theme description):
    /// - Lines 0-5: AI Provider (header + 5 options)
    /// - Line 6: empty
    /// - Lines 7-14: Theme (header + 7 options)
    /// - Line 15: empty
    /// - Lines 16-23: Sources (header + 7 options)
    /// - Line 24: empty
    /// - Lines 25-27: Usage (header + 2 options)
    /// - Line 28: empty
    /// - Line 29: Buttons
    pub fn start_line(&self, custom_theme_selected: bool) -> usize {
        let theme_extra = if custom_theme_selected { 1 } else { 0 };
        match self {
            Self::AiProvider => 0,
            Self::Theme => 7,
            Self::Sources => 16 + theme_extra,
            Self::UsageMode => 25 + theme_extra,
            Self::Buttons => 29 + theme_extra,
        }
    }

    /// Get the line range for items in this section (excluding header).
    /// Returns (first_item_line, last_item_line) inclusive.
    pub fn item_lines(&self, custom_theme_selected: bool) -> (usize, usize) {
        let theme_extra = if custom_theme_selected { 1 } else { 0 };
        match self {
            Self::AiProvider => (1, 5),                              // 5 AI providers
            Self::Theme => (8, 14),                                  // 7 themes (indices 0-6)
            Self::Sources => (17 + theme_extra, 23 + theme_extra),   // 7 sources
            Self::UsageMode => (26 + theme_extra, 27 + theme_extra), // 2 modes
            Self::Buttons => (29 + theme_extra, 29 + theme_extra),   // 1 line
        }
    }

    /// Number of selectable items in this section
    pub fn item_count(&self) -> usize {
        match self {
            Self::AiProvider => 5, // None, Claude, Gemini, Codex, Opencode
            Self::Theme => 7,      // 6 built-in + Custom
            Self::Sources => 7,    // cargo, apt, pip, npm, brew, flatpak, manual
            Self::UsageMode => 2,  // Scan, Hook
            Self::Buttons => 2,    // Save, Cancel
        }
    }
}

/// Config menu layout constants
pub mod config_menu_layout {
    /// Base number of lines in config menu (without custom theme description)
    pub const TOTAL_LINES_BASE: usize = 30;
    /// Extra line when custom theme is selected (for file path hint)
    pub const CUSTOM_THEME_EXTRA_LINES: usize = 1;
    /// Index of custom theme
    pub const CUSTOM_THEME_INDEX: usize = 6;

    /// Calculate total lines based on whether custom theme is selected
    pub fn total_lines(custom_theme_selected: bool) -> usize {
        if custom_theme_selected {
            TOTAL_LINES_BASE + CUSTOM_THEME_EXTRA_LINES
        } else {
            TOTAL_LINES_BASE
        }
    }
}

/// State for the config menu
#[derive(Debug, Clone)]
pub struct ConfigMenuState {
    /// Currently focused section
    pub section: ConfigSection,
    /// Selected index within current section (for radio buttons)
    pub ai_selected: usize,
    pub theme_selected: usize,
    pub usage_selected: usize,
    /// Source toggles (separate state for checkboxes)
    pub sources: SourcesConfig,
    /// Which source is focused (0-6)
    pub source_focused: usize,
    /// Button focus (0=Save, 1=Cancel)
    pub button_focused: usize,
    /// Scroll offset for the config menu content
    pub scroll_offset: usize,
}

impl Default for ConfigMenuState {
    fn default() -> Self {
        Self {
            section: ConfigSection::AiProvider,
            ai_selected: 0, // None
            theme_selected: 0,
            usage_selected: 0, // Scan
            sources: SourcesConfig::default(),
            source_focused: 0,
            button_focused: 0, // Save
            scroll_offset: 0,
        }
    }
}

impl ConfigMenuState {
    /// Initialize from existing config
    pub fn from_config(config: &HoardConfig) -> Self {
        Self {
            section: ConfigSection::AiProvider,
            ai_selected: AiProvider::all()
                .iter()
                .position(|p| *p == config.ai.provider)
                .unwrap_or(0),
            theme_selected: config.tui.theme.index(),
            usage_selected: match config.usage.mode {
                UsageMode::Scan => 0,
                UsageMode::Hook => 1,
            },
            sources: config.sources.clone(),
            source_focused: 0,
            button_focused: 0,
            scroll_offset: 0,
        }
    }

    /// Build config from current state
    pub fn to_config(&self) -> HoardConfig {
        let mut config = HoardConfig::default();
        config.ai.provider = AiProvider::all()[self.ai_selected];
        config.tui.theme = TuiTheme::from_index(self.theme_selected);
        config.usage.mode = if self.usage_selected == 0 {
            UsageMode::Scan
        } else {
            UsageMode::Hook
        };
        config.sources = self.sources.clone();
        config
    }

    /// Move to next item in current section
    pub fn next_item(&mut self) {
        let count = self.section.item_count();
        match self.section {
            ConfigSection::AiProvider => {
                self.ai_selected = (self.ai_selected + 1) % count;
            }
            ConfigSection::Theme => {
                self.theme_selected = (self.theme_selected + 1) % count;
            }
            ConfigSection::Sources => {
                self.source_focused = (self.source_focused + 1) % count;
            }
            ConfigSection::UsageMode => {
                self.usage_selected = (self.usage_selected + 1) % count;
            }
            ConfigSection::Buttons => {
                self.button_focused = (self.button_focused + 1) % count;
            }
        }
    }

    /// Move to prev item in current section
    pub fn prev_item(&mut self) {
        let count = self.section.item_count();
        match self.section {
            ConfigSection::AiProvider => {
                self.ai_selected = if self.ai_selected == 0 {
                    count - 1
                } else {
                    self.ai_selected - 1
                };
            }
            ConfigSection::Theme => {
                self.theme_selected = if self.theme_selected == 0 {
                    count - 1
                } else {
                    self.theme_selected - 1
                };
            }
            ConfigSection::Sources => {
                self.source_focused = if self.source_focused == 0 {
                    count - 1
                } else {
                    self.source_focused - 1
                };
            }
            ConfigSection::UsageMode => {
                self.usage_selected = if self.usage_selected == 0 {
                    count - 1
                } else {
                    self.usage_selected - 1
                };
            }
            ConfigSection::Buttons => {
                self.button_focused = if self.button_focused == 0 {
                    count - 1
                } else {
                    self.button_focused - 1
                };
            }
        }
    }

    /// Toggle the current source checkbox (only for Sources section)
    pub fn toggle_current_source(&mut self) {
        if self.section == ConfigSection::Sources {
            let sources = SourcesConfig::all_sources();
            if self.source_focused < sources.len() {
                self.sources.toggle(sources[self.source_focused]);
            }
        }
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down by one line (with max limit)
    pub fn scroll_down(&mut self, max_scroll: usize) {
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }
}

/// Fuzzy match a query against a target string (fzf-style)
/// Returns Some(score) if matches, None if no match
/// Higher scores = better matches
fn fuzzy_match(query: &str, target: &str) -> Option<i32> {
    let query = query.to_lowercase();
    let target = target.to_lowercase();

    if query.is_empty() {
        return Some(0);
    }

    let query_chars: Vec<char> = query.chars().collect();
    let target_chars: Vec<char> = target.chars().collect();

    let mut query_idx = 0;
    let mut score = 0i32;
    let mut prev_match_idx: Option<usize> = None;
    let mut consecutive_bonus = 0i32;

    for (target_idx, &tc) in target_chars.iter().enumerate() {
        if query_idx < query_chars.len() && tc == query_chars[query_idx] {
            // Character matched
            score += 1;

            // Bonus for consecutive matches
            if let Some(prev) = prev_match_idx {
                if target_idx == prev + 1 {
                    consecutive_bonus += 2;
                    score += consecutive_bonus;
                } else {
                    consecutive_bonus = 0;
                }
            }

            // Bonus for matching at word boundaries
            if target_idx == 0
                || target_chars
                    .get(target_idx.wrapping_sub(1))
                    .map(|c| !c.is_alphanumeric())
                    .unwrap_or(true)
            {
                score += 3;
            }

            prev_match_idx = Some(target_idx);
            query_idx += 1;
        }
    }

    // All query characters must match
    if query_idx == query_chars.len() {
        // Bonus for exact match
        if query == target {
            score += 100;
        }
        // Bonus for prefix match
        else if target.starts_with(&query) {
            score += 50;
        }
        Some(score)
    } else {
        None
    }
}

/// Fuzzy match returning matched character positions for highlighting
/// Returns (score, positions) if matches, None if no match
pub fn fuzzy_match_positions(query: &str, target: &str) -> Option<(i32, Vec<usize>)> {
    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    if query_lower.is_empty() {
        return Some((0, vec![]));
    }

    let query_chars: Vec<char> = query_lower.chars().collect();
    let target_chars: Vec<char> = target_lower.chars().collect();

    let mut query_idx = 0;
    let mut score = 0i32;
    let mut prev_match_idx: Option<usize> = None;
    let mut consecutive_bonus = 0i32;
    let mut positions = Vec::new();

    for (target_idx, &tc) in target_chars.iter().enumerate() {
        if query_idx < query_chars.len() && tc == query_chars[query_idx] {
            positions.push(target_idx);
            score += 1;

            if let Some(prev) = prev_match_idx {
                if target_idx == prev + 1 {
                    consecutive_bonus += 2;
                    score += consecutive_bonus;
                } else {
                    consecutive_bonus = 0;
                }
            }

            if target_idx == 0
                || target_chars
                    .get(target_idx.wrapping_sub(1))
                    .map(|c| !c.is_alphanumeric())
                    .unwrap_or(true)
            {
                score += 3;
            }

            prev_match_idx = Some(target_idx);
            query_idx += 1;
        }
    }

    if query_idx == query_chars.len() {
        if query_lower == target_lower {
            score += 100;
        } else if target_lower.starts_with(&query_lower) {
            score += 50;
        }
        Some((score, positions))
    } else {
        None
    }
}

/// Available tabs in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Installed,
    Available,
    Updates,
    Bundles,
    Discover,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Installed,
            Tab::Available,
            Tab::Updates,
            Tab::Bundles,
            Tab::Discover,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Installed => "Installed",
            Tab::Available => "Available",
            Tab::Updates => "Updates",
            Tab::Bundles => "Bundles",
            Tab::Discover => "Discover",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Installed => 0,
            Tab::Available => 1,
            Tab::Updates => 2,
            Tab::Bundles => 3,
            Tab::Discover => 4,
        }
    }

    pub fn from_index(index: usize) -> Option<Tab> {
        match index {
            0 => Some(Tab::Installed),
            1 => Some(Tab::Available),
            2 => Some(Tab::Updates),
            3 => Some(Tab::Bundles),
            4 => Some(Tab::Discover),
            _ => None,
        }
    }
}

/// Input mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
    Command,      // Vim-style command palette with ':'
    JumpToLetter, // Waiting for letter input to jump to
}

/// Background operation that needs loading indicator
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundOp {
    CheckUpdates { step: usize },
}

impl BackgroundOp {
    pub fn title(&self) -> &'static str {
        match self {
            BackgroundOp::CheckUpdates { .. } => "Checking for Updates",
        }
    }
}

/// Progress information for loading overlay
#[derive(Debug, Clone, Default)]
pub struct LoadingProgress {
    pub current_step: usize,
    pub total_steps: usize,
    pub step_name: String,
    pub found_count: usize,
}

/// Package manager info for update checking
const PACKAGE_MANAGERS: &[(&str, &str)] = &[
    ("cargo", "Cargo (Rust)"),
    ("pip", "pip (Python)"),
    ("npm", "npm (Node.js)"),
    ("apt", "apt (Debian/Ubuntu)"),
    ("brew", "Homebrew"),
];

/// Pending action requiring confirmation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    Install(Vec<String>),   // Tool names to install
    Uninstall(Vec<String>), // Tool names to uninstall
    Update(Vec<String>),    // Tool names to update
}

/// Undoable action for history
#[derive(Debug, Clone)]
pub enum UndoableAction {
    /// Selection change (stores previous selection state)
    Selection(HashSet<String>),
    /// Filter/search change (stores previous query)
    Filter(String),
    /// Tab switch (stores previous tab)
    TabSwitch(Tab),
    /// Sort change (stores previous sort)
    Sort(SortBy),
}

/// Action history for undo/redo
#[derive(Debug, Default)]
pub struct ActionHistory {
    undo_stack: Vec<UndoableAction>,
    redo_stack: Vec<UndoableAction>,
    max_size: usize,
}

impl ActionHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    /// Push an action to the undo stack
    pub fn push(&mut self, action: UndoableAction) {
        if self.undo_stack.len() >= self.max_size {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
        self.redo_stack.clear(); // Clear redo on new action
    }

    /// Pop an action for undo
    pub fn pop_undo(&mut self) -> Option<UndoableAction> {
        self.undo_stack.pop()
    }

    /// Push to redo stack
    pub fn push_redo(&mut self, action: UndoableAction) {
        if self.redo_stack.len() >= self.max_size {
            self.redo_stack.remove(0);
        }
        self.redo_stack.push(action);
    }

    /// Pop an action for redo
    pub fn pop_redo(&mut self) -> Option<UndoableAction> {
        self.redo_stack.pop()
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

impl PendingAction {
    pub fn description(&self) -> String {
        match self {
            PendingAction::Install(tools) => {
                if tools.len() == 1 {
                    format!("Install {}?", tools[0])
                } else {
                    format!("Install {} tools?", tools.len())
                }
            }
            PendingAction::Uninstall(tools) => {
                if tools.len() == 1 {
                    format!("Uninstall {}?", tools[0])
                } else {
                    format!("Uninstall {} tools?", tools.len())
                }
            }
            PendingAction::Update(tools) => {
                if tools.len() == 1 {
                    format!("Update {}?", tools[0])
                } else {
                    format!("Update {} tools?", tools.len())
                }
            }
        }
    }

    pub fn tools(&self) -> &[String] {
        match self {
            PendingAction::Install(t) | PendingAction::Uninstall(t) | PendingAction::Update(t) => t,
        }
    }
}

/// Status message to display temporarily
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

/// Sort options for tool list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    Name,
    Usage,
    Recent,
}

impl SortBy {
    pub fn next(&self) -> SortBy {
        match self {
            SortBy::Name => SortBy::Usage,
            SortBy::Usage => SortBy::Recent,
            SortBy::Recent => SortBy::Name,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SortBy::Name => "name",
            SortBy::Usage => "usage",
            SortBy::Recent => "recent",
        }
    }
}

/// Available commands for the command palette with descriptions
pub const COMMANDS: &[(&str, &str)] = &[
    ("q", "quit - exit the application"),
    ("quit", "quit - exit the application"),
    ("exit", "exit the application"),
    ("h", "help - show help"),
    ("help", "show help dialog"),
    ("r", "refresh - reload tools"),
    ("refresh", "reload tools from database"),
    ("t", "theme [name] - cycle or set theme"),
    ("theme", "theme [name] - cycle or set theme"),
    ("s", "sort [field] - cycle or set sort"),
    (
        "sort",
        "sort [field] - cycle or set sort (name/usage/recent)",
    ),
    (
        "filter",
        "filter [source] - filter by source (cargo/apt/pip/npm)",
    ),
    ("source", "source [name] - filter by source"),
    ("src", "src [name] - filter by source"),
    ("fav", "fav - toggle favorites filter"),
    ("favorites", "favorites - toggle favorites filter"),
    ("starred", "starred - toggle favorites filter"),
    ("1", "go to Installed tab"),
    ("installed", "go to Installed tab"),
    ("2", "go to Available tab"),
    ("available", "go to Available tab"),
    ("3", "go to Updates tab"),
    ("updates", "go to Updates tab"),
    ("4", "go to Bundles tab"),
    ("bundles", "go to Bundles tab"),
    ("5", "go to Discover tab"),
    ("discover", "go to Discover tab"),
    ("i", "install selected item"),
    ("install", "install selected tool/bundle"),
    ("d", "delete/uninstall selected"),
    ("delete", "delete selected tool"),
    ("uninstall", "uninstall selected tool"),
    ("u", "update selected"),
    ("update", "update selected tool"),
    ("upgrade", "upgrade selected tool"),
    ("undo", "undo last action"),
    ("z", "undo last action"),
    ("redo", "redo undone action"),
    ("y", "redo undone action"),
    ("c", "config - open configuration menu"),
    ("config", "open configuration menu"),
    ("settings", "open configuration menu"),
    ("cfg", "open configuration menu"),
    ("create-theme", "create custom theme file"),
    ("new-theme", "create custom theme file"),
    ("edit-theme", "show custom theme file path"),
];

/// Main application state
pub struct App {
    pub running: bool,
    pub tab: Tab,
    pub input_mode: InputMode,
    pub search_query: String,
    pub source_filter: Option<String>, // Filter by source (cargo, apt, etc.)
    pub favorites_only: bool,          // Filter to show only favorites
    pub command_input: String,         // Command palette input (after ':')

    // Tool list state
    pub all_tools: Vec<Tool>, // All tools for current tab (unfiltered)
    pub tools: Vec<Tool>,     // Filtered/sorted tools to display
    pub selected_index: usize,
    pub list_offset: usize,

    // Cached data
    pub usage_data: HashMap<String, ToolUsage>,
    pub daily_usage: HashMap<String, Vec<i64>>, // 7-day usage for sparklines
    pub github_cache: HashMap<String, GitHubInfo>,
    pub labels_cache: HashMap<String, Vec<String>>, // tool_name -> labels

    // Updates state
    pub available_updates: HashMap<String, Update>,
    pub updates_checked: bool,
    pub updates_loading: bool,

    // Bundle list state (for Bundles tab)
    pub bundles: Vec<Bundle>,
    pub bundle_selected: usize,

    // UI state
    pub show_help: bool,
    pub show_details_popup: bool,
    pub sort_by: SortBy,
    pub theme_variant: super::theme::ThemeVariant,

    // Multi-selection
    pub selected_tools: HashSet<String>,

    // Actions
    pub pending_action: Option<PendingAction>,
    pub status_message: Option<StatusMessage>,

    // Background operations (executed by main loop with loading indicator)
    pub background_op: Option<BackgroundOp>,
    pub loading_progress: LoadingProgress,

    // Undo/redo history
    pub history: ActionHistory,

    // Command history (for ↑/↓ navigation)
    pub command_history: Vec<String>,
    pub command_history_index: Option<usize>,
    pub command_history_temp: String, // Temporary storage for current input when navigating

    // Mouse interaction state
    pub last_list_area: Option<(u16, u16, u16, u16)>, // (x, y, width, height) of tool list
    pub last_tab_area: Option<(u16, u16, u16, u16)>,  // (x, y, width, height) of tabs
    pub last_config_popup_area: Option<(u16, u16, u16, u16)>, // (x, y, width, height) of config popup

    // Feature availability status (for footer display)
    pub ai_available: bool, // AI provider is configured
    pub gh_available: bool, // GitHub CLI is installed

    // Last sync timestamp
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,

    // Discover tab state
    pub discover_query: String,
    pub discover_results: Vec<DiscoverResult>,
    pub discover_selected: usize,
    pub discover_loading: bool,

    // Config menu state
    pub show_config_menu: bool,
    pub config_menu: ConfigMenuState,
}

impl App {
    pub fn new(db: &Database) -> Result<Self> {
        let all_tools = db.list_tools(true, None)?; // installed only
        let bundles = db.list_bundles()?;

        // Load usage data
        let usage_data: HashMap<String, ToolUsage> = db.get_all_usage()?.into_iter().collect();

        // Load 7-day daily usage for sparklines
        let daily_usage = db.get_all_daily_usage(7).unwrap_or_default();

        // Preload GitHub info for stars display
        let github_cache: HashMap<String, GitHubInfo> = db
            .get_all_github_info()
            .unwrap_or_default()
            .into_iter()
            .collect();

        // Preload labels for all tools
        let labels_cache = db.get_all_tool_labels().unwrap_or_default();

        let tools = all_tools.clone();

        // Load config and check feature availability
        let config_exists = HoardConfig::exists();
        let config = HoardConfig::load().unwrap_or_default();
        let ai_available = config.ai.provider != AiProvider::None;
        let gh_available = which::which("gh").is_ok();

        // Get theme from config
        let theme_variant = super::theme::ThemeVariant::from_config_theme(config.tui.theme);

        // Auto-show config menu if no config file exists
        let show_config_menu = !config_exists;
        let config_menu = if show_config_menu {
            ConfigMenuState::from_config(&config)
        } else {
            ConfigMenuState::default()
        };

        Ok(Self {
            running: true,
            tab: Tab::Installed,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            source_filter: None,
            favorites_only: false,
            command_input: String::new(),
            all_tools,
            tools,
            selected_index: 0,
            list_offset: 0,
            usage_data,
            daily_usage,
            github_cache,
            labels_cache,
            available_updates: HashMap::new(),
            updates_checked: false,
            updates_loading: false,
            bundles,
            bundle_selected: 0,
            show_help: false,
            show_details_popup: false,
            sort_by: SortBy::default(),
            theme_variant,
            selected_tools: HashSet::new(),
            pending_action: None,
            status_message: None,
            background_op: None,
            loading_progress: LoadingProgress::default(),
            history: ActionHistory::new(50), // Keep 50 actions max
            command_history: Vec::new(),
            command_history_index: None,
            command_history_temp: String::new(),
            last_list_area: None,
            last_tab_area: None,
            last_config_popup_area: None,
            ai_available,
            gh_available,
            last_sync: db.get_last_sync_time().ok().flatten(),
            discover_query: String::new(),
            discover_results: Vec::new(),
            discover_selected: 0,
            discover_loading: false,
            show_config_menu,
            config_menu,
        })
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Cycle to the next theme
    pub fn cycle_theme(&mut self) {
        self.theme_variant = self.theme_variant.next();
        self.set_status(
            format!("Theme: {}", self.theme_variant.display_name()),
            false,
        );
    }

    /// Get the current theme
    pub fn theme(&self) -> super::theme::Theme {
        self.theme_variant.theme()
    }

    /// Switch to a specific tab
    pub fn switch_tab(&mut self, tab: Tab, db: &Database) {
        if self.tab != tab {
            self.tab = tab;
            self.selected_index = 0;
            self.list_offset = 0;
            self.search_query.clear();
            self.refresh_tools(db);
        }
    }

    /// Go to next tab
    pub fn next_tab(&mut self, db: &Database) {
        let next_index = (self.tab.index() + 1) % Tab::all().len();
        if let Some(tab) = Tab::from_index(next_index) {
            self.switch_tab(tab, db);
        }
    }

    /// Go to previous tab
    pub fn prev_tab(&mut self, db: &Database) {
        let prev_index = if self.tab.index() == 0 {
            Tab::all().len() - 1
        } else {
            self.tab.index() - 1
        };
        if let Some(tab) = Tab::from_index(prev_index) {
            self.switch_tab(tab, db);
        }
    }

    /// Refresh tool list based on current tab
    pub fn refresh_tools(&mut self, db: &Database) {
        let result = match self.tab {
            Tab::Installed => db.list_tools(true, None),
            Tab::Available => db.list_tools(false, None),
            Tab::Updates => {
                // For Updates tab, only show tools with available updates
                if self.updates_checked {
                    let update_names: HashSet<_> = self.available_updates.keys().cloned().collect();
                    db.list_tools(true, None).map(|mut tools| {
                        tools.retain(|t| update_names.contains(&t.name));
                        tools
                    })
                } else {
                    // No updates checked yet, show empty list
                    Ok(Vec::new())
                }
            }
            Tab::Bundles => db.list_tools(true, None),
            Tab::Discover => Ok(Vec::new()), // Discover has its own search results
        };

        if let Ok(mut tools) = result {
            // For Available tab, filter to only non-installed tools
            if self.tab == Tab::Available {
                tools.retain(|t| !t.is_installed);
            }
            self.all_tools = tools;
            self.apply_filter_and_sort();
        }

        // Also refresh bundles if on that tab
        if self.tab == Tab::Bundles
            && let Ok(bundles) = db.list_bundles()
        {
            self.bundles = bundles;
        }
    }

    /// Get update info for a tool if available
    pub fn get_update(&self, tool_name: &str) -> Option<&Update> {
        self.available_updates.get(tool_name)
    }

    /// Apply current search filter and sort to tools
    pub fn apply_filter_and_sort(&mut self) {
        // Start with all tools, optionally filtered by source and favorites
        let source_filtered: Vec<&Tool> = self
            .all_tools
            .iter()
            .filter(|t| {
                // Filter by source if set
                if let Some(ref source) = self.source_filter
                    && format!("{:?}", t.source).to_lowercase() != *source
                {
                    return false;
                }
                // Filter by favorites if enabled
                if self.favorites_only && !t.is_favorite {
                    return false;
                }
                true
            })
            .collect();

        // Apply fuzzy search filter
        let mut filtered: Vec<(Tool, i32)> = if self.search_query.is_empty() {
            source_filtered
                .into_iter()
                .map(|t| (t.clone(), 0))
                .collect()
        } else {
            // Fuzzy match against name, description, and category
            source_filtered
                .into_iter()
                .filter_map(|t| {
                    // Get best score across all fields
                    let name_score = fuzzy_match(&self.search_query, &t.name);
                    let desc_score = t
                        .description
                        .as_ref()
                        .and_then(|d| fuzzy_match(&self.search_query, d));
                    let cat_score = t
                        .category
                        .as_ref()
                        .and_then(|c| fuzzy_match(&self.search_query, c));

                    // Use best score (name matches get priority bonus)
                    let score = [
                        name_score.map(|s| s + 10), // Bonus for name match
                        desc_score,
                        cat_score,
                    ]
                    .into_iter()
                    .flatten()
                    .max();

                    score.map(|s| (t.clone(), s))
                })
                .collect()
        };

        // Sort by fuzzy score when searching, otherwise by user preference
        if !self.search_query.is_empty() {
            // Sort by score descending (best matches first)
            filtered.sort_by(|a, b| b.1.cmp(&a.1));
        } else {
            // Sort by user preference
            match self.sort_by {
                SortBy::Name => filtered.sort_by(|a, b| a.0.name.cmp(&b.0.name)),
                SortBy::Usage => {
                    let usage = &self.usage_data;
                    filtered.sort_by(|a, b| {
                        let a_usage = usage.get(&a.0.name).map(|u| u.use_count).unwrap_or(0);
                        let b_usage = usage.get(&b.0.name).map(|u| u.use_count).unwrap_or(0);
                        b_usage.cmp(&a_usage) // Descending
                    });
                }
                SortBy::Recent => {
                    filtered.sort_by(|a, b| b.0.updated_at.cmp(&a.0.updated_at));
                }
            }
        }

        self.tools = filtered.into_iter().map(|(t, _)| t).collect();

        // Adjust selection if needed
        if self.selected_index >= self.tools.len() {
            self.selected_index = self.tools.len().saturating_sub(1);
        }
    }

    /// Cycle through sort options
    pub fn cycle_sort(&mut self) {
        self.sort_by = self.sort_by.next();
        self.apply_filter_and_sort();
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.tools.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.tools.len() - 1);
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// Move to next match with wrapping (vim n)
    pub fn search_next(&mut self) {
        if self.tools.is_empty() {
            return;
        }
        // Move to next item, wrap to start if at end
        if self.selected_index + 1 >= self.tools.len() {
            self.selected_index = 0;
            self.set_status("Search wrapped to top".to_string(), false);
        } else {
            self.selected_index += 1;
        }
    }

    /// Move to previous match with wrapping (vim N)
    pub fn search_prev(&mut self) {
        if self.tools.is_empty() {
            return;
        }
        // Move to previous item, wrap to end if at start
        if self.selected_index == 0 {
            self.selected_index = self.tools.len() - 1;
            self.set_status("Search wrapped to bottom".to_string(), false);
        } else {
            self.selected_index -= 1;
        }
    }

    /// Enter jump-to-letter mode (vim f)
    pub fn enter_jump_mode(&mut self) {
        self.input_mode = InputMode::JumpToLetter;
    }

    /// Exit jump-to-letter mode
    pub fn exit_jump_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Jump to first tool starting with the given letter
    pub fn jump_to_letter(&mut self, letter: char) {
        let letter = letter.to_ascii_lowercase();
        for (i, tool) in self.tools.iter().enumerate() {
            if tool.name.to_lowercase().starts_with(letter) {
                self.selected_index = i;
                self.set_status(format!("Jumped to '{}'", letter), false);
                break;
            }
        }
        self.exit_jump_mode();
    }

    /// Toggle favorite status for the selected tool
    pub fn toggle_favorite(&mut self, db: &Database) {
        if let Some(tool) = self.selected_tool() {
            let name = tool.name.clone();
            let new_status = !tool.is_favorite;

            match db.set_tool_favorite(&name, new_status) {
                Ok(true) => {
                    // Update local state
                    for t in &mut self.all_tools {
                        if t.name == name {
                            t.is_favorite = new_status;
                            break;
                        }
                    }
                    for t in &mut self.tools {
                        if t.name == name {
                            t.is_favorite = new_status;
                            break;
                        }
                    }
                    let status = if new_status {
                        "★ Added to favorites"
                    } else {
                        "Removed from favorites"
                    };
                    self.set_status(format!("{}: {}", name, status), false);
                }
                Ok(false) => {
                    self.set_status(format!("Tool not found: {}", name), true);
                }
                Err(e) => {
                    self.set_status(format!("Failed to update favorite: {}", e), true);
                }
            }
        }
    }

    /// Move selection to top
    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    /// Move selection to bottom
    pub fn select_last(&mut self) {
        if !self.tools.is_empty() {
            self.selected_index = self.tools.len() - 1;
        }
    }

    // ==================== Bundle Navigation ====================

    /// Move bundle selection down
    pub fn select_next_bundle(&mut self) {
        if !self.bundles.is_empty() {
            self.bundle_selected = (self.bundle_selected + 1).min(self.bundles.len() - 1);
        }
    }

    /// Move bundle selection up
    pub fn select_prev_bundle(&mut self) {
        self.bundle_selected = self.bundle_selected.saturating_sub(1);
    }

    /// Move bundle selection to top
    pub fn select_first_bundle(&mut self) {
        self.bundle_selected = 0;
    }

    /// Move bundle selection to bottom
    pub fn select_last_bundle(&mut self) {
        if !self.bundles.is_empty() {
            self.bundle_selected = self.bundles.len() - 1;
        }
    }

    /// Get the currently selected bundle
    pub fn selected_bundle(&self) -> Option<&Bundle> {
        self.bundles.get(self.bundle_selected)
    }

    /// Get the currently selected tool
    pub fn selected_tool(&self) -> Option<&Tool> {
        self.tools.get(self.selected_index)
    }

    /// Get usage for a tool
    pub fn get_usage(&self, tool_name: &str) -> Option<&ToolUsage> {
        self.usage_data.get(tool_name)
    }

    /// Get GitHub info for a tool (cached, or fetch from db)
    pub fn get_github_info(&mut self, tool_name: &str, db: &Database) -> Option<&GitHubInfo> {
        if !self.github_cache.contains_key(tool_name)
            && let Ok(Some(info)) = db.get_github_info(tool_name)
        {
            self.github_cache.insert(tool_name.to_string(), info);
        }
        self.github_cache.get(tool_name)
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Open config menu
    pub fn open_config_menu(&mut self) {
        // Load current config and initialize menu state
        if let Ok(config) = HoardConfig::load() {
            self.config_menu = ConfigMenuState::from_config(&config);
        } else {
            self.config_menu = ConfigMenuState::default();
        }
        self.show_config_menu = true;
    }

    /// Close config menu without saving
    pub fn close_config_menu(&mut self) {
        self.show_config_menu = false;
    }

    /// Save config from menu and close
    pub fn save_config_menu(&mut self) {
        let config = self.config_menu.to_config();

        // Apply theme immediately
        self.theme_variant = super::theme::ThemeVariant::from_config_theme(config.tui.theme);

        // Update AI availability
        self.ai_available = config.ai.provider != AiProvider::None;

        // Save to file
        if let Err(e) = config.save() {
            self.set_status(format!("Failed to save config: {}", e), true);
        } else {
            self.set_status("Configuration saved".to_string(), false);
        }

        self.show_config_menu = false;
    }

    /// Navigate config menu sections (with auto-scroll)
    pub fn config_menu_next_section(&mut self) {
        self.config_menu.section = self.config_menu.section.next();
        self.scroll_to_config_section();
    }

    pub fn config_menu_prev_section(&mut self) {
        self.config_menu.section = self.config_menu.section.prev();
        self.scroll_to_config_section();
    }

    /// Scroll config menu to make current section visible
    fn scroll_to_config_section(&mut self) {
        use config_menu_layout::CUSTOM_THEME_INDEX;
        let custom_selected = self.config_menu.theme_selected == CUSTOM_THEME_INDEX;
        let section_line = self.config_menu.section.start_line(custom_selected);
        // Cap scroll to keep buttons visible (don't scroll past ~25 lines)
        self.config_menu.scroll_offset = section_line.min(25);
    }

    /// Navigate items within config menu section
    pub fn config_menu_next_item(&mut self) {
        self.config_menu.next_item();
    }

    pub fn config_menu_prev_item(&mut self) {
        self.config_menu.prev_item();
    }

    /// Toggle source in config menu
    pub fn config_menu_toggle_source(&mut self) {
        self.config_menu.toggle_current_source();
    }

    /// Scroll config menu up
    pub fn config_menu_scroll_up(&mut self) {
        self.config_menu.scroll_up();
    }

    /// Scroll config menu down (pass total_lines from UI)
    pub fn config_menu_scroll_down(&mut self, total_lines: usize, visible_lines: usize) {
        let max_scroll = total_lines.saturating_sub(visible_lines);
        self.config_menu.scroll_down(max_scroll);
    }

    /// Get config menu scroll offset
    pub fn config_menu_scroll_offset(&self) -> usize {
        self.config_menu.scroll_offset
    }

    /// Handle Enter key in config menu
    pub fn config_menu_select(&mut self) {
        match self.config_menu.section {
            ConfigSection::Buttons => {
                if self.config_menu.button_focused == 0 {
                    // Save
                    self.save_config_menu();
                } else {
                    // Cancel
                    self.close_config_menu();
                }
            }
            ConfigSection::Sources => {
                // Toggle the current source
                self.config_menu.toggle_current_source();
            }
            _ => {
                // For radio button sections, the current selection is already the value
                // Move to next section
                self.config_menu.section = self.config_menu.section.next();
            }
        }
    }

    /// Check if config menu should auto-launch (no config file exists)
    pub fn should_show_config_on_start() -> bool {
        !HoardConfig::exists()
    }

    /// Enter search mode
    pub fn enter_search(&mut self) {
        self.record_filter(); // Record current filter for undo
        self.input_mode = InputMode::Search;
        self.search_query.clear();
    }

    /// Exit search mode
    pub fn exit_search(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Add character to search query and filter
    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_filter_and_sort();
    }

    /// Remove last character from search query and filter
    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.apply_filter_and_sort();
    }

    /// Clear search and show all tools
    pub fn clear_search(&mut self) {
        if !self.search_query.is_empty() {
            self.record_filter(); // Record for undo
            self.search_query.clear();
            self.apply_filter_and_sort();
        }
    }

    // ==================== Command Palette ====================

    /// Enter command mode (vim-style ':')
    pub fn enter_command(&mut self) {
        self.input_mode = InputMode::Command;
        self.command_input.clear();
        self.reset_command_history_nav();
    }

    /// Exit command mode
    pub fn exit_command(&mut self) {
        self.input_mode = InputMode::Normal;
        self.command_input.clear();
    }

    /// Add character to command input
    pub fn command_push(&mut self, c: char) {
        self.command_input.push(c);
    }

    /// Remove last character from command input
    pub fn command_pop(&mut self) {
        self.command_input.pop();
    }

    /// Get command suggestions based on current input
    pub fn get_command_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        let input = self.command_input.trim().to_lowercase();
        if input.is_empty() {
            return Vec::new();
        }

        COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(&input))
            .take(5) // Limit to 5 suggestions
            .copied()
            .collect()
    }

    /// Autocomplete the current command with the first suggestion
    pub fn autocomplete_command(&mut self) {
        let suggestions = self.get_command_suggestions();
        if let Some((cmd, _)) = suggestions.first() {
            self.command_input = cmd.to_string();
        }
    }

    /// Navigate to previous command in history (Up arrow)
    pub fn command_history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        match self.command_history_index {
            None => {
                // Save current input and go to most recent history
                self.command_history_temp = self.command_input.clone();
                self.command_history_index = Some(self.command_history.len() - 1);
            }
            Some(0) => {
                // Already at oldest, stay there
                return;
            }
            Some(idx) => {
                self.command_history_index = Some(idx - 1);
            }
        }

        if let Some(idx) = self.command_history_index {
            self.command_input = self.command_history[idx].clone();
        }
    }

    /// Navigate to next command in history (Down arrow)
    pub fn command_history_next(&mut self) {
        match self.command_history_index {
            None => {
                // Not navigating history, do nothing
            }
            Some(idx) if idx + 1 >= self.command_history.len() => {
                // Return to current input
                self.command_history_index = None;
                self.command_input = self.command_history_temp.clone();
            }
            Some(idx) => {
                self.command_history_index = Some(idx + 1);
                self.command_input = self.command_history[idx + 1].clone();
            }
        }
    }

    /// Add command to history (called after successful execution)
    fn add_to_command_history(&mut self, cmd: &str) {
        let cmd = cmd.trim().to_string();
        if cmd.is_empty() {
            return;
        }

        // Avoid duplicates at the end
        if self.command_history.last() != Some(&cmd) {
            self.command_history.push(cmd);
            // Limit history size
            if self.command_history.len() > 50 {
                self.command_history.remove(0);
            }
        }
    }

    /// Reset history navigation state (called when entering command mode)
    pub fn reset_command_history_nav(&mut self) {
        self.command_history_index = None;
        self.command_history_temp.clear();
    }

    /// Execute the current command
    pub fn execute_command(&mut self, db: &Database) {
        let cmd = self.command_input.trim().to_lowercase();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            self.exit_command();
            return;
        }

        // Add to command history
        self.add_to_command_history(&cmd);

        match parts[0] {
            // Quit commands
            "q" | "quit" | "exit" => self.quit(),

            // Help
            "h" | "help" => {
                self.show_help = true;
                self.exit_command();
            }

            // Refresh
            "r" | "refresh" => {
                self.refresh_tools(db);
                self.exit_command();
            }

            // Theme commands
            "theme" | "t" => {
                if parts.len() > 1 {
                    self.set_theme_by_name(parts[1]);
                } else {
                    self.cycle_theme();
                }
                self.exit_command();
            }

            // Sort commands
            "sort" | "s" => {
                if parts.len() > 1 {
                    self.set_sort_by_name(parts[1]);
                } else {
                    self.cycle_sort();
                }
                self.exit_command();
            }

            // Source filter commands
            "filter" | "source" | "src" => {
                if parts.len() > 1 {
                    self.set_source_filter(Some(parts[1]));
                } else {
                    self.set_source_filter(None); // Clear filter
                }
                self.exit_command();
            }

            // Favorites commands
            "fav" | "favorites" | "starred" => {
                self.toggle_favorites_filter();
                self.exit_command();
            }

            // Tab navigation
            "installed" | "1" => {
                self.switch_tab(Tab::Installed, db);
                self.exit_command();
            }
            "available" | "2" => {
                self.switch_tab(Tab::Available, db);
                self.exit_command();
            }
            "updates" | "3" => {
                self.switch_tab(Tab::Updates, db);
                self.exit_command();
            }
            "bundles" | "4" => {
                self.switch_tab(Tab::Bundles, db);
                self.exit_command();
            }
            "discover" | "5" => {
                self.switch_tab(Tab::Discover, db);
                self.exit_command();
            }

            // Install/Uninstall/Update
            "i" | "install" => {
                if self.tab == Tab::Bundles {
                    self.request_bundle_install(db);
                } else {
                    self.request_install();
                }
                self.exit_command();
            }
            "d" | "delete" | "uninstall" => {
                self.request_uninstall();
                self.exit_command();
            }
            "u" | "update" | "upgrade" => {
                self.request_update();
                self.exit_command();
            }

            // Undo/Redo
            "undo" | "z" => {
                self.undo();
                self.exit_command();
            }
            "redo" | "y" => {
                self.redo();
                self.exit_command();
            }

            // Config
            "c" | "config" | "settings" | "cfg" => {
                self.open_config_menu();
                self.exit_command();
            }

            // Create custom theme
            "create-theme" | "new-theme" => {
                self.create_custom_theme();
                self.exit_command();
            }

            // Edit custom theme (open file location)
            "edit-theme" => {
                self.show_custom_theme_path();
                self.exit_command();
            }

            // Unknown command
            _ => {
                self.set_status(format!("Unknown command: {}", parts[0]), true);
                self.exit_command();
            }
        }
    }

    /// Set theme by name
    fn set_theme_by_name(&mut self, name: &str) {
        use super::theme::{CustomTheme, ThemeVariant};
        self.theme_variant = match name {
            "mocha" | "catppuccin" | "catppuccin-mocha" => ThemeVariant::CatppuccinMocha,
            "latte" | "catppuccin-latte" => ThemeVariant::CatppuccinLatte,
            "dracula" => ThemeVariant::Dracula,
            "nord" => ThemeVariant::Nord,
            "tokyo" | "tokyo-night" | "tokyonight" => ThemeVariant::TokyoNight,
            "gruvbox" => ThemeVariant::Gruvbox,
            "custom" => {
                if CustomTheme::exists() {
                    ThemeVariant::Custom
                } else {
                    self.set_status(
                        "Custom theme not found. Use :create-theme to create one.".to_string(),
                        true,
                    );
                    return;
                }
            }
            _ => {
                self.set_status(
                    "Themes: mocha, latte, dracula, nord, tokyo, gruvbox, custom".to_string(),
                    true,
                );
                return;
            }
        };
        self.set_status(format!("Theme: {}", self.theme().name), false);
    }

    /// Create custom theme file
    fn create_custom_theme(&mut self) {
        use super::theme::CustomTheme;

        if CustomTheme::exists() {
            if let Ok(path) = CustomTheme::file_path() {
                self.set_status(
                    format!("Custom theme already exists: {}", path.display()),
                    false,
                );
            } else {
                self.set_status("Custom theme already exists".to_string(), false);
            }
            return;
        }

        match CustomTheme::create_default_if_missing() {
            Ok(true) => {
                if let Ok(path) = CustomTheme::file_path() {
                    self.set_status(format!("Created custom theme: {}", path.display()), false);
                } else {
                    self.set_status("Created custom theme file".to_string(), false);
                }
            }
            Ok(false) => {
                self.set_status("Custom theme already exists".to_string(), false);
            }
            Err(e) => {
                self.set_status(format!("Failed to create theme: {}", e), true);
            }
        }
    }

    /// Show custom theme file path
    fn show_custom_theme_path(&mut self) {
        use super::theme::CustomTheme;

        match CustomTheme::file_path() {
            Ok(path) => {
                if path.exists() {
                    self.set_status(format!("Custom theme: {}", path.display()), false);
                } else {
                    self.set_status(
                        "Custom theme not found. Create with :create-theme".to_string(),
                        true,
                    );
                }
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e), true);
            }
        }
    }

    /// Set sort by name
    fn set_sort_by_name(&mut self, name: &str) {
        self.sort_by = match name {
            "name" | "n" | "alpha" => SortBy::Name,
            "usage" | "u" | "used" => SortBy::Usage,
            "recent" | "r" | "last" => SortBy::Recent,
            _ => {
                self.set_status("Sort: name, usage, recent".to_string(), true);
                return;
            }
        };
        self.apply_filter_and_sort();
        self.set_status(format!("Sort by: {:?}", self.sort_by), false);
    }

    /// Set source filter
    pub fn set_source_filter(&mut self, source: Option<&str>) {
        match source {
            Some(s) if !s.is_empty() => {
                self.source_filter = Some(s.to_lowercase());
                self.set_status(format!("Filter: source={}", s), false);
            }
            _ => {
                self.source_filter = None;
                self.set_status("Source filter cleared".to_string(), false);
            }
        }
        self.apply_filter_and_sort();
    }

    /// Toggle favorites-only filter
    pub fn toggle_favorites_filter(&mut self) {
        self.favorites_only = !self.favorites_only;
        let status = if self.favorites_only {
            "Showing favorites only"
        } else {
            "Showing all tools"
        };
        self.set_status(status.to_string(), false);
        self.apply_filter_and_sort();
    }

    // ==================== Selection ====================

    /// Toggle selection of current tool
    pub fn toggle_selection(&mut self) {
        // Get tool name first to avoid borrow checker issues
        let tool_name = self.selected_tool().map(|t| t.name.clone());
        if let Some(name) = tool_name {
            self.record_selection(); // Record for undo
            if self.selected_tools.contains(&name) {
                self.selected_tools.remove(&name);
            } else {
                self.selected_tools.insert(name);
            }
        }
    }

    /// Check if a tool is selected
    pub fn is_selected(&self, tool_name: &str) -> bool {
        self.selected_tools.contains(tool_name)
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        if !self.selected_tools.is_empty() {
            self.record_selection(); // Record for undo
            self.selected_tools.clear();
        }
    }

    /// Select all visible tools
    pub fn select_all(&mut self) {
        self.record_selection(); // Record for undo
        for tool in &self.tools {
            self.selected_tools.insert(tool.name.clone());
        }
    }

    /// Get count of selected tools
    pub fn selection_count(&self) -> usize {
        self.selected_tools.len()
    }

    /// Get names of selected tools
    pub fn get_selected_tools(&self) -> Vec<String> {
        self.selected_tools.iter().cloned().collect()
    }

    // ==================== Details Popup ====================

    /// Toggle details popup (for narrow terminals)
    pub fn toggle_details_popup(&mut self) {
        self.show_details_popup = !self.show_details_popup;
    }

    /// Close details popup
    pub fn close_details_popup(&mut self) {
        self.show_details_popup = false;
    }

    // ==================== Mouse Support ====================

    /// Set the list area for mouse interaction
    pub fn set_list_area(&mut self, x: u16, y: u16, width: u16, height: u16) {
        self.last_list_area = Some((x, y, width, height));
    }

    /// Set the tab area for mouse interaction
    pub fn set_tab_area(&mut self, x: u16, y: u16, width: u16, height: u16) {
        self.last_tab_area = Some((x, y, width, height));
    }

    /// Handle mouse click on list item
    pub fn click_list_item(&mut self, row: u16) {
        if self.tab == Tab::Bundles {
            // Handle bundle list clicks
            let target_index = row as usize; // Bundles don't scroll currently
            if target_index < self.bundles.len() {
                self.bundle_selected = target_index;
            }
        } else {
            // Handle tool list clicks
            let target_index = self.list_offset + row as usize;
            if target_index < self.tools.len() {
                self.selected_index = target_index;
            }
        }
    }

    /// Handle mouse click on tab
    pub fn click_tab(&mut self, x: u16, db: &Database) {
        if let Some((area_x, _, _, _)) = self.last_tab_area {
            // Account for block border (1 char on left)
            let content_start = area_x + 1;
            let relative_x = x.saturating_sub(content_start) as usize;

            // Tab layout (with padding("", "") set in UI):
            // Each tab: " title " = title.len() + 2
            // Divider between tabs: "│" (1 char)
            let tabs = Tab::all();
            let mut pos = 0;

            for (i, tab) in tabs.iter().enumerate() {
                let tab_width = tab.title().len() + 2; // " title "

                if relative_x >= pos && relative_x < pos + tab_width {
                    self.switch_tab(*tab, db);
                    return;
                }

                pos += tab_width;

                // Add divider width (1 char) after each tab except the last
                if i < tabs.len() - 1 {
                    pos += 1;
                }
            }
        }
    }

    /// Check if click is in list area and return relative row
    pub fn get_list_row(&self, x: u16, y: u16) -> Option<u16> {
        if let Some((area_x, area_y, width, height)) = self.last_list_area
            && x >= area_x
            && x < area_x + width
            && y >= area_y
            && y < area_y + height
        {
            // Skip header row
            if y > area_y {
                return Some(y - area_y - 1);
            }
        }
        None
    }

    /// Check if click is in tab area
    pub fn is_in_tab_area(&self, x: u16, y: u16) -> bool {
        if let Some((area_x, area_y, width, height)) = self.last_tab_area {
            x >= area_x && x < area_x + width && y >= area_y && y < area_y + height
        } else {
            false
        }
    }

    // ==================== Undo/Redo ====================

    /// Undo the last action
    pub fn undo(&mut self) {
        if let Some(action) = self.history.pop_undo() {
            // Save current state for redo
            let redo_action = match &action {
                UndoableAction::Selection(_) => {
                    UndoableAction::Selection(self.selected_tools.clone())
                }
                UndoableAction::Filter(_) => UndoableAction::Filter(self.search_query.clone()),
                UndoableAction::TabSwitch(_) => UndoableAction::TabSwitch(self.tab),
                UndoableAction::Sort(_) => UndoableAction::Sort(self.sort_by),
            };
            self.history.push_redo(redo_action);

            // Restore previous state
            match action {
                UndoableAction::Selection(prev) => {
                    self.selected_tools = prev;
                    self.set_status("Selection restored".to_string(), false);
                }
                UndoableAction::Filter(prev) => {
                    self.search_query = prev;
                    self.apply_filter_and_sort();
                    self.set_status("Filter restored".to_string(), false);
                }
                UndoableAction::TabSwitch(prev) => {
                    self.tab = prev;
                    self.set_status(format!("Tab: {:?}", self.tab), false);
                }
                UndoableAction::Sort(prev) => {
                    self.sort_by = prev;
                    self.apply_filter_and_sort();
                    self.set_status(format!("Sort: {:?}", self.sort_by), false);
                }
            }
        } else {
            self.set_status("Nothing to undo".to_string(), true);
        }
    }

    /// Redo the last undone action
    pub fn redo(&mut self) {
        if let Some(action) = self.history.pop_redo() {
            // Save current state for undo
            let undo_action = match &action {
                UndoableAction::Selection(_) => {
                    UndoableAction::Selection(self.selected_tools.clone())
                }
                UndoableAction::Filter(_) => UndoableAction::Filter(self.search_query.clone()),
                UndoableAction::TabSwitch(_) => UndoableAction::TabSwitch(self.tab),
                UndoableAction::Sort(_) => UndoableAction::Sort(self.sort_by),
            };
            self.history.undo_stack.push(undo_action);

            // Apply the redo action
            match action {
                UndoableAction::Selection(new) => {
                    self.selected_tools = new;
                    self.set_status("Selection redone".to_string(), false);
                }
                UndoableAction::Filter(new) => {
                    self.search_query = new;
                    self.apply_filter_and_sort();
                    self.set_status("Filter redone".to_string(), false);
                }
                UndoableAction::TabSwitch(new) => {
                    self.tab = new;
                    self.set_status(format!("Tab: {:?}", self.tab), false);
                }
                UndoableAction::Sort(new) => {
                    self.sort_by = new;
                    self.apply_filter_and_sort();
                    self.set_status(format!("Sort: {:?}", self.sort_by), false);
                }
            }
        } else {
            self.set_status("Nothing to redo".to_string(), true);
        }
    }

    /// Record a selection change
    fn record_selection(&mut self) {
        self.history
            .push(UndoableAction::Selection(self.selected_tools.clone()));
    }

    /// Record a filter change
    fn record_filter(&mut self) {
        self.history
            .push(UndoableAction::Filter(self.search_query.clone()));
    }

    // ==================== Actions ====================

    /// Request install action for selected tools (or current tool if none selected)
    pub fn request_install(&mut self) {
        let tools = if self.selected_tools.is_empty() {
            // Use current tool if nothing selected
            self.selected_tool()
                .filter(|t| !t.is_installed)
                .map(|t| vec![t.name.clone()])
                .unwrap_or_default()
        } else {
            // Use selected tools that aren't installed
            self.selected_tools
                .iter()
                .filter(|name| {
                    self.tools
                        .iter()
                        .any(|t| &t.name == *name && !t.is_installed)
                })
                .cloned()
                .collect()
        };

        if !tools.is_empty() {
            self.pending_action = Some(PendingAction::Install(tools));
        }
    }

    /// Request uninstall action for selected tools (or current tool if none selected)
    pub fn request_uninstall(&mut self) {
        let tools = if self.selected_tools.is_empty() {
            // Use current tool if nothing selected
            self.selected_tool()
                .filter(|t| t.is_installed)
                .map(|t| vec![t.name.clone()])
                .unwrap_or_default()
        } else {
            // Use selected tools that are installed
            self.selected_tools
                .iter()
                .filter(|name| {
                    self.tools
                        .iter()
                        .any(|t| &t.name == *name && t.is_installed)
                })
                .cloned()
                .collect()
        };

        if !tools.is_empty() {
            self.pending_action = Some(PendingAction::Uninstall(tools));
        }
    }

    /// Request update action for selected tools (or current tool if none selected)
    pub fn request_update(&mut self) {
        let tools = if self.selected_tools.is_empty() {
            // Use current tool if it has an update
            self.selected_tool()
                .filter(|t| self.available_updates.contains_key(&t.name))
                .map(|t| vec![t.name.clone()])
                .unwrap_or_default()
        } else {
            // Use selected tools that have updates
            self.selected_tools
                .iter()
                .filter(|name| self.available_updates.contains_key(*name))
                .cloned()
                .collect()
        };

        if !tools.is_empty() {
            self.pending_action = Some(PendingAction::Update(tools));
        }
    }

    /// Request install for missing tools in selected bundle
    pub fn request_bundle_install(&mut self, db: &Database) {
        let Some(bundle) = self.selected_bundle() else {
            return;
        };

        // Find tools that aren't installed
        let missing_tools: Vec<String> = bundle
            .tools
            .iter()
            .filter(|name| {
                !db.get_tool_by_name(name)
                    .ok()
                    .flatten()
                    .map(|t| t.is_installed)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if !missing_tools.is_empty() {
            self.pending_action = Some(PendingAction::Install(missing_tools));
        } else {
            self.set_status("All tools in bundle are already installed", false);
        }
    }

    /// Track missing bundle tools as available (add to tools table with is_installed=false)
    pub fn track_bundle_tools(&mut self, db: &Database) {
        use crate::models::Tool;

        let Some(bundle) = self.selected_bundle() else {
            return;
        };

        // Find tools that don't exist in the tools table yet
        let untracked: Vec<String> = bundle
            .tools
            .iter()
            .filter(|name| db.get_tool_by_name(name).ok().flatten().is_none())
            .cloned()
            .collect();

        if untracked.is_empty() {
            self.set_status("All bundle tools are already tracked", false);
            return;
        }

        let count = untracked.len();
        let mut added = 0;

        for name in &untracked {
            let tool = Tool::new(name);
            if db.insert_tool(&tool).is_ok() {
                added += 1;
            }
        }

        if added > 0 {
            self.set_status(format!("Added {} tool(s) to Available", added), false);
            // Refresh the labels cache in case we want to add labels later
            self.labels_cache = db.get_all_tool_labels().unwrap_or_default();
        } else {
            self.set_status(format!("Failed to add {} tool(s)", count), true);
        }
    }

    /// Confirm and return the pending action
    pub fn confirm_action(&mut self) -> Option<PendingAction> {
        self.pending_action.take()
    }

    /// Cancel the pending action
    pub fn cancel_action(&mut self) {
        self.pending_action = None;
    }

    /// Check if there's a pending action
    pub fn has_pending_action(&self) -> bool {
        self.pending_action.is_some()
    }

    /// Set a status message
    pub fn set_status(&mut self, text: impl Into<String>, is_error: bool) {
        self.status_message = Some(StatusMessage {
            text: text.into(),
            is_error,
        });
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    // ==================== Background Operations ====================

    /// Schedule a background operation (will be executed by main loop)
    pub fn schedule_op(&mut self, op: BackgroundOp) {
        self.background_op = Some(op);
    }

    /// Check if there's a pending background operation
    pub fn has_background_op(&self) -> bool {
        self.background_op.is_some()
    }

    /// Execute one step of the pending background operation
    /// Returns true if there are more steps to execute
    pub fn execute_background_step(&mut self, db: &Database) -> bool {
        use crate::{
            check_apt_updates, check_brew_updates, check_cargo_updates, check_npm_updates,
            check_pip_updates,
        };

        let Some(op) = self.background_op.take() else {
            return false;
        };

        match op {
            BackgroundOp::CheckUpdates { step } => {
                let checkers: &[fn() -> anyhow::Result<Vec<Update>>] = &[
                    check_cargo_updates,
                    check_pip_updates,
                    check_npm_updates,
                    check_apt_updates,
                    check_brew_updates,
                ];

                // Initialize on first step
                if step == 0 {
                    self.available_updates.clear();
                    self.updates_loading = true;
                }

                // Get tracked tool names to filter updates
                let tracked_tools: HashSet<String> = db
                    .list_tools(true, None)
                    .map(|tools| tools.into_iter().map(|t| t.name).collect())
                    .unwrap_or_default();

                // Update progress for UI
                self.loading_progress = LoadingProgress {
                    current_step: step + 1,
                    total_steps: PACKAGE_MANAGERS.len(),
                    step_name: PACKAGE_MANAGERS[step].1.to_string(),
                    found_count: self.available_updates.len(),
                };

                // Execute this step's checker - only keep updates for tracked tools
                if let Ok(updates) = checkers[step]() {
                    for update in updates {
                        if tracked_tools.contains(&update.name) {
                            self.available_updates.insert(update.name.clone(), update);
                        }
                    }
                }

                // Check if there are more steps
                let next_step = step + 1;
                if next_step < checkers.len() {
                    // More steps to go
                    self.background_op = Some(BackgroundOp::CheckUpdates { step: next_step });
                    true
                } else {
                    // All done - finalize
                    self.updates_checked = true;
                    self.updates_loading = false;
                    self.refresh_tools(db);

                    let count = self.available_updates.len();
                    if count == 0 {
                        self.set_status("All tools are up to date!", false);
                    } else {
                        self.set_status(format!("{} update(s) available", count), false);
                    }
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact() {
        assert!(fuzzy_match("ripgrep", "ripgrep").is_some());
        let score = fuzzy_match("ripgrep", "ripgrep").unwrap();
        assert!(score > 100); // Exact match bonus
    }

    #[test]
    fn test_fuzzy_match_prefix() {
        assert!(fuzzy_match("rip", "ripgrep").is_some());
        let score = fuzzy_match("rip", "ripgrep").unwrap();
        assert!(score > 50); // Prefix bonus
    }

    #[test]
    fn test_fuzzy_match_subsequence() {
        // "rg" matches "ripgrep" (r...g)
        assert!(fuzzy_match("rg", "ripgrep").is_some());

        // "fdf" matches "fd-find"
        assert!(fuzzy_match("fdf", "fd-find").is_some());
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        // Characters must appear in order in target
        assert!(fuzzy_match("xyz", "ripgrep").is_none());
        assert!(fuzzy_match("abc", "ripgrep").is_none());
        // "gr" actually matches ripGRep (g at 3, r at 4)
        assert!(fuzzy_match("gr", "ripgrep").is_some());
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        assert!(fuzzy_match("RIP", "ripgrep").is_some());
        assert!(fuzzy_match("rip", "RIPGREP").is_some());
    }

    #[test]
    fn test_fuzzy_match_word_boundary_bonus() {
        // Matching at word boundary should score higher
        let boundary_score = fuzzy_match("f", "fd-find").unwrap();
        let mid_score = fuzzy_match("i", "fd-find").unwrap();
        assert!(boundary_score > mid_score);
    }

    #[test]
    fn test_fuzzy_match_consecutive_bonus() {
        // Consecutive matches should score higher
        let consecutive = fuzzy_match("rip", "ripgrep").unwrap();
        let spread = fuzzy_match("rgp", "ripgrep").unwrap(); // r...g...p (positions 0,3,6)
        assert!(consecutive > spread);
    }

    // ==================== Command Palette Tests ====================

    #[test]
    fn test_command_mode_enter_exit() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.command_input.is_empty());

        app.enter_command();
        assert_eq!(app.input_mode, InputMode::Command);
        assert!(app.command_input.is_empty());

        app.command_push('q');
        assert_eq!(app.command_input, "q");

        app.exit_command();
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.command_input.is_empty());
    }

    #[test]
    fn test_command_push_pop() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        app.enter_command();
        app.command_push('h');
        app.command_push('e');
        app.command_push('l');
        app.command_push('p');
        assert_eq!(app.command_input, "help");

        app.command_pop();
        assert_eq!(app.command_input, "hel");

        app.command_pop();
        app.command_pop();
        app.command_pop();
        assert!(app.command_input.is_empty());
    }

    #[test]
    fn test_command_execute_help() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        app.enter_command();
        app.command_push('h');
        app.execute_command(&db);

        assert!(app.show_help);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_command_execute_quit() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        assert!(app.running);
        app.enter_command();
        app.command_push('q');
        app.execute_command(&db);

        assert!(!app.running);
    }

    #[test]
    fn test_command_unknown() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        app.enter_command();
        for c in "invalidcmd".chars() {
            app.command_push(c);
        }
        app.execute_command(&db);

        // Should have status message about unknown command
        assert!(app.status_message.is_some());
        assert!(app.status_message.as_ref().unwrap().is_error);
    }

    // ==================== Undo/Redo Tests ====================

    #[test]
    fn test_undo_selection() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Initial state - no selections
        assert!(app.selected_tools.is_empty());

        // Record initial empty state, then add selections
        app.record_selection();
        app.selected_tools.insert("tool1".to_string());
        app.selected_tools.insert("tool2".to_string());

        // Undo should restore to empty state
        app.undo();
        assert!(app.selected_tools.is_empty());
    }

    #[test]
    fn test_undo_filter() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Set a filter and record it
        app.search_query = "old_filter".to_string();
        app.record_filter();
        app.search_query = "new_filter".to_string();

        // Undo should restore old filter
        app.undo();
        assert_eq!(app.search_query, "old_filter");
    }

    #[test]
    fn test_redo() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Set filter and record
        app.search_query = "filter1".to_string();
        app.record_filter();
        app.search_query = "filter2".to_string();

        // Undo
        app.undo();
        assert_eq!(app.search_query, "filter1");

        // Redo should restore to filter2
        app.redo();
        assert_eq!(app.search_query, "filter2");
    }

    #[test]
    fn test_action_history() {
        let mut history = ActionHistory::new(3);

        // Initially empty
        assert!(!history.can_undo());
        assert!(!history.can_redo());

        // Add actions
        history.push(UndoableAction::Filter("a".to_string()));
        history.push(UndoableAction::Filter("b".to_string()));
        assert!(history.can_undo());

        // Pop undo
        let action = history.pop_undo().unwrap();
        if let UndoableAction::Filter(s) = action {
            assert_eq!(s, "b");
        }

        // Push to redo
        history.push_redo(UndoableAction::Filter("b".to_string()));
        assert!(history.can_redo());

        // Pop redo
        let action = history.pop_redo().unwrap();
        if let UndoableAction::Filter(s) = action {
            assert_eq!(s, "b");
        }
    }

    #[test]
    fn test_history_max_size() {
        let mut history = ActionHistory::new(2);

        history.push(UndoableAction::Filter("a".to_string()));
        history.push(UndoableAction::Filter("b".to_string()));
        history.push(UndoableAction::Filter("c".to_string()));

        // Should only have 2 actions (oldest removed)
        assert!(history.can_undo());
        let _ = history.pop_undo(); // c
        let action = history.pop_undo(); // b
        if let Some(UndoableAction::Filter(s)) = action {
            assert_eq!(s, "b");
        }

        // No more undo
        assert!(!history.can_undo());
    }

    // ==================== Mouse Handler Tests ====================

    #[test]
    fn test_click_list_item_tool() {
        use crate::models::InstallSource;
        let db = Database::open_in_memory().unwrap();
        // Insert installed tools (App starts on Installed tab)
        db.insert_tool(
            &Tool::new("tool1")
                .with_source(InstallSource::Cargo)
                .installed(),
        )
        .unwrap();
        db.insert_tool(
            &Tool::new("tool2")
                .with_source(InstallSource::Cargo)
                .installed(),
        )
        .unwrap();
        db.insert_tool(
            &Tool::new("tool3")
                .with_source(InstallSource::Cargo)
                .installed(),
        )
        .unwrap();
        let mut app = App::new(&db).unwrap();

        assert_eq!(app.selected_index, 0);

        // Click on second item (row 1)
        app.click_list_item(1);
        assert_eq!(app.selected_index, 1);

        // Click on third item (row 2)
        app.click_list_item(2);
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn test_click_list_item_with_offset() {
        use crate::models::InstallSource;
        let db = Database::open_in_memory().unwrap();
        for i in 0..10 {
            db.insert_tool(
                &Tool::new(format!("tool{}", i))
                    .with_source(InstallSource::Cargo)
                    .installed(),
            )
            .unwrap();
        }
        let mut app = App::new(&db).unwrap();

        // Simulate scrolled list with offset 5
        app.list_offset = 5;

        // Click on first visible item (row 0) should select tool5
        app.click_list_item(0);
        assert_eq!(app.selected_index, 5);

        // Click on row 3 should select tool8
        app.click_list_item(3);
        assert_eq!(app.selected_index, 8);
    }

    #[test]
    fn test_click_list_item_out_of_bounds() {
        use crate::models::InstallSource;
        let db = Database::open_in_memory().unwrap();
        db.insert_tool(
            &Tool::new("tool1")
                .with_source(InstallSource::Cargo)
                .installed(),
        )
        .unwrap();
        let mut app = App::new(&db).unwrap();

        assert_eq!(app.selected_index, 0);

        // Click on row 10 (out of bounds) - should not change selection
        app.click_list_item(10);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_set_list_area() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        assert!(app.last_list_area.is_none());

        app.set_list_area(10, 20, 100, 50);
        assert_eq!(app.last_list_area, Some((10, 20, 100, 50)));
    }

    #[test]
    fn test_get_list_row() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Set list area: x=10, y=5, width=80, height=20
        app.set_list_area(10, 5, 80, 20);

        // Click inside list area (accounting for border)
        // y=6 is first content row (after top border at y=5)
        let row = app.get_list_row(15, 6);
        assert_eq!(row, Some(0));

        // y=7 is second content row
        let row = app.get_list_row(15, 7);
        assert_eq!(row, Some(1));

        // Click outside list area (x too small)
        let row = app.get_list_row(5, 7);
        assert!(row.is_none());

        // Click outside list area (y too small - on border)
        let row = app.get_list_row(15, 5);
        assert!(row.is_none());

        // Click outside list area (y too large)
        let row = app.get_list_row(15, 30);
        assert!(row.is_none());
    }

    #[test]
    fn test_set_tab_area() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        assert!(app.last_tab_area.is_none());

        app.set_tab_area(0, 0, 120, 3);
        assert_eq!(app.last_tab_area, Some((0, 0, 120, 3)));
    }

    #[test]
    fn test_click_tab() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Set tab area starting at x=0
        app.set_tab_area(0, 0, 120, 3);

        // Initially on Installed tab
        assert_eq!(app.tab, Tab::Installed);

        // Tab layout (accounting for border and padding):
        // Content starts at x=1 (after border)
        // Tab format: " title " with dividers
        // Installed: " Installed " (11 chars), Available: " Available " (11 chars), etc.

        // Click on first tab (Installed) - should stay on Installed
        // Position 1-12 is " Installed "
        app.click_tab(5, &db);
        assert_eq!(app.tab, Tab::Installed);

        // Click on second tab (Available)
        // After Installed (12 chars) + divider (1) = start at 13
        app.click_tab(15, &db);
        assert_eq!(app.tab, Tab::Available);
    }
}
