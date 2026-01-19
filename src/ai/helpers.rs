//! Utility functions for AI features
//!
//! Helper functions for interacting with external tools and GitHub.

use std::process::Command;

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose};

/// Parse a GitHub URL to extract owner and repo
pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    // Handle various GitHub URL formats:
    // https://github.com/owner/repo
    // https://github.com/owner/repo.git
    // https://github.com/owner/repo/...
    // git@github.com:owner/repo.git
    // owner/repo (shorthand)

    let url = url.trim();

    // Shorthand format: owner/repo
    if !url.contains("github.com") && url.contains('/') && !url.contains(':') {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    // SSH format: git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.trim_start_matches("git@github.com:");
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    // HTTPS format
    if let Some(path) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    bail!("Invalid GitHub URL format: {}", url)
}

/// Fetch README content from GitHub using gh CLI
pub fn fetch_readme(owner: &str, repo: &str) -> Result<String> {
    let output = Command::new("gh")
        .args(["api", &format!("repos/{}/{}/readme", owner, repo)])
        .output()
        .context("Failed to run gh api")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to fetch README: {}", stderr);
    }

    #[derive(serde::Deserialize)]
    struct ReadmeResponse {
        content: String,
        encoding: String,
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let readme: ReadmeResponse =
        serde_json::from_str(&stdout).context("Failed to parse README response")?;

    if readme.encoding != "base64" {
        bail!("Unexpected README encoding: {}", readme.encoding);
    }

    // Decode base64 content
    let decoded = general_purpose::STANDARD
        .decode(readme.content.replace('\n', ""))
        .context("Failed to decode README content")?;

    String::from_utf8(decoded).context("README is not valid UTF-8")
}

/// Fetch the latest commit SHA for a repo (used for cache versioning)
pub fn fetch_repo_version(owner: &str, repo: &str) -> Result<String> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/{}/commits/HEAD", owner, repo),
            "--jq",
            ".sha",
        ])
        .output()
        .context("Failed to run gh api")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to fetch repo version: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get tool version by running `tool --version`
pub fn get_tool_version(binary: &str) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        let version = version.trim();
        if !version.is_empty() {
            // Extract just the version number if possible (first line, cleaned up)
            let first_line = version.lines().next().unwrap_or(version);
            return Some(first_line.to_string());
        }
    }

    None
}

/// Check if a binary is installed on the system
pub fn is_binary_installed(binary: &str) -> bool {
    which::which(binary).is_ok()
}

/// Get --help output for a tool
pub fn get_help_output(binary: &str) -> Result<String> {
    // Try --help first, then -h
    let output = Command::new(binary)
        .arg("--help")
        .output()
        .or_else(|_| Command::new(binary).arg("-h").output())
        .with_context(|| format!("Failed to run {} --help", binary))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Some tools output help to stderr
    let help_text = if stdout.len() > stderr.len() {
        stdout.to_string()
    } else {
        stderr.to_string()
    };

    if help_text.trim().is_empty() {
        bail!("No help output from {}", binary);
    }

    Ok(help_text)
}
