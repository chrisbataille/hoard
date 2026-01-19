//! Traits for list navigation behavior
//!
//! This module provides the `SelectableList` trait that unifies navigation
//! logic across Tools, Bundles, and Discover lists.

/// Trait for navigable list state
///
/// Provides default implementations for common navigation operations
/// (next, prev, first, last) based on the list length and current selection.
pub trait SelectableList {
    /// Returns the total number of items in the list
    fn len(&self) -> usize;

    /// Returns true if the list is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the currently selected index
    fn selected_index(&self) -> usize;

    /// Sets the selected index (will be clamped to valid range)
    fn set_selected_index(&mut self, idx: usize);

    /// Move selection to the next item (clamped to last item)
    fn select_next(&mut self) {
        if !self.is_empty() {
            let new_idx = (self.selected_index() + 1).min(self.len() - 1);
            self.set_selected_index(new_idx);
        }
    }

    /// Move selection to the previous item (clamped to first item)
    fn select_prev(&mut self) {
        let new_idx = self.selected_index().saturating_sub(1);
        self.set_selected_index(new_idx);
    }

    /// Move selection to the first item
    fn select_first(&mut self) {
        self.set_selected_index(0);
    }

    /// Move selection to the last item
    fn select_last(&mut self) {
        if !self.is_empty() {
            self.set_selected_index(self.len() - 1);
        }
    }

    /// Select a specific index (clamped to valid range)
    fn select(&mut self, index: usize) {
        if !self.is_empty() {
            let clamped = index.min(self.len() - 1);
            self.set_selected_index(clamped);
        }
    }
}
