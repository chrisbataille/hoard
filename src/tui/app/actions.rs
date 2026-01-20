//! Action operations for the TUI
//!
//! This module contains methods for selection, install/uninstall actions,
//! undo/redo history, and pending action management.

use crate::commands::install::get_install_command_versioned;
use crate::db::Database;
use crate::models::Tool;

use super::App;
use super::types::{
    DiscoverMetadata, DiscoverSource, InstallOption, InstallTask, PendingAction, UndoableAction,
};

impl App {
    // ========================================================================
    // Multi-Selection
    // ========================================================================

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

    // ========================================================================
    // Undo/Redo
    // ========================================================================

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
    pub fn record_selection(&mut self) {
        self.history
            .push(UndoableAction::Selection(self.selected_tools.clone()));
    }

    /// Record a filter change
    pub fn record_filter(&mut self) {
        self.history
            .push(UndoableAction::Filter(self.search_query.clone()));
    }

    // ========================================================================
    // Install/Uninstall/Update Actions
    // ========================================================================

    /// Build InstallTask from a Tool with optional version and metadata
    /// Always regenerates display_command for consistency and security
    fn build_install_task(name: &str, source: &str, version: Option<&str>) -> Option<InstallTask> {
        Self::build_install_task_with_metadata(name, source, version, None, None, None)
    }

    /// Build InstallTask with optional Discover metadata (description, stars, url)
    fn build_install_task_with_metadata(
        name: &str,
        source: &str,
        version: Option<&str>,
        description: Option<String>,
        stars: Option<u64>,
        url: Option<String>,
    ) -> Option<InstallTask> {
        // Always regenerate display command - don't trust external sources
        let display_command =
            get_install_command_versioned(name, source, version).unwrap_or_else(|| {
                // Fallback display
                match version {
                    Some(v) => format!("{} install {}@{}", source, name, v),
                    None => format!("{} install {}", source, name),
                }
            });

        Some(InstallTask {
            name: name.to_string(),
            source: source.to_string(),
            version: version.map(String::from),
            display_command,
            description,
            stars,
            url,
        })
    }

