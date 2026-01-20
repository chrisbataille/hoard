# Hoard v2.0 Implementation Plan

*Created: January 2026*
*Based on: Strategic research and competitive analysis*

---

## Vision

Transform hoard from a CLI tool tracker into the **AI-powered developer tool management platform** - combining multi-source tracking, usage analytics, intelligent discovery, and a modern TUI.

**Tagline:** *"Know what you use. Discover what you need."*

---

## Phase Overview

| Phase | Focus | Duration | Status |
|-------|-------|----------|--------|
| 1 | CLI Simplification | 2-3 weeks | ✅ Complete |
| 2 | AI Enhancements | 2-3 weeks | ✅ Complete |
| 3 | TUI MVP | 4-6 weeks | ✅ Complete |
| 4 | TUI Polish | 2-3 weeks | ✅ Complete |
| 5 | TUI Discover Tab | 3-4 weeks | ✅ Complete |

---

## Phase 1: CLI Simplification

**Goal:** Reduce cognitive load from 27 commands to ~15 organized commands + 3 workflows.

### 1.1 Create Unified `sync` Command

**Current state:**
```bash
hoard scan              # Discover tools
hoard sync              # Update status
hoard fetch-descriptions # Get descriptions
hoard gh sync           # GitHub data
hoard usage scan        # Usage tracking
```

**Target state:**
```bash
hoard sync                    # Smart sync (status only)
hoard sync --scan             # Include discovery
hoard sync --github           # Include GitHub data
hoard sync --usage            # Include usage tracking
hoard sync --all              # Everything
hoard sync --dry-run          # Preview changes
```

**Tasks:**
- [x] Add `--scan` flag to sync command
- [x] Add `--github` flag to sync command
- [x] Add `--usage` flag to sync command
- [x] Add `--all` flag combining all operations
- [x] Deprecate standalone `scan` (keep as alias for 1 version)
- [x] Deprecate `fetch-descriptions` (merge into sync)
- [x] Update help text and documentation

---

### 1.2 Create `discover` Command Group

**Current state:**
```bash
hoard list              # List tools
hoard search <query>    # Search
hoard categories        # Show categories
hoard labels            # Show labels
hoard suggest           # Missing tools
hoard gh search         # GitHub search
```

**Target state:**
```bash
hoard discover                      # Interactive discovery menu
hoard discover list [filters]       # List tools (absorbs `list`)
hoard discover search <query>       # Local + GitHub search
hoard discover categories           # Browse by category
hoard discover labels               # Browse by label
hoard discover similar <tool>       # NEW: Find related tools
hoard discover trending             # NEW: Popular tools (GitHub stars)
hoard discover recommended          # Absorbs `recommend`
```

**Tasks:**
- [x] Create `discover` command group in cli.rs
- [x] Move `list` to `discover list` (keep alias)
- [x] Move `search` to `discover search` (keep alias)
- [x] Move `categories` to `discover categories`
- [x] Move `labels` to `discover labels`
- [x] Move `suggest` to `discover missing`
- [x] Move `recommend` to `discover recommended`
- [x] Implement `discover similar <tool>` (same category + labels)
- [x] Implement `discover trending` (top GitHub stars)
- [x] Merge `gh search` into `discover search --github`

---

### 1.3 Create `insights` Command Group

**Current state:**
```bash
hoard usage show        # Usage stats
hoard usage tool <name> # Tool usage
hoard unused            # Unused tools
hoard stats             # Database stats
hoard doctor            # Health check
hoard info              # Database info
```

**Target state:**
```bash
hoard insights                  # Overview dashboard
hoard insights usage [tool]     # Usage statistics
hoard insights unused           # Unused tools
hoard insights health           # Absorbs `doctor`
hoard insights stats            # Absorbs `stats` + `info`
```

**Tasks:**
- [x] Create `insights` command group in cli.rs
- [x] Implement `insights` overview (combined stats)
- [x] Move `usage show` to `insights usage`
- [x] Move `usage tool` to `insights usage <tool>`
- [x] Move `unused` to `insights unused`
- [x] Merge `doctor` + `stats` + `info` into `insights health`
- [x] Deprecate standalone commands (keep aliases for 1 version)

---

### 1.4 Reorganize AI Commands

