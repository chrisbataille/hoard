//! Extracted components for the TUI application
//!
//! This module contains helper structs that manage specific aspects of the app state:
//! - CacheManager: Usage data, GitHub info, labels caches
//! - CommandPalette: Command input and history
//! - Fuzzy match functions for search

use std::collections::HashMap;

use crate::db::{Database, GitHubInfo, ToolUsage};

/// Fuzzy match a query against a target string (fzf-style)
/// Returns Some(score) if matches, None if no match
/// Higher scores = better matches
pub fn fuzzy_match(query: &str, target: &str) -> Option<i32> {
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

/// Manages cached data for the TUI (usage, GitHub info, labels)
#[derive(Debug, Default)]
pub struct CacheManager {
    /// Usage data per tool
    pub usage_data: HashMap<String, ToolUsage>,
    /// 7-day daily usage counts for sparklines
    pub daily_usage: HashMap<String, Vec<i64>>,
    /// GitHub info cache (stars, description, etc.)
    pub github_cache: HashMap<String, GitHubInfo>,
    /// Labels/tags per tool
    pub labels_cache: HashMap<String, Vec<String>>,
}

impl CacheManager {
    /// Create a new cache manager, loading data from database
    pub fn new(db: &Database) -> Self {
        let usage_data = db.get_all_usage().unwrap_or_default().into_iter().collect();
        let daily_usage = db.get_all_daily_usage(7).unwrap_or_default();
        let github_cache = db
            .get_all_github_info()
            .unwrap_or_default()
            .into_iter()
            .collect();
        let labels_cache = db.get_all_tool_labels().unwrap_or_default();

        Self {
            usage_data,
            daily_usage,
            github_cache,
            labels_cache,
        }
    }

    /// Get usage data for a tool
    pub fn get_usage(&self, tool_name: &str) -> Option<&ToolUsage> {
        self.usage_data.get(tool_name)
    }

    /// Get GitHub info for a tool, fetching from DB if not cached
    pub fn get_github_info(&mut self, tool_name: &str, db: &Database) -> Option<&GitHubInfo> {
        if !self.github_cache.contains_key(tool_name)
            && let Ok(Some(info)) = db.get_github_info(tool_name)
        {
            self.github_cache.insert(tool_name.to_string(), info);
        }
        self.github_cache.get(tool_name)
    }

    /// Reload labels cache from database
    pub fn reload_labels(&mut self, db: &Database) {
        self.labels_cache = db.get_all_tool_labels().unwrap_or_default();
    }
}

/// Manages command palette input and history
#[derive(Debug, Default)]
pub struct CommandPalette {
    /// Current command input (after ':')
    pub input: String,
    /// Command history for ↑/↓ navigation
    history: Vec<String>,
    /// Current position in history (None = not navigating)
    history_index: Option<usize>,
    /// Temporary storage for current input when navigating history
    history_temp: String,
    /// Maximum history size
    max_history: usize,
}

impl CommandPalette {
    /// Create new command palette with default history size
    pub fn new() -> Self {
        Self {
            max_history: 50,
            ..Default::default()
        }
    }

    /// Navigate to previous command in history
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Start navigating - save current input
                self.history_temp = self.input.clone();
                self.history_index = Some(self.history.len() - 1);
            }
            Some(0) => {
                // Already at oldest, do nothing
            }
            Some(idx) => {
                self.history_index = Some(idx - 1);
            }
        }

        if let Some(idx) = self.history_index {
            self.input = self.history[idx].clone();
        }
    }

    /// Navigate to next command in history
    pub fn history_next(&mut self) {
        match self.history_index {
            None => {
                // Not navigating, do nothing
            }
            Some(idx) if idx + 1 >= self.history.len() => {
                // Back to current input
                self.history_index = None;
                self.input = self.history_temp.clone();
            }
            Some(idx) => {
                self.history_index = Some(idx + 1);
                self.input = self.history[idx + 1].clone();
            }
        }
    }

    /// Add command to history (if not duplicate of last)
    pub fn add_to_history(&mut self, cmd: String) {
        if cmd.is_empty() {
            return;
        }

        // Avoid consecutive duplicates
        if self.history.last() != Some(&cmd) {
            self.history.push(cmd);

            // Trim if over limit
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
    }

    /// Clear history navigation state (after command execution)
    pub fn clear_history_nav(&mut self) {
        self.history_index = None;
        self.history_temp.clear();
    }

    /// Clear input
    pub fn clear(&mut self) {
        self.input.clear();
        self.clear_history_nav();
    }
}
