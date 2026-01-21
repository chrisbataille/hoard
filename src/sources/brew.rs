//! Homebrew package source

use super::PackageSource;
use crate::http::HTTP_AGENT;
use crate::models::{InstallSource, Tool};
use crate::scanner::{KNOWN_TOOLS, is_installed};
use anyhow::Result;
use std::process::Command;

pub struct BrewSource;

impl PackageSource for BrewSource {
    fn name(&self) -> &'static str {
        "brew"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Brew
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        // Use --versions to get both package name and version
        let output = Command::new("brew")
            .args(["list", "--formula", "--versions"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tools = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Format: "package version" or "package version1 version2" (multiple versions)
            let parts: Vec<&str> = line.split_whitespace().collect();
            let package = match parts.first() {
                Some(p) => *p,
                None => continue,
            };
            // Take the first version (most recent)
            let version = parts.get(1).map(|s| s.to_string());

            // Skip if already in KNOWN_TOOLS
            if KNOWN_TOOLS.iter().any(|kt| kt.name == package) {
                continue;
            }

            // Check if package has a binary in PATH
            if !is_installed(package) {
                continue;
            }

            let mut tool = Tool::new(package)
                .with_source(InstallSource::Brew)
                .with_binary(package)
                .with_category("cli")
                .with_install_command(self.install_command(package))
                .installed();

            // Set installed version if available
            if let Some(ver) = version {
                tool = tool.with_installed_version(ver);
            }

            tools.push(tool);
        }

        Ok(tools)
    }

    fn fetch_description(&self, package: &str) -> Option<String> {
        let url = format!("https://formulae.brew.sh/api/formula/{}.json", package);
        let mut response = HTTP_AGENT.get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("desc")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    fn install_command(&self, package: &str) -> String {
        format!("brew install {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("brew uninstall {}", package)
    }

    fn supports_updates(&self) -> bool {
        true
    }

    fn check_update(&self, package: &str, _current_version: &str) -> Option<String> {
        let url = format!("https://formulae.brew.sh/api/formula/{}.json", package);
        let mut response = HTTP_AGENT.get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("versions")?
            .get("stable")?
            .as_str()
            .map(|s| s.to_string())
    }
}
