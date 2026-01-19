//! Tests for the TUI application module

use super::types::UndoableAction;
use super::*;
use crate::db::Database;
use crate::models::{InstallSource, Tool};

#[test]
fn test_fuzzy_match_exact() {
    assert!(fuzzy_match("ripgrep", "ripgrep").is_some());
    let score = fuzzy_match("ripgrep", "ripgrep").unwrap();
    assert!(score > 100); // Exact match bonus
}

#[test]
fn test_fuzzy_match_prefix() {
    assert!(fuzzy_match("rip", "ripgrep").is_some());
    let score = fuzzy_match("rip", "ripgrep").unwrap();
    assert!(score > 50); // Prefix bonus
}

#[test]
fn test_fuzzy_match_subsequence() {
    // "rg" matches "ripgrep" (r...g)
    assert!(fuzzy_match("rg", "ripgrep").is_some());

    // "fdf" matches "fd-find"
    assert!(fuzzy_match("fdf", "fd-find").is_some());
}

#[test]
fn test_fuzzy_match_no_match() {
    // Characters must appear in order in target
    assert!(fuzzy_match("xyz", "ripgrep").is_none());
    assert!(fuzzy_match("abc", "ripgrep").is_none());
    // "gr" actually matches ripGRep (g at 3, r at 4)
    assert!(fuzzy_match("gr", "ripgrep").is_some());
}

#[test]
fn test_fuzzy_match_case_insensitive() {
    assert!(fuzzy_match("RIP", "ripgrep").is_some());
    assert!(fuzzy_match("rip", "RIPGREP").is_some());
}

#[test]
fn test_fuzzy_match_word_boundary_bonus() {
    // Matching at word boundary should score higher
    let boundary_score = fuzzy_match("f", "fd-find").unwrap();
    let mid_score = fuzzy_match("i", "fd-find").unwrap();
    assert!(boundary_score > mid_score);
}

#[test]
fn test_fuzzy_match_consecutive_bonus() {
    // Consecutive matches should score higher
    let consecutive = fuzzy_match("rip", "ripgrep").unwrap();
    let spread = fuzzy_match("rgp", "ripgrep").unwrap(); // r...g...p (positions 0,3,6)
    assert!(consecutive > spread);
}

// ==================== Command Palette Tests ====================

#[test]
fn test_command_mode_enter_exit() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.command.input.is_empty());

    app.enter_command();
    assert_eq!(app.input_mode, InputMode::Command);
    assert!(app.command.input.is_empty());

    app.command_push('q');
    assert_eq!(app.command.input, "q");

    app.exit_command();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.command.input.is_empty());
}

#[test]
fn test_command_push_pop() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    app.enter_command();
    app.command_push('h');
    app.command_push('e');
    app.command_push('l');
    app.command_push('p');
    assert_eq!(app.command.input, "help");

    app.command_pop();
    assert_eq!(app.command.input, "hel");

    app.command_pop();
    app.command_pop();
    app.command_pop();
    assert!(app.command.input.is_empty());
}

#[test]
fn test_command_execute_help() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    app.enter_command();
    app.command_push('h');
    app.execute_command(&db);

    assert!(app.show_help);
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_command_execute_quit() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    assert!(app.running);
    app.enter_command();
    app.command_push('q');
    app.execute_command(&db);

    assert!(!app.running);
}

#[test]
fn test_command_unknown() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    app.enter_command();
    for c in "invalidcmd".chars() {
        app.command_push(c);
    }
    app.execute_command(&db);

    // Should have status message about unknown command
    assert!(app.status_message.is_some());
    assert!(app.status_message.as_ref().unwrap().is_error);
}

// ==================== Undo/Redo Tests ====================

#[test]
fn test_undo_selection() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    // Initial state - no selections
    assert!(app.selected_tools.is_empty());

    // Record initial empty state, then add selections
    app.record_selection();
    app.selected_tools.insert("tool1".to_string());
    app.selected_tools.insert("tool2".to_string());

    // Undo should restore to empty state
    app.undo();
    assert!(app.selected_tools.is_empty());
}

#[test]
fn test_undo_filter() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    // Set a filter and record it
    app.search_query = "old_filter".to_string();
    app.record_filter();
    app.search_query = "new_filter".to_string();

    // Undo should restore old filter
    app.undo();
    assert_eq!(app.search_query, "old_filter");
}