**Current state:**
```bash
hoard ai set <provider>     # Config
hoard ai show               # Config
hoard ai test               # Config
hoard ai categorize         # Operation
hoard ai describe           # Operation
hoard ai suggest-bundle     # Operation
```

**Target state:**
```bash
# Configuration
hoard ai config set <provider>
hoard ai config show
hoard ai config test

# Operations (renamed to "enrich")
hoard ai enrich                     # Interactive menu
hoard ai enrich --categorize        # Categorize tools
hoard ai enrich --describe          # Generate descriptions
hoard ai enrich --all               # Both operations
hoard ai enrich --dry-run           # Preview changes
```

**Tasks:**
- [x] Create `ai config` subcommand group
- [x] Move `ai set/show/test` to `ai config set/show/test`
- [x] Create `ai enrich` with flags
- [x] Deprecate `ai categorize` and `ai describe` (suggest new commands)
- [x] Move `ai suggest-bundle` to Phase 2 (AI enhancements)

---

### 1.5 Add Workflow Commands

**New commands for common multi-step operations:**

```bash
hoard init
# First-time setup wizard:
# 1. Scan system for tools
# 2. Sync installation status
# 3. Fetch descriptions
# 4. Optionally: GitHub sync, AI categorization
# Interactive prompts guide the user

hoard maintain
# Daily/weekly maintenance:
# 1. Sync status
# 2. Check for updates
# 3. Scan usage
# 4. Show health issues
# Can be run with --auto for non-interactive

hoard cleanup
# Cleanup wizard:
# 1. Show unused tools
# 2. Show orphaned entries
# 3. Fix health issues
# 4. Optionally: Remove unused tools
# Interactive confirmation for destructive actions
```

**Tasks:**
- [x] Implement `init` command with interactive wizard
- [x] Implement `maintain` command with `--auto` flag
- [x] Implement `cleanup` command with confirmations
- [x] Add progress indicators for multi-step operations
- [x] Update Fish completions for new commands

---

### 1.6 Simplify GitHub Integration

**Current state (6 commands):**
```bash
hoard gh sync
hoard gh fetch <tool>
hoard gh search <query>
hoard gh info <tool>
hoard gh rate-limit
hoard gh backfill
```

**Target state (integrated into other commands):**
```bash
hoard sync --github              # Absorbs gh sync
hoard show <tool>                # Shows GitHub info inline
hoard discover search --github   # Absorbs gh search
hoard insights health            # Shows rate limit status

# Keep only for power users:
hoard gh fetch <tool>            # Force fetch single tool
hoard gh backfill                # Fill from cache
```

**Tasks:**
- [x] Add GitHub info to `show` command output
- [x] Add `--github` flag to `sync`
- [x] Add `--github` flag to `discover search`
- [x] Add rate limit to `insights health`
- [x] Deprecate `gh sync`, `gh search`, `gh info`, `gh rate-limit`
- [x] Keep `gh fetch` and `gh backfill` for advanced use

---

### 1.7 Update Documentation & Completions

**Tasks:**
- [x] Update USER_GUIDE.md with new command structure
- [x] Update API.md with new exports
- [x] Update README.md quick start
- [x] Rewrite Fish completions for new structure
- [x] Add deprecation warnings for old commands
- [x] Create migration guide for existing users

---

## Phase 2: AI Enhancements

**Goal:** Add AI-powered features that differentiate hoard from competitors.

### 2.1 GitHub README Extraction

```bash
hoard ai extract <github-url>
hoard ai extract https://github.com/BurntSushi/ripgrep

# Output:
# Extracted from README:
#   Name: ripgrep
#   Binary: rg
#   Source: cargo
#   Install: cargo install ripgrep
#   Description: ripgrep recursively searches directories...
#   Category: search (detected)
#
# Add to database? [Y/n]
```

**Implementation:**
1. Fetch README.md via GitHub API
2. Send to Claude with extraction prompt
3. Parse structured response
4. Validate extracted data
5. Optionally add to database

**Tasks:**
- [x] Create extraction prompt template
- [x] Implement GitHub README fetching
- [x] Implement AI extraction with Claude
- [x] Parse and validate response
- [x] Add interactive confirmation
- [x] Handle edge cases (no README, multiple install methods)
- [x] Cache extractions to avoid repeat API calls

