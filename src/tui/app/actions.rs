//! Action operations for the TUI
//!
//! This module contains methods for selection, install/uninstall actions,
//! undo/redo history, and pending action management.

use crate::db::Database;
use crate::models::Tool;

use super::App;
use super::types::{PendingAction, UndoableAction};

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
