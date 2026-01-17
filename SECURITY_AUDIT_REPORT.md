# Security Audit Report - Hoards CLI Tool Manager

**Audit Date:** 2026-01-15
**Auditor:** Security Audit via Claude Code
**Version Audited:** 0.2.0
**Codebase Location:** `/home/chris/hoard`

---

## Executive Summary

This security audit identified **3 HIGH**, **5 MEDIUM**, and **4 LOW** severity vulnerabilities in the Hoards CLI tool manager. The application demonstrates good security practices in several areas (parameterized SQL queries, SafeCommand pattern for package installs), but contains critical command injection vectors in the AI migration functionality and has an unmaintained dependency with known issues.

### Risk Overview

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | - |
| HIGH | 3 | Requires immediate attention |
| MEDIUM | 5 | Should be addressed in next release |
| LOW | 4 | Address when convenient |

---

## Findings

### HIGH Severity

#### H1: Shell Command Injection in AI Migration (CVSS 7.8)

**Location:** `/home/chris/hoard/src/commands/ai.rs` (lines 1398, 2174, 2221, 2245)

**Description:** The `execute_migration` function and `install_discovered_tool` function use `sh -c` with user-controlled commands, bypassing the SafeCommand pattern used elsewhere.

**Vulnerable Code:**
```rust
// Line 1398 - install_discovered_tool fallback
match std::process::Command::new("sh").arg("-c").arg(cmd).status() {

// Line 2174 - execute_migration install
let install_result = Command::new("sh").arg("-c").arg(&install_cmd).output();

// Line 2245 - execute_migration uninstall
let result = Command::new("sh").arg("-c").arg(&uninstall_cmd).output();
```

**Attack Vector:** If AI responses are manipulated (prompt injection or compromised AI provider), the `install_cmd` or `cmd` variables could contain shell metacharacters leading to arbitrary command execution.

**Impact:** Complete system compromise if attacker controls AI response content.

**Remediation:**
1. Use the existing `SafeCommand` pattern consistently for all package operations
2. Validate AI-returned install commands against an allowlist of known safe patterns
3. Never pass user/AI-controlled strings directly to `sh -c`

**Proof of Concept:**
```
# If AI returns install command: "cargo install foo; rm -rf ~"
# The shell injection would execute the malicious command
```

---

#### H2: Unmaintained Dependency with Memory Safety Issue (CVSS 6.5)

**Location:** `Cargo.toml` - `atty = "0.2.14"`

**Description:** The `atty` crate is unmaintained (RUSTSEC-2024-0375) and has a known potential unaligned read vulnerability (RUSTSEC-2021-0145).

**Advisory Details:**
- RUSTSEC-2024-0375: Crate is officially unmaintained
- RUSTSEC-2021-0145: Potential memory unsoundness from unaligned reads

**Impact:** Potential memory corruption or undefined behavior. The maintainer has abandoned the crate and recommends migration.

**Remediation:**
Replace `atty` with `std::io::IsTerminal` (stable since Rust 1.70.0):

```rust
// Before
use atty::is;
if atty::is(atty::Stream::Stdout) { ... }

// After
use std::io::IsTerminal;
if std::io::stdout().is_terminal() { ... }
```

---

#### H3: Process Name Injection in pgrep/kill Commands (CVSS 6.8)

**Location:** `/home/chris/hoard/src/commands/install.rs` (lines 54-90)

**Description:** The `is_process_running`, `get_running_pids`, and `kill_processes` functions pass tool binary names directly to `pgrep` and `kill` without validation.

**Vulnerable Code:**
```rust
pub fn is_process_running(binary_name: &str) -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg(binary_name)  // Unvalidated user input
        .output()
        ...
}
```

**Attack Vector:** While `binary_name` typically comes from the database, a maliciously crafted tool name could exploit pgrep argument parsing.

**Impact:** Information disclosure about running processes or denial of service.

**Remediation:**
1. Apply the same `validate_package_name` function to binary names
2. Ensure binary names match expected patterns before use in process commands

---

### MEDIUM Severity

#### M1: SQL LIKE Pattern Injection (CVSS 5.3)

**Location:** `/home/chris/hoard/src/db.rs` (line 318)