---

### 2.2 Smart Bundle Suggestions

```bash
hoard ai suggest-bundles

# Output:
# Based on your usage patterns:
#
# 📦 "Modern Unix" Bundle
#    You use ripgrep (847x) and fd (423x) heavily.
#    Suggested additions:
#    • eza - modern ls replacement (12K ★)
#    • zoxide - smarter cd (22K ★)
#    • dust - intuitive du (8K ★)
#    [c]reate  [i]nstall all  [s]kip
#
# 📦 "Git Power Tools" Bundle
#    You use git (2341x) and delta (156x).
#    Suggested additions:
#    • lazygit - TUI for git (45K ★)
#    • gh - GitHub CLI (47K ★)
#    [c]reate  [i]nstall all  [s]kip
```

**Implementation:**
1. Analyze installed tools and usage patterns
2. Send context to Claude
3. Get bundle suggestions with reasoning
4. Present interactive menu
5. Create bundle and/or install tools

**Tasks:**
- [x] Create bundle suggestion prompt template
- [x] Gather context (installed tools, usage, categories)
- [x] Implement AI suggestion call
- [x] Parse bundle suggestions
- [x] Implement interactive selection UI
- [x] Connect to bundle create/install commands

---

### 2.3 Contextual Tool Discovery

```bash
hoard ai discover "I'm setting up a Kubernetes development environment"

# Output:
# For Kubernetes development, I recommend:
#
# Essential:
#   kubectl     - Kubernetes CLI (installed ✓)
#   k9s         - TUI for Kubernetes (25K ★)
#   helm        - Package manager for K8s (27K ★)
#
# Productivity:
#   kubectx     - Switch contexts easily (18K ★)
#   stern       - Multi-pod log tailing (7K ★)
#   k3d         - Local K8s clusters (5K ★)
#
# [i]nstall selected  [b]undle all  [s]how details
```

**Tasks:**
- [x] Create discovery prompt template
- [x] Implement natural language query handling
- [x] Query GitHub for tool popularity
- [x] Present categorized suggestions
- [x] Allow batch installation

---

### 2.4 Tool Cheatsheet Generation

```bash
hoard ai cheatsheet ripgrep

# Output:
# ┌─────────────────────────────────────────┐
# │ ripgrep (rg) - Fast grep replacement    │
# ├─────────────────────────────────────────┤
# │ BASIC USAGE                             │
# │   rg pattern              Search files  │
# │   rg -i pattern           Ignore case   │
# │   rg -w pattern           Whole words   │
# │                                         │
# │ FILE FILTERING                          │
# │   rg -t rust pattern      Rust files    │
# │   rg -g '*.md' pattern    Glob pattern  │
# │   rg --hidden pattern     Hidden files  │
# │                                         │
# │ OUTPUT                                  │
# │   rg -c pattern           Count matches │
# │   rg -l pattern           Files only    │
# │   rg -C 3 pattern         3 lines ctx   │
# └─────────────────────────────────────────┘
```

**Tasks:**
- [x] Create cheatsheet prompt template
- [x] Fetch --help output for tool
- [x] Generate concise cheatsheet with AI
- [x] Format for terminal display
- [x] Cache generated cheatsheets

---

### 2.5 Usage Analysis & Tips ✅

**Status:** COMPLETED

```bash
hoards ai analyze              # Full analysis with AI insights
hoards ai analyze --no-ai      # Static rules only (fast)
hoards ai analyze --json       # JSON output for scripts
hoards ai analyze --min-uses 5 # Lower threshold
```

**Features:**
- Detects when traditional Unix tools (grep, find, cat, etc.) are used but modern alternatives are installed
- Identifies high-value unused tools sorted by GitHub stars
- Optional AI-generated personalized insights
- JSON output for scripting

**Tasks:**
- [x] Analyze usage patterns for inefficiencies
- [x] Detect traditional vs modern tool usage
- [x] Generate actionable recommendations
- [x] Identify underutilized installed tools

---

### 2.6 Migration Assistant ✅

**Status:** COMPLETED

