//! Configuration menu rendering
//!
//! This module handles rendering of the configuration menu popup.

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use super::super::app::{App, ConfigMenuState, ConfigSection};
use super::super::theme::Theme;
use super::dialogs::centered_rect;
use crate::config::{AiProvider, SourcesConfig, TuiTheme};
use crate::sources::PackageManagerStatus;

/// Create a radio button line for config menu
fn make_radio_line<'a>(selected: bool, focused: bool, label: String, theme: &Theme) -> Line<'a> {
    let bullet = if selected { "●" } else { "○" };
    let style = if focused {
        Style::default().fg(theme.blue).bold()
    } else if selected {
        Style::default().fg(theme.green)
    } else {
        Style::default().fg(theme.subtext0)
    };
    Line::from(vec![
        Span::styled(format!("  {} ", bullet), style),
        Span::styled(label, style),
    ])
}

/// Create a checkbox line for config menu
/// Create a checkbox line for package managers with version info
/// If unavailable, shows greyed out with "not installed"
fn make_source_checkbox_line<'a>(
    checked: bool,
    focused: bool,
    label: &str,
    available: bool,
    version: Option<&str>,
    theme: &Theme,
) -> Line<'a> {
    if !available {
        // Unavailable: greyed out with "not installed"
        let style = Style::default().fg(theme.surface1);
        return Line::from(vec![
            Span::styled("  ☐ ", style),
            Span::styled(format!("{} ", label), style),
            Span::styled("(not installed)", Style::default().fg(theme.surface0)),
        ]);
    }

    let mark = if checked { "☑" } else { "☐" };
    let style = if focused {
        Style::default().fg(theme.blue).bold()
    } else if checked {
        Style::default().fg(theme.green)
    } else {
        Style::default().fg(theme.subtext0)
    };

    let mut spans = vec![
        Span::styled(format!("  {} ", mark), style),
        Span::styled(label.to_string(), style),
    ];

    // Add version info if available
    if let Some(ver) = version {
        spans.push(Span::styled(
            format!(" v{}", ver),
            Style::default().fg(theme.surface1),
        ));
    }

    Line::from(spans)
}

/// Create a section header line
fn make_section_header<'a>(title: &'static str, focused: bool, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(
        title,
        if focused {
            Style::default().fg(theme.blue).bold()
        } else {
            Style::default().fg(theme.text).bold()
        },
    ))
}

/// Create a dimmed section header for disabled sections
fn make_section_header_dimmed<'a>(title: &'static str, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(
        title,
        Style::default().fg(theme.subtext0).italic(),
    ))
}

