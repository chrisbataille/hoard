//! Helper functions for UI rendering
//!
//! This module contains utility functions used across the UI, including:
//! - Date/time formatting
//! - Sparkline generation
//! - Color helpers
//! - Health indicators

use chrono::{DateTime, Datelike, Local, Utc};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::super::app::fuzzy_match_positions;
use super::super::theme::Theme;

/// Get a consistent color for a label based on its hash
pub fn label_color(label: &str, theme: &Theme) -> Color {
    let colors = [
        theme.blue,
        theme.green,
        theme.yellow,
        theme.mauve,
        theme.peach,
        theme.teal,
        theme.red,
    ];
    let hash: usize = label.bytes().map(|b| b as usize).sum();
    colors[hash % colors.len()]
}

/// Format an RFC3339 datetime string to a friendly local time format
/// e.g., "Today at 3:45 PM", "Yesterday at 10:30 AM", "Jan 15 at 2:00 PM", "Jan 15, 2025"
pub fn format_friendly_datetime(rfc3339: &str) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339) else {
        return rfc3339.to_string(); // Fallback to raw if parsing fails
    };

    let local_dt = dt.with_timezone(&Local);
    let now = Local::now();
    let today = now.date_naive();
    let dt_date = local_dt.date_naive();

    let time_str = local_dt.format("%-I:%M %p").to_string();

    if dt_date == today {
        format!("Today at {}", time_str)
    } else if dt_date == today.pred_opt().unwrap_or(today) {
        format!("Yesterday at {}", time_str)
    } else if (today - dt_date).num_days() < 7 {
        // Within a week: "Mon at 3:45 PM"
        format!("{} at {}", local_dt.format("%a"), time_str)
    } else if local_dt.year() == now.year() {
        // Same year: "Jan 15 at 3:45 PM"
        format!("{} at {}", local_dt.format("%b %-d"), time_str)
    } else {
        // Different year: "Jan 15, 2025"
        local_dt.format("%b %-d, %Y").to_string()
    }
}

/// Format a timestamp as relative time (e.g., "5m", "2h", "3d")
pub fn format_relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_seconds() < 60 {
        "now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{}d", duration.num_days())
    } else if duration.num_weeks() < 4 {
        format!("{}w", duration.num_weeks())
    } else {
        format!("{}mo", duration.num_days() / 30)
    }
}

/// Create spans for a tool name with fuzzy match highlighting
pub fn highlight_matches(
    name: &str,
    query: &str,
    normal: Color,
    highlight: Color,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(name.to_string(), Style::default().fg(normal))];
    }

    if let Some((_, positions)) = fuzzy_match_positions(query, name) {
        let chars: Vec<char> = name.chars().collect();
        let mut spans = Vec::new();
        let mut current_span = String::new();
        let mut in_highlight = false;

        for (i, c) in chars.iter().enumerate() {
            let should_highlight = positions.contains(&i);

            if should_highlight != in_highlight {
                // State changed, emit current span
                if !current_span.is_empty() {
                    let color = if in_highlight { highlight } else { normal };
                    spans.push(Span::styled(
                        current_span.clone(),
                        Style::default().fg(color),
                    ));
                    current_span.clear();
                }
                in_highlight = should_highlight;
            }
            current_span.push(*c);
        }

        // Emit final span
        if !current_span.is_empty() {
            let color = if in_highlight { highlight } else { normal };
            spans.push(Span::styled(current_span, Style::default().fg(color)));
        }

        spans
    } else {
        vec![Span::styled(name.to_string(), Style::default().fg(normal))]
    }
}

/// Generate a sparkline string from usage data
/// Uses Unicode block elements: ▁▂▃▄▅▆▇█
pub fn sparkline(data: &[i64]) -> String {
    if data.is_empty() || data.iter().all(|&x| x == 0) {
        return "·······".to_string(); // No data indicator
    }

    let max = *data.iter().max().unwrap_or(&1).max(&1);
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    data.iter()
        .map(|&value| {
            if value == 0 {
                ' '
            } else {
                // Scale to 0-7 range
                let idx = ((value as f64 / max as f64) * 7.0).round() as usize;
                blocks[idx.min(7)]
            }
        })
        .collect()
}

/// Determine health status based on usage recency
/// Returns (indicator, color) tuple
pub fn health_indicator(
    last_used: Option<&str>,
    use_count: i64,
    theme: &Theme,
) -> (&'static str, Color) {
    // Parse last_used timestamp
    let days_since_use = last_used
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| {
            let now = Utc::now();
            let used = dt.with_timezone(&Utc);
            (now - used).num_days()
        });

    match (use_count, days_since_use) {
        // Never used - red
        (0, _) => ("●", theme.red),
        // Used within last 7 days - green
        (_, Some(days)) if days < 7 => ("●", theme.green),
        // Used within last 30 days - yellow
        (_, Some(days)) if days < 30 => ("●", theme.yellow),
        // Used but more than 30 days ago - red
        (_, Some(_)) => ("●", theme.red),
        // Has usage but no timestamp (legacy data) - green
        (_, None) => ("●", theme.green),
    }
}

/// Format star count with K/M suffixes
pub fn format_stars(stars: i64) -> String {
    if stars >= 1_000_000 {
        format!("{:.1}M", stars as f64 / 1_000_000.0)
    } else if stars >= 1_000 {
        format!("{:.1}K", stars as f64 / 1_000.0)
    } else {
        stars.to_string()
    }
}

/// Custom stylesheet for markdown rendering that uses the TUI theme
#[derive(Clone)]
pub struct ThemedStyleSheet {
    pub heading_color: Color,
    pub code_color: Color,
    pub link_color: Color,
    pub blockquote_color: Color,
    pub meta_color: Color,
}

impl ThemedStyleSheet {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            heading_color: theme.blue,
            code_color: theme.green,
            link_color: theme.teal,
            blockquote_color: theme.subtext0,
            meta_color: theme.subtext0,
        }
    }
}

impl tui_markdown::StyleSheet for ThemedStyleSheet {
    fn heading(&self, level: u8) -> Style {
        let modifier = if level == 1 {
            Modifier::BOLD | Modifier::UNDERLINED
        } else {
            Modifier::BOLD
        };
        Style::default()
            .fg(self.heading_color)
            .add_modifier(modifier)
    }

    fn code(&self) -> Style {
        Style::default().fg(self.code_color)
    }

    fn link(&self) -> Style {
        Style::default()
            .fg(self.link_color)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::default()
            .fg(self.blockquote_color)
            .add_modifier(Modifier::ITALIC)
    }

    fn heading_meta(&self) -> Style {
        Style::default().fg(self.meta_color)
    }

    fn metadata_block(&self) -> Style {
        Style::default().fg(self.meta_color)
    }
}
