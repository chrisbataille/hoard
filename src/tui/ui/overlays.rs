//! Overlay rendering
//!
//! This module handles rendering of overlay widgets like help, loading, and notifications.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::super::app::{App, NotificationLevel};
use super::super::theme::Theme;
use super::dialogs::centered_rect;

/// Render the help overlay
pub fn render_help_overlay(frame: &mut Frame, theme: &Theme, area: Rect) {
    // Center the help popup
    let popup_area = centered_rect(60, 80, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(theme.mauve).bold(),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  j/↓      ", Style::default().fg(theme.yellow)),
            Span::styled("Move down", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  k/↑      ", Style::default().fg(theme.yellow)),
            Span::styled("Move up", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  g        ", Style::default().fg(theme.yellow)),
            Span::styled("Go to top", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  G        ", Style::default().fg(theme.yellow)),
            Span::styled("Go to bottom", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  n/N      ", Style::default().fg(theme.yellow)),
            Span::styled("Next/prev match (wrap)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  f<char>  ", Style::default().fg(theme.peach)),
            Span::styled("Jump to letter", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d   ", Style::default().fg(theme.yellow)),
            Span::styled("Page down", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u   ", Style::default().fg(theme.yellow)),
            Span::styled("Page up", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tabs",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  1-4      ", Style::default().fg(theme.yellow)),
            Span::styled("Switch to tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Tab/]    ", Style::default().fg(theme.yellow)),
            Span::styled("Next tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  S-Tab/[  ", Style::default().fg(theme.yellow)),
            Span::styled("Previous tab", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Selection",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  Space    ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle selection", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+a   ", Style::default().fg(theme.yellow)),
            Span::styled("Select all", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  x        ", Style::default().fg(theme.yellow)),
            Span::styled("Clear selection", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  *        ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle favorite", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  F        ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle favorites filter", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  i        ", Style::default().fg(theme.green)),
            Span::styled("Install tool(s)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  D        ", Style::default().fg(theme.red)),
            Span::styled("Uninstall tool(s)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  u        ", Style::default().fg(theme.yellow)),
            Span::styled("Update tool(s)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(theme.yellow)),
            Span::styled("Show details popup", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  /        ", Style::default().fg(theme.yellow)),
            Span::styled("Search/filter tools", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  :        ", Style::default().fg(theme.mauve)),
            Span::styled(
                "Command palette (vim-style)",
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("  s        ", Style::default().fg(theme.yellow)),
            Span::styled(
                "Cycle sort (name/usage/recent)",
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Esc      ", Style::default().fg(theme.yellow)),
            Span::styled("Clear search filter", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  r        ", Style::default().fg(theme.yellow)),
            Span::styled("Refresh list", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  t        ", Style::default().fg(theme.teal)),
            Span::styled("Cycle theme", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+z   ", Style::default().fg(theme.peach)),
            Span::styled("Undo", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+y   ", Style::default().fg(theme.peach)),
            Span::styled("Redo", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Mouse",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  Click    ", Style::default().fg(theme.green)),
            Span::styled("Select item / switch tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  R-Click  ", Style::default().fg(theme.green)),
            Span::styled("Toggle selection", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Scroll   ", Style::default().fg(theme.green)),
            Span::styled("Navigate list", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  ?        ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle help", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  q        ", Style::default().fg(theme.yellow)),
            Span::styled("Quit", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(theme.subtext0),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.mauve))
                .title(Span::styled(
                    " Help ",
                    Style::default().fg(theme.mauve).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(help, popup_area);
}

/// Render the loading overlay
pub fn render_loading_overlay(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(50, 30, area);

    let title = app
        .background_op
        .as_ref()
        .map(|op| op.title())
        .unwrap_or("Working");

    let progress = &app.loading_progress;

    // Build progress bar
    let bar_width = 30;
    let filled = if progress.total_steps > 0 {
        (progress.current_step * bar_width) / progress.total_steps
    } else {
        0
    };
    let empty = bar_width - filled;
    let progress_bar = format!(
        "[{}{}] {}/{}",
        "█".repeat(filled),
        "░".repeat(empty),
        progress.current_step,
        progress.total_steps
    );

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            &progress.step_name,
            Style::default().fg(theme.blue).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            progress_bar,
            Style::default().fg(theme.yellow),
        )),
        Line::from(""),
    ];

    // Show found count if any
    if progress.found_count > 0 {
        lines.push(Line::from(vec![
            Span::styled("Found: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{} update(s)", progress.found_count),
                Style::default().fg(theme.green),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "Please wait...",
        Style::default().fg(theme.subtext0),
    )));

    let content = Text::from(lines);

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.yellow))
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(theme.yellow).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// Render toast notifications in top-right corner
pub fn render_notifications(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.notifications.is_empty() {
        return;
    }

    // Stack notifications from top-right
    // Use 60% of screen width for notifications, min 40, max 80
    let max_width = (area.width * 60 / 100)
        .clamp(40, 80)
        .min(area.width.saturating_sub(4));
    let mut y_offset = 1u16;

    for notification in &app.notifications {
        let (border_color, icon) = match notification.level {
            NotificationLevel::Info => (theme.blue, "ℹ"),
            NotificationLevel::Warning => (theme.yellow, "⚠"),
            NotificationLevel::Error => (theme.red, "✗"),
        };

        // Calculate wrapped lines
        let inner_width = (max_width as usize).saturating_sub(4); // borders + padding
        let text = &notification.text;

        // Word wrap the text
        let mut lines: Vec<Line> = Vec::new();
        let mut current_line = format!("{} ", icon);

        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > inner_width {
                lines.push(Line::from(Span::styled(
                    current_line.clone(),
                    Style::default().fg(if lines.is_empty() {
                        border_color
                    } else {
                        theme.text
                    }),
                )));
                current_line = format!("  {}", word); // indent continuation
            } else {
                if !current_line.ends_with(' ') && !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
        }
        if !current_line.is_empty() {
            lines.push(Line::from(Span::styled(
                current_line,
                Style::default().fg(if lines.is_empty() {
                    border_color
                } else {
                    theme.text
                }),
            )));
        }

        // Calculate height needed (lines + 2 for borders)
        let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(y_offset));

        if y_offset + height > area.height {
            break; // No more room
        }

        let toast_area = Rect {
            x: area.width.saturating_sub(max_width + 2),
            y: y_offset,
            width: max_width,
            height,
        };

        let content = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(theme.surface0)),
            )
            .wrap(ratatui::widgets::Wrap { trim: false });

        frame.render_widget(Clear, toast_area);
        frame.render_widget(content, toast_area);

        y_offset += height + 1; // toast height + gap
    }
}
