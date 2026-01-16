//! UI rendering for the TUI

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};

use super::app::{App, InputMode, Tab};
use crate::db::Database;
use crate::icons::source_icon;

/// Catppuccin Mocha color palette
mod colors {
    use ratatui::style::Color;

    pub const BASE: Color = Color::Rgb(30, 30, 46);
    pub const SURFACE0: Color = Color::Rgb(49, 50, 68);
    pub const SURFACE1: Color = Color::Rgb(69, 71, 90);
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200);
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const TEAL: Color = Color::Rgb(148, 226, 213);
    pub const RED: Color = Color::Rgb(243, 139, 168);
}

/// Main render function
pub fn render(frame: &mut Frame, app: &mut App, db: &Database) {
    let area = frame.area();

    // Main layout: header, body, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tabs
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_body(frame, app, db, chunks[1]);
    render_footer(frame, app, chunks[2]);

    // Render overlays (in order of priority)
    if app.show_help {
        render_help_overlay(frame, area);
    }

    if app.show_details_popup {
        render_details_popup(frame, app, db, area);
    }

    // Confirmation dialog takes highest priority
    if app.has_pending_action() {
        render_confirmation_dialog(frame, app, area);
    }

    // Loading overlay takes absolute highest priority
    if app.has_background_op() {
        render_loading_overlay(frame, app, area);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.tab {
                Style::default().fg(colors::BLUE).bold()
            } else {
                Style::default().fg(colors::SUBTEXT0)
            };
            Line::from(Span::styled(format!(" {} ", t.title()), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::SURFACE1))
                .title(Span::styled(
                    " hoard ",
                    Style::default().fg(colors::MAUVE).bold(),
                )),
        )
        .highlight_style(Style::default().fg(colors::BLUE))
        .select(app.tab.index());

    frame.render_widget(tabs, area);
}

fn render_body(frame: &mut Frame, app: &mut App, db: &Database, area: Rect) {
    // Responsive layout: side-by-side for wide terminals, stacked for narrow
    let min_width_for_split = 80;

    // Bundles tab has its own rendering
    if app.tab == super::app::Tab::Bundles {
        if area.width >= min_width_for_split {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            render_bundle_list(frame, app, chunks[0]);
            render_bundle_details(frame, app, db, chunks[1]);
        } else {
            render_bundle_list(frame, app, area);
        }
        return;
    }

    if area.width >= min_width_for_split {
        // Wide terminal: side-by-side layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        render_tool_list(frame, app, chunks[0]);
        render_details(frame, app, db, chunks[1]);
    } else {
        // Narrow terminal: list only (details on Enter in future)
        render_tool_list(frame, app, area);
    }
}