```bash
hoards ai migrate                    # Auto-detect best migrations
hoards ai migrate --from apt         # Migrate from apt only
hoards ai migrate --from apt --to cargo  # Explicit source pair
hoards ai migrate --dry-run          # Preview without executing
hoards ai migrate --json             # JSON output for scripts
hoards ai migrate --no-ai            # Skip AI benefit descriptions
```

**Features:**
- Finds tools that have newer versions on other package sources
- Optional AI-generated benefit descriptions for each migration
- Interactive selection (migrate all / select / cancel)
- Safe execution: install new before removing old
- Database updated after successful migration

**Tasks:**
- [x] Compare versions across sources
- [x] Identify migration candidates
- [x] Generate migration plan
- [x] Execute migration with database update

---

### 2.7 Real-time Usage Tracking ✅

**Status:** COMPLETED

Shell hooks for real-time command tracking, eliminating the need for periodic history scans.

```bash
# Configure tracking mode
hoards usage config --mode hook

# Output:
# > Switching to hook mode...
# > Detected shell: zsh
#
# ? Add hook to ~/.zshrc automatically? [Y/n] y
#
# > Adding hook to ~/.zshrc...
# + Hook added successfully!
# + Configuration saved.
```

**Commands:**
```bash
hoards usage config              # View/change tracking mode
hoards usage config --mode scan  # Use history scanning
hoards usage config --mode hook  # Use shell hooks
hoards usage init [shell]        # Show/setup hook instructions
hoards usage log <cmd>           # Log a command (called by hook)
hoards usage reset [-f]          # Reset all counters
```

**Implementation:**
- [x] Add `UsageConfig` and `UsageMode` to config
- [x] Add `usage log` command for shell hooks
- [x] Add `usage init` command for setup instructions
- [x] Add `usage config` command for mode management
- [x] Add `usage reset` command for counter reset
- [x] Automatic shell hook setup for Fish, Zsh, Bash
- [x] Automatic bash-preexec download and installation
- [x] Idempotent setup (detects existing hooks)
- [x] Add `match_command_to_tool()` DB method for fast lookup

**Shell Support:**
| Shell | Config File | Hook Setup |
|-------|-------------|------------|
| Fish | `~/.config/fish/config.fish` | Automatic |
| Zsh | `~/.zshrc` | Automatic |
| Bash | `~/.bashrc` + `~/.bash-preexec.sh` | Automatic (downloads bash-preexec) |

---

## Phase 3: TUI MVP ✅

**Status:** COMPLETED

**Goal:** Build a functional terminal UI using Ratatui.

### 3.1 Project Setup ✅

**Tasks:**
- [x] Add ratatui and crossterm dependencies
- [x] Create `src/tui/` module structure
- [x] Set up basic app state management
- [x] Implement terminal initialization/cleanup
- [x] Add `hoard tui` command entry point

**File structure:**
```
src/tui/
├── mod.rs          # Module exports
├── app.rs          # App state and logic
├── ui.rs           # UI rendering
├── event.rs        # Event handling
└── theme.rs        # Theme definitions
```

---

### 3.2 Core Layout ✅

```
┌─────────────────────────────────────────────────────────────┐
│ hoard  [1]Installed [2]Available [3]Updates [4]Bundles [5]Discover │
├────────────────────────┬────────────────────────────────────┤
│ Tools            [147] │ Details                            │
│ ────────────────────── │ ──────────────────────────────────│
│                        │                                    │
│  (list widget)         │  (details widget)                  │
│                        │                                    │
├────────────────────────┴────────────────────────────────────┤
│ (status bar with keybindings)   🤖  ⟳ 5m v0.2.1           │
└─────────────────────────────────────────────────────────────┘
```

**Tasks:**
- [x] Implement main layout with constraints
- [x] Create header with tab bar
- [x] Create left panel (tool list)
- [x] Create right panel (details)
- [x] Create footer (status/help bar with AI/GitHub indicators)
- [x] Implement responsive resizing (stacked layout for narrow terminals)

---

### 3.3 Navigation & Input ✅

**Tasks:**
- [x] Implement vim-style navigation (j/k/g/G)
- [x] Implement tab switching (1-5, [, ])
- [x] Implement selection (space, Ctrl+a, x)
- [x] Implement search mode (/)
- [x] Implement help modal (?)
- [x] Handle terminal resize events

