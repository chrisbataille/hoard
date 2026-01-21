//! Config menu operations for the TUI
//!
//! This module contains all methods related to the configuration menu,
//! including opening, navigation, and saving config.

use crate::config::{AiProvider, HoardConfig};

use super::App;
use super::types::{ConfigMenuState, ConfigSection, config_menu_layout};

impl App {
    // ========================================================================
    // Config Menu Core
    // ========================================================================

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

    /// Close config menu without saving (reverts any preview changes)
    pub fn close_config_menu(&mut self) {
        // Revert any live preview changes by reloading from config
        if let Ok(config) = HoardConfig::load() {
            self.theme_variant =
                super::super::theme::ThemeVariant::from_config_theme(config.tui.theme);
            self.ai_available = config.ai.provider != AiProvider::None;
        }
        self.show_config_menu = false;
        // Refresh discover sources in case config changed
        self.refresh_discover_sources();
    }

    /// Save config from menu and close
    pub fn save_config_menu(&mut self) {
        let config = self.config_menu.to_config();

        // Apply theme immediately
        self.theme_variant = super::super::theme::ThemeVariant::from_config_theme(config.tui.theme);

        // Update AI availability
        self.ai_available = config.ai.provider != AiProvider::None;

        // Save to file
        if let Err(e) = config.save() {
            self.set_status(format!("Failed to save config: {}", e), true);
        } else {
            self.set_status("Configuration saved".to_string(), false);
        }

        self.show_config_menu = false;
        // Refresh discover sources based on new config
        self.refresh_discover_sources();
    }

    /// Check if config menu should auto-launch (no config file exists)
    pub fn should_show_config_on_start() -> bool {
        !HoardConfig::exists()
    }

    // ========================================================================
    // Config Menu Navigation
    // ========================================================================

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

    /// Toggle source in config menu (only if the source is available)
    pub fn config_menu_toggle_source(&mut self) {
        // Only allow toggling if the source is available
        if self.config_menu.section == ConfigSection::Sources {
            let sources = crate::config::SourcesConfig::all_sources();
            if self.config_menu.source_focused < sources.len() {
                let source_name = sources[self.config_menu.source_focused];
                // Check if this package manager is available
                if self.package_managers.is_available(source_name) {
                    self.config_menu.toggle_current_source();
                }
                // If not available, do nothing (can't toggle)
            }
        }
    }

    // ========================================================================
    // Config Menu Scrolling
    // ========================================================================

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

    // ========================================================================
    // Config Menu Selection
    // ========================================================================

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
}