/// Render AI Provider section lines
fn render_config_ai_section(state: &ConfigMenuState, theme: &Theme) -> Vec<Line<'static>> {
    let ai_focused = state.section == ConfigSection::AiProvider;
    let mut lines = vec![make_section_header("AI Provider", ai_focused, theme)];

    for (i, provider) in AiProvider::all().iter().enumerate() {
        let label = match provider {
            AiProvider::None => "None (disabled)",
            AiProvider::Claude => "Claude",
            AiProvider::Gemini => "Gemini",
            AiProvider::Codex => "Codex",
            AiProvider::Opencode => "Opencode",
        };
        let selected = i == state.ai_selected;
        let focused = ai_focused && selected;
        lines.push(make_radio_line(selected, focused, label.to_string(), theme));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Claude Model section lines (only shown when Claude is selected)
fn render_config_claude_model_section(
    state: &ConfigMenuState,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let claude_focused = state.section == ConfigSection::ClaudeModel;

    // Check if Claude is selected as the AI provider
    let claude_provider_index = AiProvider::all()
        .iter()
        .position(|p| *p == AiProvider::Claude)
        .unwrap_or(1);
    let is_claude_selected = state.ai_selected == claude_provider_index;

    // Only show full section if Claude is selected as provider
    if !is_claude_selected {
        return vec![
            make_section_header_dimmed("Claude Model (select Claude above)", theme),
            Line::from(""),
        ];
    }

    let mut lines = vec![make_section_header("Claude Model", claude_focused, theme)];

    let models = [
        ("Haiku", "Fast and cost-effective"),
        ("Sonnet", "Balanced intelligence"),
        ("Opus", "Most capable"),
    ];

    for (i, (name, desc)) in models.iter().enumerate() {
        let selected = i == state.claude_model_selected;
        let focused = claude_focused && selected;
        let label = format!("{} - {}", name, desc);
        lines.push(make_radio_line(selected, focused, label, theme));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Theme section lines
fn render_config_theme_section(state: &ConfigMenuState, theme: &Theme) -> Vec<Line<'static>> {
    let theme_focused = state.section == ConfigSection::Theme;
    let mut lines = vec![make_section_header("Theme", theme_focused, theme)];

    let builtin_themes = [
        TuiTheme::CatppuccinMocha,
        TuiTheme::CatppuccinLatte,
        TuiTheme::Dracula,
        TuiTheme::Nord,
        TuiTheme::TokyoNight,
        TuiTheme::Gruvbox,
    ];

    for (i, t) in builtin_themes.iter().enumerate() {
        let selected = i == state.theme_selected;
        let focused = theme_focused && selected;
        lines.push(make_radio_line(selected, focused, t.to_string(), theme));
    }

    // Custom theme option
    let custom_exists = super::super::theme::CustomTheme::exists();
    let custom_selected = state.theme_selected == 6;
    let custom_focused = theme_focused && custom_selected;
    let custom_label = if custom_exists {
        "Custom".to_string()
    } else {
        "Custom (use :create-theme to create)".to_string()
    };
    lines.push(make_radio_line(
        custom_selected,
        custom_focused,
        custom_label,
        theme,
    ));

    // Show file path hint when Custom is selected
    if custom_selected && let Ok(path) = super::super::theme::CustomTheme::file_path() {
        lines.push(Line::from(Span::styled(
            format!("    Edit: {}", path.display()),
            Style::default().fg(theme.subtext0).italic(),
        )));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Package Managers section lines
fn render_config_sources_section(
    state: &ConfigMenuState,
    theme: &Theme,
    package_managers: &PackageManagerStatus,
) -> Vec<Line<'static>> {
    let sources_focused = state.section == ConfigSection::Sources;
    let mut lines = vec![make_section_header(
        "Package Managers",
        sources_focused,
        theme,
    )];

    let source_names = SourcesConfig::all_sources();
    let source_labels = [
        "Cargo", "Apt", "Pip", "npm", "Brew", "Go", "Flatpak", "Manual",
    ];
    for (i, (&name, label)) in source_names.iter().zip(source_labels.iter()).enumerate() {
        let checked = state.sources.is_enabled(name);
        let focused = sources_focused && i == state.source_focused;
        let available = package_managers.is_available(name);
        let version = package_managers.version(name);

        lines.push(make_source_checkbox_line(
            checked, focused, label, available, version, theme,
        ));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Usage Tracking section lines
fn render_config_usage_section(state: &ConfigMenuState, theme: &Theme) -> Vec<Line<'static>> {
    let usage_focused = state.section == ConfigSection::UsageMode;
    let mut lines = vec![make_section_header("Usage Tracking", usage_focused, theme)];

    lines.push(make_radio_line(
        state.usage_selected == 0,
        usage_focused && state.usage_selected == 0,
        "Scan (manual)".to_string(),
        theme,
    ));
    lines.push(make_radio_line(
        state.usage_selected == 1,
        usage_focused && state.usage_selected == 1,
        "Hook (real-time)".to_string(),
        theme,
    ));

    lines.push(Line::from(""));
    lines
}

/// Render Buttons section line
fn render_config_buttons_section(state: &ConfigMenuState, theme: &Theme) -> Line<'static> {
    let buttons_focused = state.section == ConfigSection::Buttons;
    let save_style = if buttons_focused && state.button_focused == 0 {
        Style::default().fg(theme.base).bg(theme.green).bold()
    } else {
        Style::default().fg(theme.green)
    };
    let cancel_style = if buttons_focused && state.button_focused == 1 {
        Style::default().fg(theme.base).bg(theme.red).bold()
    } else {
        Style::default().fg(theme.red)
    };

    Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(" Save ", save_style),
        Span::styled("  ", Style::default()),
        Span::styled(" Cancel ", cancel_style),
    ])
}

/// Render the config menu popup
pub fn render_config_menu(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(60, 85, area);
    app.last_config_popup_area = Some((
        popup_area.x,
        popup_area.y,
        popup_area.width,
        popup_area.height,
    ));

    let state = &app.config_menu;

    // Build content lines from section helpers
    let mut lines = Vec::new();
    lines.extend(render_config_ai_section(state, theme));
    lines.extend(render_config_claude_model_section(state, theme));
    lines.extend(render_config_theme_section(state, theme));
    lines.extend(render_config_sources_section(
        state,
        theme,
        &app.package_managers,
    ));
    lines.extend(render_config_usage_section(state, theme));
    lines.push(render_config_buttons_section(state, theme));

    let total_lines = lines.len();
    let content_height = popup_area.height.saturating_sub(3) as usize;
    let scroll_offset = state
        .scroll_offset
        .min(total_lines.saturating_sub(content_height));

    let config_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.mauve))
                .title(Span::styled(
                    " Configuration ",
                    Style::default().fg(theme.mauve).bold(),
                ))
                .title_bottom(Line::from(vec![
                    Span::styled(" s", Style::default().fg(theme.green).bold()),
                    Span::styled(" Save ", Style::default().fg(theme.subtext0)),
                    Span::styled("Esc", Style::default().fg(theme.red).bold()),
                    Span::styled(" Cancel ", Style::default().fg(theme.subtext0)),
                    Span::styled("↑↓", Style::default().fg(theme.blue).bold()),
                    Span::styled(" Nav ", Style::default().fg(theme.subtext0)),
                    Span::styled("Tab", Style::default().fg(theme.blue).bold()),
                    Span::styled(" Section ", Style::default().fg(theme.subtext0)),
                ]))
                .style(Style::default().bg(theme.base)),
        )
        .scroll((scroll_offset as u16, 0))
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(config_widget, popup_area);

    // Render scrollbar if needed
    let max_scroll = total_lines.saturating_sub(content_height);
    if max_scroll > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_offset);
        let scrollbar_area = Rect {
            x: popup_area.x + popup_area.width - 2,
            y: popup_area.y + 1,
            width: 1,
            height: popup_area.height.saturating_sub(2),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}