---

### 3.4 Tab Views ✅

**Installed Tab:**
- [x] List installed tools with status indicators
- [x] Show source, usage count, GitHub stars
- [x] Sort by name/usage/date
- [x] Filter by search query

**Available Tab:**
- [x] List tools in database but not installed
- [x] Show GitHub stars, descriptions
- [x] Quick install action

**Updates Tab:**
- [x] List tools with available updates
- [x] Show current vs available version
- [x] Batch update selection

**Bundles Tab:**
- [x] List bundles with tool counts
- [x] Show bundle contents with install status
- [x] Quick install bundle action (i)
- [x] Track missing tools to Available (a)

**Discover Tab (shell only):**
- [x] Basic UI structure with search bar and results area
- [x] Empty state with usage instructions
- [ ] Search functionality (see Phase 5)
- [ ] AI integration (see Phase 5)

---

### 3.5 Actions ✅

**Tasks:**
- [x] Implement install action (i)
- [x] Implement uninstall action (D)
- [x] Implement update action (u)
- [x] Implement refresh action (r)
- [x] Show confirmation dialogs for destructive actions
- [x] Show progress indicators for long operations

---

## Phase 4: TUI Polish ✅

**Status:** COMPLETED

**Goal:** Add advanced features and polish.

### 4.1 Visual Enhancements ✅

**Tasks:**
- [x] Add usage sparklines (7-day trend) with dimmed theme colors
- [x] Add health indicators (● green/yellow/red based on recency)
- [x] Add GitHub stars inline (★ 1.2K format)
- [x] Implement theme support (6 themes: Catppuccin Mocha/Latte, Dracula, Nord, Tokyo Night, Gruvbox)
- [x] Add loading indicators
- [x] Add success/error status messages
- [x] Add labels as colored pills in details view
- [x] User-friendly datetime formatting with local timezone

---

### 4.2 Advanced Features ✅

**Tasks:**
- [x] Implement undo/redo system (Ctrl+z/Ctrl+y)
- [x] Add command palette (:) with vim-style commands
- [x] Implement fuzzy search (fzf-style with scoring)
- [x] Add mouse support (click, scroll, right-click)
- [x] Implement bulk operations UI (multi-select)
- [ ] Add AI assistant panel (see Phase 5)

---

### 4.3 Configuration Menu & JSON Config

**Status:** ✅ COMPLETE

**Goal:** Migrate from TOML to JSON config with JSON Schema validation, and add interactive config menu in TUI.

#### Config File Migration (TOML → JSON)

**Current format:** `~/.config/hoards/config.toml`
```toml
[ai]
provider = "claude"

[usage]
mode = "hook"
shell = "fish"
```

**New format:** `~/.config/hoards/config.json`
```json
{
  "$schema": "https://raw.githubusercontent.com/user/hoards/main/schema/config.schema.json",
  "ai": {
    "provider": "claude"
  },
  "usage": {
    "mode": "hook",
    "shell": "fish"
  },
  "tui": {
    "theme": "catppuccin-mocha"
  },
  "sources": {
    "cargo": true,
    "apt": true,
    "pip": false,
    "npm": false,
    "brew": false,
    "flatpak": true,
    "manual": true
  }
}
```

**Tasks - Config Migration:**
- [x] Create JSON Schema (`schema/config.schema.json`)
- [x] Update `HoardConfig` struct with new fields
- [x] Switch from `toml` to `serde_json` for config serialization
- [x] Add migration logic (read TOML if exists, write JSON)
- [x] Update `config_path()` to return `.json` extension
- [ ] Add schema validation on load (deferred - JSON serialization handles validation)

#### Config Menu in TUI

**Trigger:**
- Shortcut key `c` or `:config` command
- Auto-launch on first run (no config file exists)

