//! Go package source

use super::PackageSource;
use crate::models::{InstallSource, Tool};
use crate::scanner::{KNOWN_TOOLS, is_installed};
use anyhow::Result;
use std::path::PathBuf;

pub struct GoSource;

impl GoSource {
    /// Get the Go bin directory where installed tools are located
    fn go_bin_dir() -> Option<PathBuf> {
        // First check GOBIN
        if let Ok(gobin) = std::env::var("GOBIN") {
            let path = PathBuf::from(gobin);
            if path.is_dir() {
                return Some(path);
            }
        }

        // Then check GOPATH/bin
        if let Ok(gopath) = std::env::var("GOPATH") {
            let path = PathBuf::from(gopath).join("bin");
            if path.is_dir() {
                return Some(path);
            }
        }

        // Fall back to ~/go/bin (default GOPATH)
        if let Some(home) = dirs::home_dir() {
            let path = home.join("go").join("bin");
            if path.is_dir() {
                return Some(path);
            }
        }

        None
    }
}

impl PackageSource for GoSource {
    fn name(&self) -> &'static str {
        "go"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Go
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        let Some(bin_dir) = Self::go_bin_dir() else {
            return Ok(Vec::new());
        };

        let mut tools = Vec::new();

        // List all executables in the Go bin directory
        let entries = std::fs::read_dir(bin_dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Check if it's executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = path.metadata()
                    && metadata.permissions().mode() & 0o111 == 0
                {
                    continue;
                }
            }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            // Skip if already in KNOWN_TOOLS
            if KNOWN_TOOLS
                .iter()
                .any(|kt| kt.name == name || kt.binary == name)
            {
                continue;
            }

            // Check if the binary is in PATH and executable
            if !is_installed(name) {
                continue;
            }

            let tool = Tool::new(name)
                .with_source(InstallSource::Go)
                .with_binary(name)
                .with_category("cli")
                .with_install_command(self.install_command(name))
                .installed();

            tools.push(tool);
        }

        Ok(tools)
    }

    fn fetch_description(&self, _package: &str) -> Option<String> {
        // Go doesn't have a central registry with descriptions
        // Tools installed via go install typically come from GitHub
        // where we already fetch descriptions
        None
    }

    fn install_command(&self, package: &str) -> String {
        // For GitHub-based packages, the full path is needed
        // e.g., go install github.com/user/repo@latest
        format!("go install {}@latest", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        // Go doesn't have a built-in uninstall command
        // The binary can be removed directly from GOBIN/GOPATH/bin
        format!("rm $(go env GOBIN)/{}", package)
    }

    fn supports_updates(&self) -> bool {
        // Go doesn't have a central registry to check for updates
        // Re-running go install will fetch the latest version
        false
    }

    fn check_update(&self, _package: &str, _current_version: &str) -> Option<String> {
        // Go doesn't have a central registry to check for updates
        None
    }
}