**Description:** The `search_tools` function constructs LIKE patterns without escaping special SQL wildcards.

**Vulnerable Code:**
```rust
pub fn search_tools(&self, query: &str) -> Result<Vec<Tool>> {
    let pattern = format!("%{}%", query);
    ...
}
```

**Attack Vector:** User can inject SQL LIKE wildcards (`%`, `_`) to alter search behavior.

**Impact:** Information disclosure - attackers can enumerate database content using wildcard patterns.

**Remediation:**
```rust
fn escape_like_pattern(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('%', "\\%")
     .replace('_', "\\_")
}

pub fn search_tools(&self, query: &str) -> Result<Vec<Tool>> {
    let pattern = format!("%{}%", escape_like_pattern(query));
    ...
}
```

---

#### M2: Inadequate URL Validation for GitHub Operations (CVSS 5.3)

**Location:** `/home/chris/hoard/src/ai.rs` (lines 622-670, `parse_github_url`)

**Description:** GitHub URL parsing accepts shorthand format (`owner/repo`) which could be abused to construct unexpected API calls.

**Vulnerable Code:**
```rust
// Shorthand format: owner/repo
if !url.contains("github.com") && url.contains('/') && !url.contains(':') {
    let parts: Vec<&str> = url.split('/').collect();
    ...
}
```

**Impact:** Could be used to make unexpected API requests if combined with other vulnerabilities.

**Remediation:**
1. Add validation that owner/repo segments don't contain special characters
2. Validate against GitHub's username/repo naming rules

---

#### M3: HTTP User-Agent Information Disclosure (CVSS 4.3)

**Location:** `/home/chris/hoard/src/commands/ai.rs` (line 1482)

**Description:** HTTP requests to GitHub API use a generic User-Agent that identifies the tool.

**Code:**
```rust
let mut response = ureq::get(&url)
    .header("User-Agent", "hoards-cli")
    ...
```

**Impact:** Allows fingerprinting of hoards users making GitHub API requests.

**Remediation:** Consider using a more generic User-Agent or making it configurable.

---

#### M4: Shell History File Exposure (CVSS 5.0)

**Location:** `/home/chris/hoard/src/history.rs`

**Description:** The tool reads shell history files which may contain sensitive information (accidentally typed passwords, tokens, etc.). While documented in SECURITY.md, there's no filtering or warning for potentially sensitive commands.

**Impact:** If shell history contains secrets, these are processed by the application.

**Remediation:**
1. Add optional filtering for lines containing known sensitive patterns
2. Add a configuration option to exclude certain commands from analysis
3. Display a warning when scanning history for the first time

---

#### M5: Config File Path Traversal via Symlink (CVSS 5.5)

**Location:** `/home/chris/hoard/src/commands/config.rs`

**Description:** The config sync functionality creates symlinks based on user-provided paths. While it validates that source exists, it doesn't prevent symlink attacks or path traversal in target paths.

**Vulnerable Flow:**
```rust
pub fn cmd_config_link(
    db: &Database,
    name: &str,
    target: &str,  // User controlled
    source: &str,  // User controlled
    ...
) -> Result<()> {
    let target_path = expand_path(target);  // Only expands ~, no validation
    let source_path = expand_path(source);
    ...
}
```

**Impact:** User could potentially create symlinks pointing to sensitive locations outside expected directories.

**Remediation:**
1. Validate that paths are within expected directories
2. Use `canonicalize()` to resolve paths before operations
3. Add an allowlist of permitted parent directories

---

### LOW Severity

#### L1: Error Messages May Leak Path Information (CVSS 3.3)

**Location:** Multiple files (e.g., `/home/chris/hoard/src/db.rs`, `/home/chris/hoard/src/commands/config.rs`)

**Description:** Error messages include full file paths which could reveal system structure.

**Example:**
```rust
.with_context(|| format!("Failed to read config file: {}", path.display()))?
```

**Remediation:** Consider sanitizing paths in user-facing error messages in production builds.

---

#### L2: No Rate Limiting on External API Calls (CVSS 3.1)

**Location:** `/home/chris/hoard/src/scanner.rs`, `/home/chris/hoard/src/updates.rs`

