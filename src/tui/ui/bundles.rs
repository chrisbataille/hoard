//! Bundle list and details rendering
//!
//! This module handles rendering of the bundle list and bundle details panels.

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

use super::super::app::App;
use super::super::theme::Theme;
use crate::db::Database;

/// Render the bundle list
pub fn render_bundle_list(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.bundles.is_empty() {
        let message =
            "No bundles yet. Create one with: hoards bundle create <name> --tools tool1,tool2";
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(theme.subtext0))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.surface1))
                    .title(Span::styled(" Bundles ", Style::default().fg(theme.text))),
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
                Span::styled(&bundle.name, Style::default().fg(theme.text).bold()),
                Span::styled(
                    format!(" ({})", count_str),
                    Style::default().fg(theme.subtext0),
                ),
            ]);

            let style = if i == app.bundles.selected {
                Style::default().bg(theme.surface0)
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
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(title, Style::default().fg(theme.text))),
        )
        .highlight_style(
            Style::default()
                .bg(theme.surface0)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.bundles.selected));

    frame.render_stateful_widget(list, area, &mut state);

    let visible_height = area.height.saturating_sub(2) as usize;
    if app.bundles.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state =
            ScrollbarState::new(app.bundles.len()).position(app.bundles.selected);

        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

/// Render the bundle details panel
pub fn render_bundle_details(
    frame: &mut Frame,
    app: &App,
    db: &Database,
    theme: &Theme,
    area: Rect,
) {
    let content = if let Some(bundle) = app.bundles.get(app.bundles.selected) {
        let mut lines = vec![
            Line::from(Span::styled(
                &bundle.name,
                Style::default()
                    .fg(theme.blue)
                    .bold()
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(""),
        ];

        if let Some(desc) = &bundle.description {
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(theme.text),
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(vec![
            Span::styled("Tools: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{}", bundle.tools.len()),
                Style::default().fg(theme.teal),
            ),
        ]));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "─── Contents ───",
            Style::default().fg(theme.surface1),
        )));

        for tool_name in &bundle.tools {
            let is_installed = db
                .get_tool_by_name(tool_name)
                .ok()
                .flatten()
                .map(|t| t.is_installed)
                .unwrap_or(false);

            let (status, status_color) = if is_installed {
                ("●", theme.green)
            } else {
                ("○", theme.subtext0)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", status), Style::default().fg(status_color)),
                Span::styled(tool_name.clone(), Style::default().fg(theme.text)),
            ]));
        }

        lines.push(Line::from(""));

        let mut untracked = 0;
        let mut not_installed = 0;

        for name in &bundle.tools {
            match db.get_tool_by_name(name).ok().flatten() {
                None => untracked += 1,
                Some(t) if !t.is_installed => not_installed += 1,
                _ => {}
            }
        }

        let missing = untracked + not_installed;

        if missing > 0 {
            lines.push(Line::from(Span::styled(
                format!("Press 'i' to install {} missing tool(s)", missing),
                Style::default().fg(theme.green),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "All tools installed ✓",
                Style::default().fg(theme.green),
            )));
        }

        if untracked > 0 {
            lines.push(Line::from(Span::styled(
                format!(
                    "Press 'a' to add {} untracked tool(s) to Available",
                    untracked
                ),
                Style::default().fg(theme.blue),
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
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(
                    " Bundle Details ",
                    Style::default().fg(theme.text),
                )),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}