**Menu Structure:**
```
┌─────────────────── Configuration ────────────────────┐
│                                                       │
│  AI Provider                                          │
│  ○ None (disabled)                                    │
│  ○ Claude                                             │
│  ● Gemini                                             │
│  ○ Codex                                              │
│  ○ Opencode                                           │
│                                                       │
│  Theme                                                │
│  ● Catppuccin Mocha                                   │
│  ○ Catppuccin Latte                                   │
│  ○ Dracula                                            │
│  ○ Nord                                               │
│  ○ Tokyo Night                                        │
│  ○ Gruvbox                                            │
│                                                       │
│  Package Managers (select all that apply)             │
│  [x] cargo     [x] apt      [ ] pip                   │
│  [ ] npm       [ ] brew     [x] flatpak               │
│  [x] manual                                           │
│                                                       │
│  Usage Tracking                                       │
│  ○ Hook (real-time)                                   │
│  ● Scan (manual)                                      │
│                                                       │
│  [Save]  [Cancel]                                     │
└───────────────────────────────────────────────────────┘
```

**Tasks - Config Menu:**
- [x] Create `ConfigMenuState` in `app.rs`
- [x] Add overlay-style config menu (like help panel)
- [x] Implement `render_config_menu()` in `ui.rs`
- [x] Handle config menu navigation (j/k, Tab, space, Enter)
- [x] Add `c` keybinding to open config
- [x] Add `:config` command (also `:settings`, `:cfg`)
- [x] Auto-launch config menu if no config file exists
- [x] Save config and apply changes immediately (theme, etc.)

**Tasks - General:**
- [ ] Configurable keybindings (future)
- [x] Theme cycling (t key)
- [ ] Configurable default view (future)
- [ ] Persist window state (future)

---

## Phase 5: TUI Discover Tab ✅

**Status:** COMPLETE

**Goal:** Implement search and AI capabilities in the Discover tab.

### 5.1 External Search Integration ✅

**Implemented Sources (7 total):**

| Source | Implementation | Install Command |
|--------|----------------|-----------------|
| crates.io | REST API (`/api/v1/crates`) | `cargo install {pkg}` |
| npm | REST API (`/-/v1/search`) | `npm install -g {pkg}` |
| PyPI | HTML scraping (no official API) | `pip install {pkg}` |
| Homebrew | `brew search` CLI | `brew install {pkg}` |
| apt | `apt-cache search` CLI | `sudo apt install {pkg}` |
| GitHub | `gh search repos` CLI | Language-based detection |
| AI | Background thread with LLM | Parsed from response |

**Features:**
- [x] Sequential search across enabled sources with progress indicator
- [x] Real-time result accumulation as each source completes
- [x] Smart deduplication (normalized names, merges install options)
- [x] Source availability detection (checks if CLI tools installed)
- [x] Config-aware (respects enabled sources in settings)

**Tasks:**
- [x] Implement GitHub search via `gh` CLI
- [x] Implement crates.io search
- [x] Implement PyPI search (HTML scraping)
- [x] Implement npm search
- [x] Implement Homebrew search
- [x] Implement apt search
- [x] Add source filtering in UI (F1-F6 toggles)
- [x] Deduplicate results across sources
- [x] Handle errors gracefully per-source

---

### 5.2 Search UX ✅

**Implemented:** Submit-on-Enter with search history

**Features:**
- [x] `/` enters search mode in Discover tab
- [x] Enter submits search across all enabled sources
- [x] Up/Down arrows navigate search history (last 100 searches)
- [x] Search history persisted to database
- [x] Loading overlay with step progress (`current/total`)
- [x] Results count shown during search

**Tasks:**
- [x] Implement search input handling in Discover tab
- [x] Add loading indicator during search
- [x] Handle search errors gracefully
- [x] Add search history (up/down to recall)

---

### 5.3 AI Integration ✅

**Implemented:** Toggle-based AI mode with context-aware recommendations

**Features:**
- [x] Shift+A toggles AI search mode
- [x] AI runs in background thread (non-blocking)
- [x] Context includes: installed tools, enabled sources
- [x] Customizable prompt template (`~/.config/hoards/prompts/discovery.txt`)
- [x] Supports multiple providers: Claude (Haiku/Sonnet/Opus), Gemini, Codex, OpenCode
- [x] Elapsed time shown during AI search
- [x] Results parsed from JSON response

**Tasks:**
- [x] Define AI prompt templates for discovery
- [x] Implement AI toggle in search controls
- [x] Show AI-suggested tools with parsed metadata
- [x] Run AI in background thread
- [x] Support multiple AI providers with model selection

