//! Tool list and details rendering
//!
//! This module handles rendering of the tool list and tool details panels.

use chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};

use super::super::app::{App, Tab};
use super::super::theme::Theme;
use super::helpers::{
    format_friendly_datetime, format_stars, health_indicator, highlight_matches, label_color,
    sparkline,
};
use crate::db::Database;
use crate::icons::source_icon;

/// Render empty state for Updates tab when updates haven't been checked
pub fn render_updates_empty_state(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let message = if app.updates_loading {
        "Checking for updates..."
    } else {
        "Press 'r' to check for updates"
    };
    let paragraph = Paragraph::new(message)
        .style(Style::default().fg(theme.subtext0))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(" Updates ", Style::default().fg(theme.text))),
        );
    frame.render_widget(paragraph, area);
}

/// Build extra info and sparkline for a tool item
fn build_tool_extra_info(app: &App, tool: &crate::models::Tool) -> (String, String) {
    if app.tab == Tab::Updates {
        let info = if let Some(update) = app.get_update(&tool.name) {
            format!(" {} → {}", update.current, update.latest)
        } else {
            String::new()
        };
        (info, String::new())
    } else {
        let usage = app.get_usage(&tool.name).map(|u| u.use_count).unwrap_or(0);
        let daily = app.cache.daily_usage.get(&tool.name);
        let spark_str = daily.map(|d| sparkline(d)).unwrap_or_default();
        let info = if usage > 0 {
            format!(" ({usage})")
        } else {
            String::new()
        };
        (info, spark_str)
    }
}

/// Get status indicator for a tool based on its state
fn get_tool_status_indicator(
    app: &App,
    tool: &crate::models::Tool,
    theme: &Theme,
) -> (&'static str, ratatui::style::Color) {
    if app.tab == Tab::Updates {
        ("↑", theme.yellow)
    } else if !tool.is_installed {
        ("○", theme.subtext0)
    } else {
        let usage = app.get_usage(&tool.name);
        let use_count = usage.as_ref().map(|u| u.use_count).unwrap_or(0);
        let last_used = usage.as_ref().and_then(|u| u.last_used.as_deref());
        health_indicator(last_used, use_count, theme)
    }
}

/// Build a single tool list item
fn build_tool_list_item(
    app: &App,
    tool: &crate::models::Tool,
    index: usize,
    theme: &Theme,
) -> ListItem<'static> {
    let selected = app.is_selected(&tool.name);
    let checkbox = if selected { "☑" } else { "☐" };
    let checkbox_color = if selected { theme.blue } else { theme.surface1 };

    let src_icon = source_icon(&tool.source.to_string());
    let (extra_info, spark) = build_tool_extra_info(app, tool);
    let (status, status_color) = get_tool_status_indicator(app, tool, theme);
    let extra_color = if app.tab == Tab::Updates {
        theme.yellow
    } else {
        theme.subtext0
    };

    let spark_span = if spark.is_empty() {
        Span::raw("")
    } else {
        Span::styled(format!(" {spark}"), Style::default().fg(theme.teal))
    };

    let stars_span = app
        .cache
        .github_cache
        .get(&tool.name)
        .filter(|gh| gh.stars > 0)
        .map(|gh| {
            Span::styled(
                format!(" ★ {}", format_stars(gh.stars)),
                Style::default().fg(theme.yellow),
            )
        })
        .unwrap_or_else(|| Span::raw(""));

    let mut spans = vec![
        Span::styled(format!("{checkbox} "), Style::default().fg(checkbox_color)),
        Span::styled(format!("{src_icon} "), Style::default()),
        Span::styled(format!("{status} "), Style::default().fg(status_color)),
    ];
    spans.extend(highlight_matches(
        &tool.name,
        &app.search_query,
        theme.text,
        theme.yellow,
    ));
    spans.push(stars_span);
    spans.push(Span::styled(extra_info, Style::default().fg(extra_color)));
    spans.push(spark_span);

    let style = if index == app.selected_index {
        Style::default().bg(theme.surface0)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(spans)).style(style)
}

/// Build the list title with count and selection info
fn build_tool_list_title(app: &App) -> String {
    let selection_info = if app.selection_count() > 0 {
        format!(" ({} selected)", app.selection_count())
    } else {
        String::new()
    };

    if app.tab == Tab::Updates {
        format!(" Updates [{}]{} ", app.tools.len(), selection_info)
    } else {
        format!(
            " Tools [{}]{} ({}↕) ",
            app.tools.len(),
            selection_info,
            app.sort_by.label()
        )
    }
}

