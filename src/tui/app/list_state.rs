//! List state management for navigable lists
//!
//! This module provides `BundleState` for managing bundle list navigation
//! using the `SelectableList` trait.

use anyhow::Result;

use super::traits::SelectableList;
use crate::db::Database;
use crate::models::Bundle;

/// Manages bundle list state and navigation
#[derive(Debug, Default)]
pub struct BundleState {
    /// All bundles
    pub items: Vec<Bundle>,
    /// Currently selected index
    pub selected: usize,
}

impl SelectableList for BundleState {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn selected_index(&self) -> usize {
        self.selected
    }

    fn set_selected_index(&mut self, idx: usize) {
        self.selected = idx;
    }
}

impl BundleState {
    /// Create from bundles list
    pub fn new(bundles: Vec<Bundle>) -> Self {
        Self {
            items: bundles,
            selected: 0,
        }
    }

    /// Get the number of bundles
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Move selection down (uses trait default)
    pub fn next(&mut self) {
        SelectableList::select_next(self);
    }

    /// Move selection up (uses trait default)
    pub fn prev(&mut self) {
        SelectableList::select_prev(self);
    }

    /// Jump to first item (uses trait default)
    pub fn first(&mut self) {
        SelectableList::select_first(self);
    }

    /// Jump to last item (uses trait default)
    pub fn last(&mut self) {
        SelectableList::select_last(self);
    }

    /// Get currently selected bundle
    pub fn selected_bundle(&self) -> Option<&Bundle> {
        self.items.get(self.selected)
    }

    /// Select by index (for mouse clicks)
    pub fn select(&mut self, index: usize) {
        SelectableList::select(self, index);
    }

    /// Reload bundles from database
    pub fn reload(&mut self, db: &Database) -> Result<()> {
        self.items = db.list_bundles()?;
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        Ok(())
    }

    /// Check if empty (delegate to items)
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get bundle by index (delegate to items)
    pub fn get(&self, index: usize) -> Option<&Bundle> {
        self.items.get(index)
    }

    /// Iterate over bundles (delegate to items)
    pub fn iter(&self) -> impl Iterator<Item = &Bundle> {
        self.items.iter()
    }
}