---

### 5.4 Actions on Results ✅

**Implemented Actions:**

| Key | Action | Status |
|-----|--------|--------|
| `j/k` | Navigate results | ✅ |
| `g/G` | First/last result | ✅ |
| `s` | Cycle sort (stars/name/source) | ✅ |
| `Enter` | View README popup | ✅ |
| `o` | Open URL in browser | ✅ |
| `i` | Show install instructions | ✅ (CLI guidance) |
| `F1-F6` | Toggle source filters | ✅ |
| Mouse | Scroll, click | ✅ |

**README Popup Features:**
- [x] Markdown rendering
- [x] Link extraction with `o` to open link picker
- [x] Scrollable with j/k or mouse
- [x] Shows description, stars, URL, install options

**Tasks:**
- [x] Implement README viewing (Enter)
- [x] Implement open URL in browser (cross-platform)
- [x] Add details view with install options
- [x] Show multiple install options per tool

**Not Implemented (future):**
- [ ] Direct installation from TUI (currently shows CLI instructions)
- [ ] Add to Available/bundle from discover

---

### 5.5 UI Refinements ✅

**Implemented:**
- [x] Source filter toggles with F-key bindings
- [x] Search scope indicator (checkbox chips in header)
- [x] Keyboard shortcuts in empty state
- [x] Graceful empty results handling
- [x] Sorting by stars/name/source
- [x] Responsive layout (40/60 split or full list)
- [x] Scrollbar with thumb indicator

**Search Controls UI:**
```
[x]🤖  F1[x]🦀 F2[x]📦 F3[x]🐍 F4[x]🍺 F5[x]📋 F6[ ]🐙    [Search...]
```

**Tasks:**
- [x] Add source filter toggles in UI
- [x] Show search scope indicator
- [x] Add keyboard shortcuts help
- [x] Handle empty results gracefully
- [ ] Add "popular tools" default view (trending) - deferred

---

## Success Metrics

### Phase 1 ✅
- [x] Command count reduced from 27 to ~15
- [x] All commands have `--help` with examples
- [x] Fish completions fully updated
- [x] No breaking changes (aliases work)

### Phase 2 ✅
- [x] AI extraction works for 90%+ of GitHub repos
- [x] Bundle suggestions implemented with AI reasoning
- [x] Cheatsheets generated with caching
- [x] Real-time usage tracking via shell hooks
- [x] Auto-install shell completions (Fish, Bash, Zsh) during `hoards init`
- [x] Usage analysis detects traditional vs modern tool usage

### Phase 3 ✅
- [x] TUI launches quickly
- [x] All core operations available in TUI
- [x] Responsive layout (stacked for narrow terminals)

### Phase 4 ✅
- [x] Theme switching works (6 themes, t to cycle)
- [x] Undo/redo for selections and filters
- [x] Config menu with JSON config

### Phase 5 ✅
- [x] Search returns results from 7 sources (crates.io, npm, PyPI, brew, apt, GitHub, AI)
- [x] AI suggestions provide context-aware recommendations
- [x] Install instructions shown from discover (CLI-based)
- [x] Results accumulate in real-time during search

---

## Technical Debt & Cleanup

**During implementation:**
- [x] Add integration tests for new commands
- [x] Update all documentation
- [ ] Remove deprecated code after 1 version
- [x] Ensure 0 clippy warnings maintained
- [x] Keep test count growing (currently 176 tests - up from 118)
- [x] Pre-commit hooks for code quality
- [x] Add cargo-deny for dependency auditing

---

## Technical Debt Audit (January 2026)

### Summary

| Metric | Value | Status |
|--------|-------|--------|
| Test Count | 176 tests | ✅ Excellent (+49% from 118) |
| God Modules (>1500 lines) | 0 | ✅ Resolved |
| Unwrap Calls | 1 risky | ⚠️ Low priority |
| Security Vulnerabilities | 0 | ✅ Clean |
| Outdated Dependencies | 0 | ✅ Current |

**Overall Debt Score: LOW** - Well-structured codebase with minimal issues.

### Resolved Issues ✅

#### 1. God Modules - FIXED