    /// Request install action for selected tools (or current tool if none selected)
    pub fn request_install(&mut self, db: &Database) {
        let tool_names: Vec<String> = if self.selected_tools.is_empty() {
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

        if tool_names.is_empty() {
            return;
        }

        // Build InstallTask for each tool (need to look up source from db)
        let tasks: Vec<InstallTask> = tool_names
            .iter()
            .filter_map(|name| {
                let tool = db.get_tool_by_name(name).ok().flatten()?;
                Self::build_install_task(&tool.name, &tool.source.to_string(), None)
            })
            .collect();

        if !tasks.is_empty() {
            self.pending_action = Some(PendingAction::Install(tasks));
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
    pub fn request_update(&mut self, db: &Database) {
        let tool_names: Vec<String> = if self.selected_tools.is_empty() {
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

        if tool_names.is_empty() {
            return;
        }

        // Build InstallTask for each tool with update info
        let tasks: Vec<InstallTask> = tool_names
            .iter()
            .filter_map(|name| {
                let tool = db.get_tool_by_name(name).ok().flatten()?;
                let update = self.available_updates.get(name)?;
                Self::build_install_task(&tool.name, &tool.source.to_string(), Some(&update.latest))
            })
            .collect();

        if !tasks.is_empty() {
            self.pending_action = Some(PendingAction::Update(tasks));
        }
    }

    /// Request install for a discovered tool
    pub fn request_discover_install(&mut self) {
        let Some(result) = self.selected_discover() else {
            return;
        };

        // Filter to installable options (exclude GitHub, AI)
        let installable_options: Vec<InstallOption> = result
            .install_options
            .iter()
            .filter(|opt| {
                matches!(
                    opt.source,
                    DiscoverSource::CratesIo
                        | DiscoverSource::PyPI
                        | DiscoverSource::Npm
                        | DiscoverSource::Homebrew
                        | DiscoverSource::Apt
                )
            })
            .cloned()
            .collect();

        if installable_options.is_empty() {
            self.set_status("No installable source available (GitHub/AI only)", true);
            return;
        }

        // Build metadata from discover result
        let metadata = DiscoverMetadata {
            description: result.description.clone(),
            stars: result.stars,
            url: result.url.clone(),
        };

        // If multiple sources available, show selection dialog
        if installable_options.len() > 1 {
            self.pending_action = Some(PendingAction::DiscoverSelectSource(
                result.name.clone(),
                installable_options,
                0,
                metadata,
            ));
            return;
        }

        // Single source - proceed directly
        let option = &installable_options[0];
        let source = Self::discover_source_to_str(&option.source);

        // Always regenerate display command for security - don't trust external sources
        let Some(task) = Self::build_install_task_with_metadata(
            &result.name,
            source,
            None,
            metadata.description,
            metadata.stars,
            metadata.url,
        ) else {
            self.set_status("Failed to build install command", true);
            return;
        };
        self.pending_action = Some(PendingAction::DiscoverInstall(task));
    }

    /// Convert DiscoverSource to source string for install
    fn discover_source_to_str(source: &DiscoverSource) -> &'static str {
        match source {
            DiscoverSource::CratesIo => "cargo",
            DiscoverSource::PyPI => "pip",
            DiscoverSource::Npm => "npm",
            DiscoverSource::Homebrew => "brew",
            DiscoverSource::Apt => "apt",
            DiscoverSource::GitHub | DiscoverSource::AI => "unknown",
        }
    }

    /// Navigate source selection up/down (delta: 1 for down, -1 for up)
    pub fn navigate_source_selection(&mut self, delta: i32) {
        if let Some(PendingAction::DiscoverSelectSource(_, options, selected, _)) =
            &mut self.pending_action
        {
            let len = options.len();
            if len == 0 {
                return;
            }
            if delta > 0 {
                *selected = (*selected + 1) % len;
            } else {
                *selected = selected.checked_sub(1).unwrap_or(len - 1);
            }
        }
    }

    /// Confirm source selection and convert to DiscoverInstall
    pub fn confirm_source_selection(&mut self) {
        let (name, options, selected, metadata) =
            if let Some(PendingAction::DiscoverSelectSource(name, options, selected, metadata)) =
                self.pending_action.take()
            {
                (name, options, selected, metadata)
            } else {
                return;
            };

        let Some(option) = options.get(selected) else {
            self.set_status("Invalid selection", true);
            return;
        };

        let source = Self::discover_source_to_str(&option.source);

        // Build install task for the selected source with metadata
        let Some(task) = Self::build_install_task_with_metadata(
            &name,
            source,
            None,
            metadata.description,
            metadata.stars,
            metadata.url,
        ) else {
            self.set_status("Failed to build install command", true);
            return;
        };

        self.pending_action = Some(PendingAction::DiscoverInstall(task));
    }

    /// Request install for missing tools in selected bundle
    pub fn request_bundle_install(&mut self, db: &Database) {
        let Some(bundle) = self.selected_bundle() else {
            return;
        };

        // Find tools that aren't installed and build tasks
        let tasks: Vec<InstallTask> = bundle
            .tools
            .iter()
            .filter_map(|name| {
                let tool = db.get_tool_by_name(name).ok().flatten()?;
                if tool.is_installed {
                    return None;
                }
                Self::build_install_task(&tool.name, &tool.source.to_string(), None)
            })
            .collect();

        if !tasks.is_empty() {
            self.pending_action = Some(PendingAction::Install(tasks));
        } else {
            self.set_status("All tools in bundle are already installed", false);
        }
    }

    /// Track missing bundle tools as available (add to tools table with is_installed=false)
    pub fn track_bundle_tools(&mut self, db: &Database) {
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
            self.cache.labels_cache = db.get_all_tool_labels().unwrap_or_default();
        } else {
            self.set_status(format!("Failed to add {} tool(s)", count), true);
        }
    }

    // ========================================================================
    // Pending Action Management
    // ========================================================================

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
}