/// Render the tool list
pub fn render_tool_list(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    if app.tab == Tab::Updates && !app.updates_checked {
        render_updates_empty_state(frame, app, theme, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tools
        .iter()
        .enumerate()
        .map(|(i, tool)| build_tool_list_item(app, tool, i, theme))
        .collect();

    let title_text = build_tool_list_title(app);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(title_text, Style::default().fg(theme.text))),
        )
        .highlight_style(
            Style::default()
                .bg(theme.surface0)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    let visible_height = area.height.saturating_sub(2) as usize;
    let offset = if visible_height > 0 {
        let offset = app.selected_index.saturating_sub(visible_height / 2);
        *state.offset_mut() = offset;
        app.list_offset = offset;
        offset
    } else {
        0
    };

    frame.render_stateful_widget(list, area, &mut state);

    if app.tools.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(app.tools.len()).position(offset);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Render the tool details panel
pub fn render_details(frame: &mut Frame, app: &mut App, db: &Database, theme: &Theme, area: Rect) {
    let tool = app.selected_tool().cloned();

    let content = if let Some(tool) = tool {
        let _ = app.get_github_info(&tool.name, db);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(theme.subtext0)),
                Span::styled(tool.name.clone(), Style::default().fg(theme.blue).bold()),
            ]),
            Line::from(""),
        ];

        if let Some(desc) = &tool.description {
            lines.push(Line::from(Span::styled(
                "Description:",
                Style::default().fg(theme.subtext0),
            )));
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(theme.text),
            )));
            lines.push(Line::from(""));
        }

        let src_icon = source_icon(&tool.source.to_string());
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{src_icon} {}", tool.source),
                Style::default().fg(theme.peach),
            ),
        ]));

        if let Some(cmd) = &tool.install_command {
            lines.push(Line::from(vec![
                Span::styled("Install: ", Style::default().fg(theme.subtext0)),
                Span::styled(cmd.clone(), Style::default().fg(theme.green)),
            ]));
        }

        if let Some(binary) = &tool.binary_name {
            lines.push(Line::from(vec![
                Span::styled("Binary: ", Style::default().fg(theme.subtext0)),
                Span::styled(binary.clone(), Style::default().fg(theme.text)),
            ]));
        }

        if let Some(category) = &tool.category {
            lines.push(Line::from(vec![
                Span::styled("Category: ", Style::default().fg(theme.subtext0)),
                Span::styled(category.clone(), Style::default().fg(theme.mauve)),
            ]));
        }

        if let Some(labels) = app.cache.labels_cache.get(&tool.name)
            && !labels.is_empty()
        {
            let mut spans = vec![Span::styled(
                "Labels: ",
                Style::default().fg(theme.subtext0),
            )];
            for (i, label) in labels.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw(" "));
                }
                let color = label_color(label, theme);
                spans.push(Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(theme.base).bg(color),
                ));
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));

        if let Some(usage) = app.cache.usage_data.get(&tool.name) {
            lines.push(Line::from(Span::styled(
                "Usage:",
                Style::default()
                    .fg(theme.subtext0)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("  Invocations: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    format!("{}", usage.use_count),
                    Style::default().fg(theme.teal),
                ),
            ]));
            if let Some(last) = &usage.last_used {
                lines.push(Line::from(vec![
                    Span::styled("  Last used: ", Style::default().fg(theme.subtext0)),
                    Span::styled(
                        format_friendly_datetime(last),
                        Style::default().fg(theme.text),
                    ),
                ]));
            }
            lines.push(Line::from(""));
        }

        if let Some(gh) = app.cache.github_cache.get(&tool.name) {
            lines.push(Line::from(Span::styled(
                "GitHub:",
                Style::default()
                    .fg(theme.subtext0)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("  ★ Stars: ", Style::default().fg(theme.yellow)),
                Span::styled(format_stars(gh.stars), Style::default().fg(theme.yellow)),
            ]));
            if let Some(lang) = &gh.language {
                lines.push(Line::from(vec![
                    Span::styled("  Language: ", Style::default().fg(theme.subtext0)),
                    Span::styled(lang.clone(), Style::default().fg(theme.peach)),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  Repo: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    format!("{}/{}", gh.repo_owner, gh.repo_name),
                    Style::default().fg(theme.blue),
                ),
            ]));
            lines.push(Line::from(""));
        }

        let (status_text, status_color, health_hint) = if !tool.is_installed {
            ("Not installed", theme.yellow, None)
        } else {
            let usage = app.cache.usage_data.get(&tool.name);
            let use_count = usage.map(|u| u.use_count).unwrap_or(0);
            let last_used = usage.and_then(|u| u.last_used.as_deref());

            let days_since = last_used
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days());

            match (use_count, days_since) {
                (0, _) => (
                    "Installed (never used)",
                    theme.red,
                    Some("Consider using or removing"),
                ),
                (_, Some(d)) if d < 7 => ("Installed (active)", theme.green, None),
                (_, Some(d)) if d < 30 => (
                    "Installed (idle)",
                    theme.yellow,
                    Some(&format!("Last used {} days ago", d) as &str).map(|_| "Not used recently"),
                ),
                (_, Some(_)) => ("Installed (stale)", theme.red, Some("Not used in 30+ days")),
                (_, None) => ("Installed", theme.green, None),
            }
        };
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(theme.subtext0)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]));
        if let Some(hint) = health_hint {
            lines.push(Line::from(Span::styled(
                format!("  ↳ {hint}"),
                Style::default().fg(theme.subtext0),
            )));
        }

        if tool.is_favorite {
            lines.push(Line::from(Span::styled(
                "★ Favorite",
                Style::default().fg(theme.yellow),
            )));
        }

        Text::from(lines)
    } else {
        Text::from(Span::styled(
            "No tool selected",
            Style::default().fg(theme.subtext0),
        ))
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(" Details ", Style::default().fg(theme.text))),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}