| File | Before | After | Resolution |
|------|--------|-------|------------|
| `src/db.rs` | 1,701 lines | Split into 9 modules | `src/db/` directory |
| `src/main.rs` | 1,607 lines | 499 lines | Commands moved to `src/commands/` |

**Database module structure:**
```
src/db/
├── mod.rs          (582 lines - struct + integration tests)
├── schema.rs       (135 lines - migrations)
├── tools.rs        (326 lines - tool CRUD)
├── bundles.rs      (188 lines - bundle ops)
├── configs.rs      (164 lines - config tracking)
├── labels.rs       (127 lines - labeling)
├── github.rs       (160 lines - GitHub metadata)
├── usage.rs        (307 lines - usage tracking)
├── extractions.rs  (167 lines - AI cache)
└── discover.rs     (167 lines - search results)
```

#### 2. Test Coverage - IMPROVED

- **176 tests** total (164 unit + 12 integration)
- 29 database tests
- 27 TUI tests
- 23 package source tests
- 10 security tests (injection, validation)

### Remaining Items

#### Low Priority
- [ ] Fix unwrap in `src/updates.rs:39` (safe but not idiomatic)
- [ ] Add file size warnings to CI (optional)

### Positive Findings

- ✅ No security vulnerabilities (cargo audit clean)
- ✅ All dependencies at latest versions
- ✅ All licenses MIT-compatible
- ✅ No TODO/FIXME comments
- ✅ No circular dependencies
- ✅ No unsafe code blocks
- ✅ Comprehensive test coverage across all modules
- ✅ All clippy warnings resolved

### Prevention Measures

**Code Review Checklist:**
- No new files >500 lines
- No new functions >50 lines
- Tests required for new functionality
- No new `unwrap()` in production code

**CI Quality Gates:**
- `cargo deny check` for dependencies
- `cargo audit` for security
- `cargo clippy` for linting

---

## What's Next (Phase 6+)

### Potential Features

**Direct TUI Installation:**
- Execute install commands from Discover tab instead of showing instructions
- Progress indicator during installation
- Auto-add to database after successful install

**Parallel Search:**
- Use `thread::scope` to search all sources concurrently
- Faster results for multi-source queries

**Trending/Popular View:**
- Default view in Discover showing popular tools
- Curated "awesome-cli" style lists

**Local Model Support:**
- Ollama integration for privacy-conscious users
- Offline AI recommendations

**Sync Daemon:**
- Background process for auto-updates
- Desktop notifications for new tool versions

---

## Open Questions

1. **Backwards compatibility:** How long to maintain aliases?
2. **AI provider:** Default to Claude? Support local models (Ollama)?
3. **TUI as default?** Should `hoard` without args launch TUI?
4. **Sync daemon?** Background process for auto-updates?

---

## Appendix: Command Migration Guide

| Old Command | New Command | Status |
|-------------|-------------|--------|
| `hoard scan` | `hoard sync --scan` | Alias kept |
| `hoard list` | `hoard discover list` | Alias kept |
| `hoard search` | `hoard discover search` | Alias kept |
| `hoard categories` | `hoard discover categories` | Deprecated |
| `hoard labels` | `hoard discover labels` | Deprecated |
| `hoard suggest` | `hoard discover missing` | Deprecated |
| `hoard recommend` | `hoard discover recommended` | Deprecated |
| `hoard usage show` | `hoard insights usage` | Deprecated |
| `hoard unused` | `hoard insights unused` | Alias kept |
| `hoard stats` | `hoard insights stats` | Deprecated |
| `hoard doctor` | `hoard insights health` | Deprecated |
| `hoard info` | `hoard insights stats` | Deprecated |
| `hoard ai set` | `hoard ai config set` | Deprecated |
| `hoard ai show` | `hoard ai config show` | Deprecated |
| `hoard ai test` | `hoard ai config test` | Deprecated |
| `hoard ai categorize` | `hoard ai enrich --categorize` | Deprecated |
| `hoard ai describe` | `hoard ai enrich --describe` | Deprecated |
| `hoard gh sync` | `hoard sync --github` | Deprecated |
| `hoard gh search` | `hoard discover search --github` | Deprecated |
| `hoard gh info` | `hoard show <tool>` | Deprecated |
| `hoard gh rate-limit` | `hoard insights health` | Deprecated |
