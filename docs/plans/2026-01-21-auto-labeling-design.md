# Auto-Labeling Feature Design

## Summary

Automatically label tools based on metadata (source, category, GitHub topics) with AI fallback for tools lacking sufficient labels.

## Requirements

- **Triggers**: Auto-label on sync/scan AND via manual command
- **Sources**: Metadata first, AI fallback when < 2 labels
- **Threshold**: Use AI only if tool has fewer than 2 metadata-derived labels

---

## Label Sources & Rules

### Metadata-based labels (automatic, no API)

| Source | Labels Generated |
|--------|------------------|
| Install source | `cargo`, `pip`, `npm`, `apt`, `brew`, `flatpak`, `go` |
| Category | Whatever category is set (e.g., `search`, `git`, `files`) |
| GitHub topics | Synced from repo (e.g., `rust`, `cli`, `terminal`) |
| Language detection | Infer from source: cargo→`rust`, pip→`python`, npm→`javascript` |

### AI fallback (when metadata yields < 2 labels)

- Send tool name + description to AI provider
- AI returns 2-5 suggested labels
- Labels are lowercase, hyphenated (e.g., `file-management`, `text-processing`)

### Label normalization

- All labels stored lowercase
- Spaces converted to hyphens
- Duplicates prevented at DB level

---

## Commands

### Automatic trigger (during sync)

```bash
hoard sync --scan      # Auto-labels new tools discovered
hoard sync --labels    # Re-labels all tools missing labels
hoard sync --all       # Includes --labels
```

### Manual command

```bash
hoard label auto                 # Auto-label all tools needing labels
hoard label auto ripgrep         # Auto-label specific tool
hoard label auto --force         # Re-label even if already labeled
hoard label auto --ai            # Force AI for all (skip metadata-only)
hoard label auto --dry-run       # Preview what would be labeled
```

### Label management commands

```bash
hoard label add ripgrep rust cli     # Manual label add
hoard label remove ripgrep cli       # Remove specific label
hoard label list                     # Show all labels with counts
hoard label list ripgrep             # Show labels for a tool
hoard label clear ripgrep            # Remove all labels from tool
```

---

## Implementation

### Files to create

- `src/commands/label.rs` - CLI command handlers

### Files to modify

- `src/cli.rs` - Add `LabelCommands` enum
- `src/main.rs` - Wire up label commands
- `src/commands/sync.rs` - Add `--labels` flag, call auto-label on scan
- `src/ai/mod.rs` - Add `generate_labels()` function

### Auto-label flow

```
1. Collect metadata labels (source, category, language)
2. Fetch GitHub topics if synced
3. Count total labels
4. If < 2 and AI configured → call AI for suggestions
5. Deduplicate and store
```

### AI prompt

```
Tool: {name}
Description: {description}
Suggest 3-5 lowercase labels for categorizing this CLI tool.
Return only labels, comma-separated.
```

---

## Testing

### Unit tests

- `test_auto_label_from_source` - cargo → [cargo, rust]
- `test_auto_label_from_category` - tool with category gets it as label
- `test_auto_label_threshold` - AI only called when < 2 labels
- `test_label_normalization` - "File Manager" → "file-manager"

### Integration tests

- `test_label_add_remove_clear` - CLI label management
- `test_sync_auto_labels` - sync --scan applies labels

### TUI tests

- `test_label_filter_narrows_list` - filtering by label shows only matching tools
- `test_label_edit_popup` - can add/remove labels in edit popup

### Manual verification

1. `hoard label auto --dry-run` shows expected labels
2. `hoard sync --scan` labels newly discovered tools
3. `hoard label list` shows all labels with counts
4. AI fallback works when metadata insufficient
5. TUI: `l` opens label filter, selecting filters the list
6. TUI: `L` opens label editor, can add/remove labels
7. TUI: `:label auto` auto-labels selected tool(s)

### Edge cases

- Tool with no description → use name only for AI
- AI unavailable → skip gracefully, log warning
- Duplicate labels from multiple sources → deduplicated

---

## TUI Integration

### Display (existing, enhance)

- Labels already show in tool list as colored tags
- Details popup shows full label list

### Filter by label

- New keybinding: `l` to open label filter popup
- Popup shows all labels with counts
- Select one or more labels to filter
- Filter indicator in footer: `[label: rust, cli]`
- Press `l` again or `x` to clear filter

### Edit labels

- New keybinding: `L` (shift+l) to edit labels on selected tool
- Popup with:
  - Current labels (toggle with space to remove)
  - Text input to add new label
  - Quick suggestions based on tool metadata
- Changes saved immediately

### Auto-label in TUI

- Add to command palette: `:label auto` to auto-label selected tool
- Batch: select multiple tools with space, then `:label auto`

### Files to modify (TUI)

- `src/tui/app/types.rs` - Add label filter state, label edit popup state
- `src/tui/app/actions.rs` - Handle `l` and `L` keybindings
- `src/tui/ui/dialogs.rs` - Add label filter popup, label edit popup
- `src/tui/ui/footer.rs` - Show active label filter
- `src/tui/app/command_exec.rs` - Add `:label auto` command

---

## Out of Scope

- Label hierarchy/nesting
- Label suggestions during manual add