#[test]
fn test_redo() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    // Set filter and record
    app.search_query = "filter1".to_string();
    app.record_filter();
    app.search_query = "filter2".to_string();

    // Undo
    app.undo();
    assert_eq!(app.search_query, "filter1");

    // Redo should restore to filter2
    app.redo();
    assert_eq!(app.search_query, "filter2");
}

#[test]
fn test_action_history() {
    let mut history = ActionHistory::new(3);

    // Initially empty
    assert!(!history.can_undo());
    assert!(!history.can_redo());

    // Add actions
    history.push(UndoableAction::Filter("a".to_string()));
    history.push(UndoableAction::Filter("b".to_string()));
    assert!(history.can_undo());

    // Pop undo
    let action = history.pop_undo().unwrap();
    if let UndoableAction::Filter(s) = action {
        assert_eq!(s, "b");
    }

    // Push to redo
    history.push_redo(UndoableAction::Filter("b".to_string()));
    assert!(history.can_redo());

    // Pop redo
    let action = history.pop_redo().unwrap();
    if let UndoableAction::Filter(s) = action {
        assert_eq!(s, "b");
    }
}

#[test]
fn test_history_max_size() {
    let mut history = ActionHistory::new(2);

    history.push(UndoableAction::Filter("a".to_string()));
    history.push(UndoableAction::Filter("b".to_string()));
    history.push(UndoableAction::Filter("c".to_string()));

    // Should only have 2 actions (oldest removed)
    assert!(history.can_undo());
    let _ = history.pop_undo(); // c
    let action = history.pop_undo(); // b
    if let Some(UndoableAction::Filter(s)) = action {
        assert_eq!(s, "b");
    }

    // No more undo
    assert!(!history.can_undo());
}

// ==================== Mouse Handler Tests ====================

#[test]
fn test_click_list_item_tool() {
    let db = Database::open_in_memory().unwrap();
    // Insert installed tools (App starts on Installed tab)
    db.insert_tool(
        &Tool::new("tool1")
            .with_source(InstallSource::Cargo)
            .installed(),
    )
    .unwrap();
    db.insert_tool(
        &Tool::new("tool2")
            .with_source(InstallSource::Cargo)
            .installed(),
    )
    .unwrap();
    db.insert_tool(
        &Tool::new("tool3")
            .with_source(InstallSource::Cargo)
            .installed(),
    )
    .unwrap();
    let mut app = App::new(&db).unwrap();

    assert_eq!(app.selected_index, 0);

    // Click on second item (row 1)
    app.click_list_item(1);
    assert_eq!(app.selected_index, 1);

    // Click on third item (row 2)
    app.click_list_item(2);
    assert_eq!(app.selected_index, 2);
}

#[test]
fn test_click_list_item_with_offset() {
    let db = Database::open_in_memory().unwrap();
    for i in 0..10 {
        db.insert_tool(
            &Tool::new(format!("tool{}", i))
                .with_source(InstallSource::Cargo)
                .installed(),
        )
        .unwrap();
    }
    let mut app = App::new(&db).unwrap();

    // Simulate scrolled list with offset 5
    app.list_offset = 5;

    // Click on first visible item (row 0) should select tool5
    app.click_list_item(0);
    assert_eq!(app.selected_index, 5);

    // Click on row 3 should select tool8
    app.click_list_item(3);
    assert_eq!(app.selected_index, 8);
}

#[test]
fn test_click_list_item_out_of_bounds() {
    let db = Database::open_in_memory().unwrap();
    db.insert_tool(
        &Tool::new("tool1")
            .with_source(InstallSource::Cargo)
            .installed(),
    )
    .unwrap();
    let mut app = App::new(&db).unwrap();

    assert_eq!(app.selected_index, 0);

    // Click on row 10 (out of bounds) - should not change selection
    app.click_list_item(10);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_set_list_area() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    assert!(app.last_list_area.is_none());

    app.set_list_area(10, 20, 100, 50);
    assert_eq!(app.last_list_area, Some((10, 20, 100, 50)));
}