fn render_tool_list(frame: &mut Frame, app: &App, area: Rect) {
    // Special handling for Updates tab when not checked yet
    if app.tab == super::app::Tab::Updates && !app.updates_checked {
        let message = if app.updates_loading {
            "Checking for updates..."
        } else {
            "Press 'r' to check for updates"
        };
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(colors::SUBTEXT0))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::SURFACE1))
                    .title(Span::styled(" Updates ", Style::default().fg(colors::TEXT))),
            );
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tools
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            // Selection checkbox
            let selected = app.is_selected(&tool.name);
            let checkbox = if selected { "☑" } else { "☐" };
            let checkbox_color = if selected {
                colors::BLUE
            } else {
                colors::SURFACE1
            };

            // Source icon
            let src_icon = source_icon(&tool.source.to_string());

            // For Updates tab, show version info instead of usage
            let extra_info = if app.tab == super::app::Tab::Updates {
                if let Some(update) = app.get_update(&tool.name) {
                    format!(" {} → {}", update.current, update.latest)
                } else {
                    String::new()
                }
            } else {
                // Usage count for other tabs
                let usage = app.get_usage(&tool.name).map(|u| u.use_count).unwrap_or(0);
                if usage > 0 {
                    format!(" ({usage})")
                } else {
                    String::new()
                }
            };

            // Status indicator
            let (status, status_color) = if app.tab == super::app::Tab::Updates {
                // Show update indicator for Updates tab
                ("↑", colors::YELLOW)
            } else if tool.is_installed {
                ("●", colors::GREEN)
            } else {
                ("○", colors::SUBTEXT0)
            };

            let extra_color = if app.tab == super::app::Tab::Updates {
                colors::YELLOW
            } else {
                colors::SUBTEXT0
            };

            let content = Line::from(vec![
                Span::styled(format!("{checkbox} "), Style::default().fg(checkbox_color)),
                Span::styled(format!("{src_icon} "), Style::default()),
                Span::styled(format!("{status} "), Style::default().fg(status_color)),
                Span::styled(&tool.name, Style::default().fg(colors::TEXT)),
                Span::styled(extra_info, Style::default().fg(extra_color)),
            ]);

            let style = if i == app.selected_index {
                Style::default().bg(colors::SURFACE0)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    // Title with count, selection count, and sort indicator
    let selection_info = if app.selection_count() > 0 {
        format!(" ({} selected)", app.selection_count())
    } else {
        String::new()
    };

    let title_text = if app.tab == super::app::Tab::Updates {
        format!(" Updates [{}]{} ", app.tools.len(), selection_info)
    } else {
        format!(
            " Tools [{}]{} ({}↕) ",
            app.tools.len(),
            selection_info,
            app.sort_by.label()
        )
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::SURFACE1))
                .title(Span::styled(title_text, Style::default().fg(colors::TEXT))),
        )
        .highlight_style(
            Style::default()
                .bg(colors::SURFACE0)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    // Scroll list to keep selection visible
    let visible_height = area.height.saturating_sub(2) as usize; // Subtract border
    if visible_height > 0 {
        *state.offset_mut() = app.selected_index.saturating_sub(visible_height / 2);
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame, app: &mut App, db: &Database, area: Rect) {
    // Clone selected tool to avoid borrow issues
    let tool = app.selected_tool().cloned();

    let content = if let Some(tool) = tool {
        // Pre-fetch GitHub info while we have mutable access
        let _ = app.get_github_info(&tool.name, db);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(tool.name.clone(), Style::default().fg(colors::BLUE).bold()),
            ]),
            Line::from(""),
        ];

        // Description
        if let Some(desc) = &tool.description {
            lines.push(Line::from(Span::styled(
                "Description:",
                Style::default().fg(colors::SUBTEXT0),
            )));
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(colors::TEXT),
            )));
            lines.push(Line::from(""));
        }

        // Source and install command
        let src_icon = source_icon(&tool.source.to_string());
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled(
                format!("{src_icon} {}", tool.source),
                Style::default().fg(colors::PEACH),
            ),
        ]));

        if let Some(cmd) = &tool.install_command {
            lines.push(Line::from(vec![
                Span::styled("Install: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(cmd.clone(), Style::default().fg(colors::GREEN)),
            ]));
        }

        // Binary name
        if let Some(binary) = &tool.binary_name {
            lines.push(Line::from(vec![
                Span::styled("Binary: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(binary.clone(), Style::default().fg(colors::TEXT)),
            ]));
        }

        // Category
        if let Some(category) = &tool.category {
            lines.push(Line::from(vec![
                Span::styled("Category: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(category.clone(), Style::default().fg(colors::MAUVE)),
            ]));
        }

        lines.push(Line::from(""));

        // Usage statistics
        if let Some(usage) = app.usage_data.get(&tool.name) {
            lines.push(Line::from(Span::styled(
                "Usage:",
                Style::default()
                    .fg(colors::SUBTEXT0)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("  Invocations: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(
                    format!("{}", usage.use_count),
                    Style::default().fg(colors::TEAL),
                ),
            ]));
            if let Some(last) = &usage.last_used {
                lines.push(Line::from(vec![
                    Span::styled("  Last used: ", Style::default().fg(colors::SUBTEXT0)),
                    Span::styled(last.clone(), Style::default().fg(colors::TEXT)),
                ]));
            }
            lines.push(Line::from(""));
        }

        // GitHub info (already fetched above)
        if let Some(gh) = app.github_cache.get(&tool.name) {
            lines.push(Line::from(Span::styled(
                "GitHub:",
                Style::default()
                    .fg(colors::SUBTEXT0)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("  ★ Stars: ", Style::default().fg(colors::YELLOW)),
                Span::styled(format_stars(gh.stars), Style::default().fg(colors::YELLOW)),
            ]));
            if let Some(lang) = &gh.language {
                lines.push(Line::from(vec![
                    Span::styled("  Language: ", Style::default().fg(colors::SUBTEXT0)),
                    Span::styled(lang.clone(), Style::default().fg(colors::PEACH)),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  Repo: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(
                    format!("{}/{}", gh.repo_owner, gh.repo_name),
                    Style::default().fg(colors::BLUE),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // Status
        let status_text = if tool.is_installed {
            "Installed"
        } else {
            "Not installed"
        };
        let status_color = if tool.is_installed {
            colors::GREEN
        } else {
            colors::YELLOW
        };
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]));

        if tool.is_favorite {
            lines.push(Line::from(Span::styled(
                "★ Favorite",
                Style::default().fg(colors::YELLOW),
            )));
        }

        Text::from(lines)
    } else {
        Text::from(Span::styled(
            "No tool selected",
            Style::default().fg(colors::SUBTEXT0),
        ))
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::SURFACE1))
                .title(Span::styled(" Details ", Style::default().fg(colors::TEXT))),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

/// Format star count (e.g., 1234 -> "1.2K")
fn format_stars(stars: i64) -> String {
    if stars >= 1000 {
        format!("{:.1}K", stars as f64 / 1000.0)
    } else {
        stars.to_string()
    }
}

fn render_bundle_list(frame: &mut Frame, app: &App, area: Rect) {
    if app.bundles.is_empty() {
        let message =
            "No bundles yet. Create one with: hoards bundle create <name> --tools tool1,tool2";
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(colors::SUBTEXT0))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors::SURFACE1))
                    .title(Span::styled(" Bundles ", Style::default().fg(colors::TEXT))),
            );
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .bundles
        .iter()
        .enumerate()
        .map(|(i, bundle)| {
            let tool_count = bundle.tools.len();
            let count_str = if tool_count == 1 {
                "1 tool".to_string()
            } else {
                format!("{} tools", tool_count)
            };

            let content = Line::from(vec![
                Span::styled("📦 ", Style::default()),
                Span::styled(&bundle.name, Style::default().fg(colors::TEXT).bold()),
                Span::styled(
                    format!(" ({})", count_str),
                    Style::default().fg(colors::SUBTEXT0),
                ),
            ]);

            let style = if i == app.bundle_selected {
                Style::default().bg(colors::SURFACE0)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let title = format!(" Bundles [{}] ", app.bundles.len());

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::SURFACE1))
                .title(Span::styled(title, Style::default().fg(colors::TEXT))),
        )
        .highlight_style(
            Style::default()
                .bg(colors::SURFACE0)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.bundle_selected));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_bundle_details(frame: &mut Frame, app: &App, db: &Database, area: Rect) {
    let content = if let Some(bundle) = app.bundles.get(app.bundle_selected) {
        let mut lines = vec![
            Line::from(Span::styled(
                &bundle.name,
                Style::default()
                    .fg(colors::BLUE)
                    .bold()
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(""),
        ];

        // Description
        if let Some(desc) = &bundle.description {
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(colors::TEXT),
            )));
            lines.push(Line::from(""));
        }

        // Tool count
        lines.push(Line::from(vec![
            Span::styled("Tools: ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled(
                format!("{}", bundle.tools.len()),
                Style::default().fg(colors::TEAL),
            ),
        ]));
        lines.push(Line::from(""));

        // List tools with installation status
        lines.push(Line::from(Span::styled(
            "─── Contents ───",
            Style::default().fg(colors::SURFACE1),
        )));

        for tool_name in &bundle.tools {
            // Check if tool is installed
            let is_installed = db
                .get_tool_by_name(tool_name)
                .ok()
                .flatten()
                .map(|t| t.is_installed)
                .unwrap_or(false);

            let (status, status_color) = if is_installed {
                ("●", colors::GREEN)
            } else {
                ("○", colors::SUBTEXT0)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", status), Style::default().fg(status_color)),
                Span::styled(tool_name.clone(), Style::default().fg(colors::TEXT)),
            ]));
        }

        lines.push(Line::from(""));

        // Install hint
        let not_installed: Vec<_> = bundle
            .tools
            .iter()
            .filter(|name| {
                !db.get_tool_by_name(name)
                    .ok()
                    .flatten()
                    .map(|t| t.is_installed)
                    .unwrap_or(false)
            })
            .collect();

        if !not_installed.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(
                    "Press 'i' to install {} missing tool(s)",
                    not_installed.len()
                ),
                Style::default().fg(colors::GREEN),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "All tools installed ✓",
                Style::default().fg(colors::GREEN),
            )));
        }

        Text::from(lines)
    } else {
        Text::from("No bundle selected")
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::SURFACE1))
                .title(Span::styled(
                    " Bundle Details ",
                    Style::default().fg(colors::TEXT),
                )),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    // Show status message if present (takes priority)
    if let Some(status) = &app.status_message {
        let color = if status.is_error {
            colors::RED
        } else {
            colors::GREEN
        };
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(&status.text, Style::default().fg(color)),
        ]))
        .style(Style::default().bg(colors::SURFACE0));

        frame.render_widget(footer, area);
        return;
    }

    let mode_text = match app.input_mode {
        InputMode::Normal => {
            let mut spans = vec![
                Span::styled(" j/k", Style::default().fg(colors::BLUE)),
                Span::styled(" nav ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(" Space", Style::default().fg(colors::BLUE)),
                Span::styled(" select ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(" i", Style::default().fg(colors::GREEN)),
                Span::styled(" install ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(" D", Style::default().fg(colors::RED)),
                Span::styled(" uninstall ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(" u", Style::default().fg(colors::YELLOW)),
                Span::styled(" update ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(" ?", Style::default().fg(colors::BLUE)),
                Span::styled(" help", Style::default().fg(colors::SUBTEXT0)),
            ];

            // Show selection count or filter
            if app.selection_count() > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(colors::SURFACE1)));
                spans.push(Span::styled(
                    format!("{} selected", app.selection_count()),
                    Style::default().fg(colors::BLUE),
                ));
            } else if !app.search_query.is_empty() {
                spans.push(Span::styled(" │ ", Style::default().fg(colors::SURFACE1)));
                spans.push(Span::styled("filter:", Style::default().fg(colors::YELLOW)));
                spans.push(Span::styled(
                    &app.search_query,
                    Style::default().fg(colors::TEXT),
                ));
            }

            spans
        }
        InputMode::Search => {
            vec![
                Span::styled(" Search: ", Style::default().fg(colors::YELLOW)),
                Span::styled(&app.search_query, Style::default().fg(colors::TEXT)),
                Span::styled("│", Style::default().fg(colors::BLUE)), // Cursor
                Span::styled("  Enter", Style::default().fg(colors::BLUE)),
                Span::styled(" apply ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(" Esc", Style::default().fg(colors::BLUE)),
                Span::styled(" cancel", Style::default().fg(colors::SUBTEXT0)),
            ]
        }
    };

    let footer = Paragraph::new(Line::from(mode_text)).style(Style::default().bg(colors::SURFACE0));

    frame.render_widget(footer, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    // Center the help popup
    let popup_area = centered_rect(60, 80, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(colors::MAUVE).bold(),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().fg(colors::BLUE).bold(),
        )]),
        Line::from(vec![
            Span::styled("  j/↓      ", Style::default().fg(colors::YELLOW)),
            Span::styled("Move down", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  k/↑      ", Style::default().fg(colors::YELLOW)),
            Span::styled("Move up", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  g        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Go to top", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  G        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Go to bottom", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d   ", Style::default().fg(colors::YELLOW)),
            Span::styled("Page down", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u   ", Style::default().fg(colors::YELLOW)),
            Span::styled("Page up", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tabs",
            Style::default().fg(colors::BLUE).bold(),
        )]),
        Line::from(vec![
            Span::styled("  1-4      ", Style::default().fg(colors::YELLOW)),
            Span::styled("Switch to tab", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Tab/]    ", Style::default().fg(colors::YELLOW)),
            Span::styled("Next tab", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  S-Tab/[  ", Style::default().fg(colors::YELLOW)),
            Span::styled("Previous tab", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Selection",
            Style::default().fg(colors::BLUE).bold(),
        )]),
        Line::from(vec![
            Span::styled("  Space    ", Style::default().fg(colors::YELLOW)),
            Span::styled("Toggle selection", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+a   ", Style::default().fg(colors::YELLOW)),
            Span::styled("Select all", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  x        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Clear selection", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default().fg(colors::BLUE).bold(),
        )]),
        Line::from(vec![
            Span::styled("  i        ", Style::default().fg(colors::GREEN)),
            Span::styled("Install tool(s)", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  D        ", Style::default().fg(colors::RED)),
            Span::styled("Uninstall tool(s)", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  u        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Update tool(s)", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(colors::YELLOW)),
            Span::styled("Show details popup", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  /        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Search/filter tools", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  s        ", Style::default().fg(colors::YELLOW)),
            Span::styled(
                "Cycle sort (name/usage/recent)",
                Style::default().fg(colors::TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Esc      ", Style::default().fg(colors::YELLOW)),
            Span::styled("Clear search filter", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  r        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Refresh list", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  ?        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Toggle help", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  q        ", Style::default().fg(colors::YELLOW)),
            Span::styled("Quit", Style::default().fg(colors::TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(colors::SUBTEXT0),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::MAUVE))
                .title(Span::styled(
                    " Help ",
                    Style::default().fg(colors::MAUVE).bold(),
                ))
                .style(Style::default().bg(colors::BASE)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(help, popup_area);
}

fn render_details_popup(frame: &mut Frame, app: &mut App, db: &Database, area: Rect) {
    let popup_area = centered_rect(70, 80, area);

    let content = if let Some(tool) = app.selected_tool().cloned() {
        // Pre-fetch GitHub info
        let _ = app.get_github_info(&tool.name, db);

        let mut lines = vec![
            Line::from(Span::styled(
                tool.name.clone(),
                Style::default()
                    .fg(colors::BLUE)
                    .bold()
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(""),
        ];

        // Description
        if let Some(desc) = &tool.description {
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(colors::TEXT),
            )));
            lines.push(Line::from(""));
        }

        // Source and install
        let src_icon = source_icon(&tool.source.to_string());
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled(
                format!("{src_icon} {}", tool.source),
                Style::default().fg(colors::PEACH),
            ),
        ]));

        if let Some(cmd) = &tool.install_command {
            lines.push(Line::from(vec![
                Span::styled("Install: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(cmd.clone(), Style::default().fg(colors::GREEN)),
            ]));
        }

        if let Some(binary) = &tool.binary_name {
            lines.push(Line::from(vec![
                Span::styled("Binary: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(binary.clone(), Style::default().fg(colors::TEXT)),
            ]));
        }

        if let Some(category) = &tool.category {
            lines.push(Line::from(vec![
                Span::styled("Category: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(category.clone(), Style::default().fg(colors::MAUVE)),
            ]));
        }

        lines.push(Line::from(""));

        // Usage
        if let Some(usage) = app.usage_data.get(&tool.name) {
            lines.push(Line::from(vec![
                Span::styled("Usage: ", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(
                    format!("{} invocations", usage.use_count),
                    Style::default().fg(colors::TEAL),
                ),
            ]));
            if let Some(last) = &usage.last_used {
                lines.push(Line::from(vec![
                    Span::styled("Last used: ", Style::default().fg(colors::SUBTEXT0)),
                    Span::styled(last.clone(), Style::default().fg(colors::TEXT)),
                ]));
            }
        }

        // GitHub
        if let Some(gh) = app.github_cache.get(&tool.name) {
            lines.push(Line::from(vec![
                Span::styled("★ Stars: ", Style::default().fg(colors::YELLOW)),
                Span::styled(format_stars(gh.stars), Style::default().fg(colors::YELLOW)),
                Span::styled("  ", Style::default()),
                Span::styled(&gh.repo_owner, Style::default().fg(colors::SUBTEXT0)),
                Span::styled("/", Style::default().fg(colors::SUBTEXT0)),
                Span::styled(&gh.repo_name, Style::default().fg(colors::BLUE)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter or Esc to close",
            Style::default().fg(colors::SUBTEXT0),
        )));

        Text::from(lines)
    } else {
        Text::from("No tool selected")
    };

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::BLUE))
                .title(Span::styled(
                    " Details ",
                    Style::default().fg(colors::BLUE).bold(),
                ))
                .style(Style::default().bg(colors::BASE)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

fn render_loading_overlay(frame: &mut Frame, app: &App, area: Rect) {
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
            Style::default().fg(colors::BLUE).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            progress_bar,
            Style::default().fg(colors::YELLOW),
        )),
        Line::from(""),
    ];

    // Show found count if any
    if progress.found_count > 0 {
        lines.push(Line::from(vec![
            Span::styled("Found: ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled(
                format!("{} update(s)", progress.found_count),
                Style::default().fg(colors::GREEN),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "Please wait...",
        Style::default().fg(colors::SUBTEXT0),
    )));

    let content = Text::from(lines);

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::YELLOW))
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(colors::YELLOW).bold(),
                ))
                .style(Style::default().bg(colors::BASE)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

fn render_confirmation_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(50, 30, area);

    let (title, description, color) = if let Some(action) = &app.pending_action {
        match action {
            super::app::PendingAction::Install(tools) => {
                let desc = action.description();
                let tool_list = if tools.len() <= 3 {
                    tools.join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        tools[..2].join(", "),
                        tools.len() - 2
                    )
                };
                (
                    " Install ",
                    format!("{}\n\nTools: {}", desc, tool_list),
                    colors::GREEN,
                )
            }
            super::app::PendingAction::Uninstall(tools) => {
                let desc = action.description();
                let tool_list = if tools.len() <= 3 {
                    tools.join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        tools[..2].join(", "),
                        tools.len() - 2
                    )
                };
                (
                    " Uninstall ",
                    format!("{}\n\nTools: {}", desc, tool_list),
                    colors::RED,
                )
            }
            super::app::PendingAction::Update(tools) => {
                let desc = action.description();
                let tool_list = if tools.len() <= 3 {
                    tools.join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        tools[..2].join(", "),
                        tools.len() - 2
                    )
                };
                (
                    " Update ",
                    format!("{}\n\nTools: {}", desc, tool_list),
                    colors::YELLOW,
                )
            }
        }
    } else {
        return;
    };

    let content = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(description, Style::default().fg(colors::TEXT))),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled("y", Style::default().fg(colors::GREEN).bold()),
            Span::styled(" to confirm, ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled("n", Style::default().fg(colors::RED).bold()),
            Span::styled(" or ", Style::default().fg(colors::SUBTEXT0)),
            Span::styled("Esc", Style::default().fg(colors::YELLOW).bold()),
            Span::styled(" to cancel", Style::default().fg(colors::SUBTEXT0)),
        ]),
    ]);

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
                .title(Span::styled(title, Style::default().fg(color).bold()))
                .style(Style::default().bg(colors::BASE)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
