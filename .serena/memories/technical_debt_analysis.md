# Technical Debt Analysis - Hoards Project

## Analysis Date
January 17, 2026

## Key Metrics
- Total lines of code: 24,135
- TUI module: 5,497 lines (22.8% of codebase)
- Number of modules: 35+ files
- Unit tests: 146 passing
- Test modules: 13
- Clippy warnings: 1 (minor)
- Unsafe code: 0 instances

## Critical Findings

### 1. FILES EXCEEDING SIZE LIMITS

Files violating 500-line architectural limit:
- **src/tui/app.rs**: 2,536 lines (5x limit) - 97 public methods, 46 struct fields
- **src/commands/ai.rs**: 2,414 lines (4.8x limit)
- **src/tui/ui.rs**: 2,028 lines (4x limit) - 24 render functions
- **src/ai.rs**: 1,241 lines (2.5x limit)
- **src/scanner.rs**: 1,191 lines (2.4x limit)
- **src/cli.rs**: 1,060 lines (2.1x limit)
- **src/commands/install.rs**: 911 lines (1.8x limit)
- **src/commands/usage.rs**: 865 lines (1.7x limit)
- **src/updates.rs**: 726 lines (1.5x limit)
- **src/commands/bundle.rs**: 695 lines (1.4x limit)
- **src/commands/misc.rs**: 636 lines (1.3x limit)
- **src/db/mod.rs**: 580 lines (1.2x limit)
- **src/commands/config.rs**: 530 lines (1.1x limit)

Total overage: ~12,500 lines across 13 files

### 2. ARCHITECTURAL VIOLATIONS

**App Struct God Object**
- Location: src/tui/app.rs:736-810
- 46 public fields
- 97 public methods in single impl block
- Single responsibility violated (UI state, cache management, undo/redo, config)

**Database Re-exports**
- src/db/mod.rs: Only 3 public functions
- Delegates to 8 sub-modules (tools, usage, bundles, configs, extractions, labels, github, schema)
- Functions properly split but mod.rs is coupling point

**Command Module Duplication**
- src/commands/ai.rs: 30+ functions (13 public)
- src/commands/install.rs: 16 public functions
- src/commands/config.rs: 7 public functions
- src/commands/usage.rs: 11 public functions

### 3. FUNCTION COMPLEXITY ISSUES

Long functions exceeding 50-line limit:
- **src/tui/app.rs**:
  - fuzzy_match (60 lines, 329-389)
  - App impl block (1,474 lines with 97 methods)
  
- **src/tui/ui.rs** (13 render functions exceeding 50 lines):
  - render_tool_list (188 lines, 300-488)
  - render_details (199 lines, 490-689)
  - render_bundle_details (113 lines, 784-897)
  - render_discover (158 lines, 899-1057)
  - render_footer (194 lines, 1059-1253)
  - render_help_overlay (194 lines, 1255-1449)
  - render_config_menu (246 lines, 1451-1697) - WORST
  - render_details_popup (139 lines, 1699-1839)
  - render_loading_overlay (75 lines, 1840-1916)
  - render_confirmation_dialog (90 lines, 1917-2007)

- **src/commands/ai.rs**:
  - install_discovered_tool (199 lines, 1261-1460)
  - execute_migration (169 lines, 2135-2304)
  - parse_install_cmd_to_safe_command (104 lines, 2310-2414)

### 4. TEST COVERAGE GAPS

Tests: 146 unit tests + 378 integration test lines
Coverage analysis:
- **Well-tested**: Sources modules, App fuzzy matching, TUI navigation, DB CRUD
- **Gaps identified**:
  - No tests for render functions (24 functions in ui.rs untested)
  - No tests for mouse interaction handlers
  - No tests for background operations
  - AI command tests missing edge cases (11 functions, only basic coverage)
  - Command parsing tests incomplete
  - Error path testing inadequate (92 unwrap/expect calls found)

92 unwrap/expect calls in codebase - potential panic points in:
- History parsing
- Config loading
- Update checking

### 5. DOCUMENTATION DEBT

Documented in DOCUMENTATION_COVERAGE_REPORT.md:
- 954 doc comments in codebase (good baseline)
- Missing module-level documentation: src/commands/mod.rs
- Render functions in ui.rs lack doc comments (24 functions)
- Complex app.rs methods need more detailed docs

### 6. PERFORMANCE DEBT

From COMPREHENSIVE_REVIEW_REPORT.md:
- **db/mod.rs:540** - Clippy: needless_range_loop (for i in 0..6 using index)
- **Sequential operations** - No identified since HTTP_AGENT properly shared
- **Database transactions** - Properly implemented per CLAUDE.md
- **Lazy initialization** - HTTP_AGENT using LazyLock correctly

### 7. DEPENDENCY DEBT

Cargo.toml analysis:
- 24 dependencies (reasonable for feature set)
- All pinned to stable versions (good practice)
- No known security vulnerabilities reported by cargo audit
- Bundled SQLite (good for distribution)

### 8. CI/CD DEBT

.github/workflows/:
- **ci.yml**: Minimal but functional
  - Tests on main, develop branches
  - Clippy with -D warnings (strict)
  - Format checking
  - Missing: code coverage, MSRV testing, multi-platform builds, cargo audit
  
- **release-plz.yml**: Automated releases
  - Conventional commits respected
  - Missing: changelog validation, security scanning pre-release

### 9. RECENTLY CHANGED FILES (Feature Branch)

Modified on feature/tui-polish:
- src/db/tools.rs - Modified Jan 17
- src/tui/app.rs - Modified Jan 17
- src/tui/event.rs - Modified Jan 17
- src/tui/ui.rs - Modified Jan 17

### 10. KNOWN TECHNICAL DEBT (from CLAUDE.md)

All items marked [x] as resolved:
- main.rs bloat reduced to 386 lines ✓
- db.rs God Object split into 9 modules ✓
- Sequential HTTP parallelized ✓
- Integration tests added ✓
- Shell injection vulnerabilities fixed ✓
- atty dependency removed ✓
- Shared HTTP agent created ✓
- Database transactions implemented ✓
- Binary validation added ✓

## Severity Summary

| Category | Count | Impact |
|----------|-------|--------|
| File size violations | 13 | HIGH - 5,100% total overage |
| Long functions | 23 | HIGH - render complexity |
| Complex structs | 1 | MEDIUM - 46 fields in App |
| Test gaps | 40+ functions | MEDIUM - untested render/UI |
| Unwrap calls | 92 | MEDIUM - panic risk |
| Doc gaps | 24 functions | LOW - complex code undocumented |
| Linter warnings | 1 | LOW - trivial fix |
| Security issues | 0 detected | NONE |

## Recommendations Priority

1. **Immediate (Week 1)**:
   - Fix clippy warning in db/mod.rs:540
   - Extract render functions to separate module
   - Add tests for UI interaction handlers

2. **High Priority (Sprint)**:
   - Split src/tui/app.rs (2,536 → max 1,200)
   - Refactor src/tui/ui.rs (2,028 → max 1,200)
   - Extract long render functions

3. **Medium Priority (Month)**:
   - Add edge case tests for AI commands
   - Increase coverage in error paths
   - Add module-level docs

4. **Low Priority (Backlog)**:
   - Improve CI matrix testing
   - Add code coverage reporting
   - Document complex algorithms