**Description:** HTTP requests to PyPI, npm registry, and crates.io lack rate limiting beyond basic timeouts.

**Impact:** Could trigger rate limiting from registries, potentially causing temporary service disruption.

**Remediation:**
1. Add configurable delays between API calls
2. Implement exponential backoff on failures
3. Cache responses more aggressively

---

#### L3: Integer Overflow in Usage Counting (CVSS 2.5)

**Location:** `/home/chris/hoard/src/db.rs` (line 650)

**Description:** Usage counting uses `i64` which could theoretically overflow with extreme usage.

**Impact:** Minimal - would require 2^63 uses to overflow.

**Remediation:** Consider using saturating arithmetic or capping values.

---

#### L4: Missing Input Length Limits on Database Fields (CVSS 2.5)

**Location:** `/home/chris/hoard/src/db.rs`

**Description:** Database TEXT fields have no length limits, potentially allowing very large values.

**Impact:** Could lead to memory exhaustion with pathological inputs.

**Remediation:** Add length validation before database insertion for all TEXT fields.

---

## Positive Security Observations

The audit identified several well-implemented security measures:

### 1. SafeCommand Pattern (Excellent)
**Location:** `/home/chris/hoard/src/commands/install.rs`

The `SafeCommand` struct and associated functions (`get_safe_install_command`, `get_safe_uninstall_command`) demonstrate excellent security practices:
- Uses `Command::new().args()` instead of shell interpolation
- Validates package names with `validate_package_name()`
- Prevents path traversal with `..` detection
- Blocks shell metacharacters (`;`, `|`, `&`, `$`, backticks, etc.)

### 2. Parameterized SQL Queries (Excellent)
**Location:** `/home/chris/hoard/src/db.rs`

All database operations use parameterized queries with `params![]` macro:
```rust
self.conn.execute(
    "UPDATE tools SET description = ?1, updated_at = ?2 WHERE name = ?3",
    params![description, Utc::now().to_rfc3339(), name],
)?;
```

### 3. Dependency Security Tooling (Good)
The project includes `deny.toml` configuration for `cargo-deny`, showing proactive dependency management.

### 4. Security Policy (Good)
**Location:** `/home/chris/hoard/SECURITY.md`

Clear vulnerability reporting instructions and scope definition.

---

## Remediation Priority Matrix

| Finding | Severity | Effort | Priority |
|---------|----------|--------|----------|
| H1: Shell Command Injection | HIGH | Medium | 1 |
| H2: Unmaintained atty | HIGH | Low | 2 |
| H3: Process Name Injection | HIGH | Low | 3 |
| M1: SQL LIKE Injection | MEDIUM | Low | 4 |
| M5: Config Path Traversal | MEDIUM | Medium | 5 |
| M4: History File Exposure | MEDIUM | Medium | 6 |
| M2: URL Validation | MEDIUM | Low | 7 |
| M3: User-Agent Disclosure | MEDIUM | Low | 8 |
| L1-L4: Low items | LOW | Low | 9-12 |

---

## Recommended Actions

### Immediate (Before Next Release)

1. **Replace `sh -c` usage** in `src/commands/ai.rs` with SafeCommand pattern
2. **Migrate from `atty`** to `std::io::IsTerminal`
3. **Add validation** to binary names before pgrep/kill operations

### Short-term (Next Sprint)

4. **Escape SQL LIKE wildcards** in search_tools function
5. **Add path validation** to config link operations
6. **Consider adding sensitive pattern filtering** to history parsing

### Long-term (Technical Debt)

7. Review all `format!()` uses in command arguments
8. Implement comprehensive input validation layer
9. Add security-focused integration tests

---

## Conclusion

The Hoards project demonstrates awareness of security best practices with the SafeCommand pattern and parameterized queries. However, the AI integration introduced command injection vulnerabilities that bypass existing protections. The unmaintained `atty` dependency should be replaced as a priority.

With the recommended remediations, the application's security posture would significantly improve. The existing security infrastructure (SafeCommand, parameterized queries) provides a good foundation for secure development going forward.

---

**Report Generated:** 2026-01-15
**Methodology:** OWASP Top 10, RUSTSEC Advisory Database, Manual Code Review