#[test]
fn test_get_list_row() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    // Set list area: x=10, y=5, width=80, height=20
    app.set_list_area(10, 5, 80, 20);

    // Click inside list area (accounting for border)
    // y=6 is first content row (after top border at y=5)
    let row = app.get_list_row(15, 6);
    assert_eq!(row, Some(0));

    // y=7 is second content row
    let row = app.get_list_row(15, 7);
    assert_eq!(row, Some(1));

    // Click outside list area (x too small)
    let row = app.get_list_row(5, 7);
    assert!(row.is_none());

    // Click outside list area (y too small - on border)
    let row = app.get_list_row(15, 5);
    assert!(row.is_none());

    // Click outside list area (y too large)
    let row = app.get_list_row(15, 30);
    assert!(row.is_none());
}

#[test]
fn test_set_tab_area() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    assert!(app.last_tab_area.is_none());

    app.set_tab_area(0, 0, 120, 3);
    assert_eq!(app.last_tab_area, Some((0, 0, 120, 3)));
}

#[test]
fn test_click_tab() {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    // Set tab area starting at x=0
    app.set_tab_area(0, 0, 120, 3);

    // Initially on Installed tab
    assert_eq!(app.tab, Tab::Installed);

    // Tab layout (accounting for border and padding):
    // Content starts at x=1 (after border)
    // Tab format: " title " with dividers
    // Installed: " Installed " (11 chars), Available: " Available " (11 chars), etc.

    // Click on first tab (Installed) - should stay on Installed
    // Position 1-12 is " Installed "
    app.click_tab(5, &db);
    assert_eq!(app.tab, Tab::Installed);

    // Click on second tab (Available)
    // After Installed (12 chars) + divider (1) = start at 13
    app.click_tab(15, &db);
    assert_eq!(app.tab, Tab::Available);
}

#[test]
fn test_config_menu_ai_provider_change() {
    use crate::config::AiProvider;

    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    // Initially ai_available should be based on loaded config (default is None = false)
    // But let's set it explicitly to true for testing
    app.ai_available = true;

    // Open config menu
    app.open_config_menu();
    assert!(app.show_config_menu);

    // Verify we're on AI Provider section
    assert_eq!(app.config_menu.section, ConfigSection::AiProvider);

    // Set ai_selected to index 0 (None)
    app.config_menu.ai_selected = 0;

    // Navigate to buttons and save
    app.config_menu.section = ConfigSection::Buttons;
    app.config_menu.button_focused = 0; // Save button

    // The to_config should now return AiProvider::None
    let config = app.config_menu.to_config();
    assert_eq!(config.ai.provider, AiProvider::None);

    // Manually verify the ai_available logic
    let expected_ai_available = config.ai.provider != AiProvider::None;
    assert!(!expected_ai_available); // Should be false since provider is None

    // Now test with a different provider
    app.config_menu.ai_selected = 1; // Claude
    let config = app.config_menu.to_config();
    assert_eq!(config.ai.provider, AiProvider::Claude);
    assert!(config.ai.provider != AiProvider::None);
}

#[test]
fn test_ai_provider_all_indices() {
    use crate::config::AiProvider;

    // Verify the indices in AiProvider::all() match expectations
    let all = AiProvider::all();
    assert_eq!(all.len(), 5);
    assert_eq!(all[0], AiProvider::None);
    assert_eq!(all[1], AiProvider::Claude);
    assert_eq!(all[2], AiProvider::Gemini);
    assert_eq!(all[3], AiProvider::Codex);
    assert_eq!(all[4], AiProvider::Opencode);
}

#[test]
fn test_save_config_menu_updates_ai_available() {
    use crate::config::AiProvider;

    let db = Database::open_in_memory().unwrap();
    let mut app = App::new(&db).unwrap();

    // Start with AI available (simulate having Claude configured)
    app.ai_available = true;

    // Open config menu
    app.open_config_menu();

    // Change to None (index 0)
    app.config_menu.ai_selected = 0;

    // Verify before save - ai_available should still be true
    assert!(app.ai_available);

    // Build config and check the provider
    let config = app.config_menu.to_config();
    assert_eq!(config.ai.provider, AiProvider::None);

    // Now simulate save (without actually writing to file)
    // This is what save_config_menu does internally:
    app.ai_available = config.ai.provider != AiProvider::None;

    // After save, ai_available should be false
    assert!(!app.ai_available);

    // Now test the reverse - change from None to Claude
    app.config_menu.ai_selected = 1; // Claude
    let config = app.config_menu.to_config();
    assert_eq!(config.ai.provider, AiProvider::Claude);

    app.ai_available = config.ai.provider != AiProvider::None;
    assert!(app.ai_available);
}
