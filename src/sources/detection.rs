//! Package manager detection and version checking

use std::collections::HashMap;
use std::process::Command;

/// Information about a package manager's availability
#[derive(Debug, Clone, Default)]
pub struct PackageManagerInfo {
    /// Whether the package manager binary is available
    pub available: bool,
    /// Version string if available (e.g., "1.75.0")
    pub version: Option<String>,
}

/// Collection of all package manager availability info
#[derive(Debug, Clone, Default)]
pub struct PackageManagerStatus {
    managers: HashMap<String, PackageManagerInfo>,
}

impl PackageManagerStatus {
    /// Detect all package managers and their versions
    pub fn detect() -> Self {
        let mut managers = HashMap::new();

        // Cargo (Rust)
        managers.insert("cargo".to_string(), detect_cargo());

        // Apt (Debian/Ubuntu)
        managers.insert("apt".to_string(), detect_apt());

        // Pip (Python)
        managers.insert("pip".to_string(), detect_pip());

        // Npm (Node.js)
        managers.insert("npm".to_string(), detect_npm());

        // Brew (Homebrew)
        managers.insert("brew".to_string(), detect_brew());

        // Go
        managers.insert("go".to_string(), detect_go());

        // Flatpak
        managers.insert("flatpak".to_string(), detect_flatpak());

        // Manual is always available (it's not a real package manager)
        managers.insert(
            "manual".to_string(),
            PackageManagerInfo {
                available: true,
                version: None,
            },
        );

        Self { managers }
    }

    /// Check if a package manager is available
    pub fn is_available(&self, name: &str) -> bool {
        self.managers.get(name).is_some_and(|info| info.available)
    }

    /// Get the version string for a package manager
    pub fn version(&self, name: &str) -> Option<&str> {
        self.managers
            .get(name)
            .and_then(|info| info.version.as_deref())
    }

    /// Get info for a package manager
    pub fn get(&self, name: &str) -> Option<&PackageManagerInfo> {
        self.managers.get(name)
    }
}

/// Detect cargo and get version
fn detect_cargo() -> PackageManagerInfo {
    let output = Command::new("cargo").arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            // "cargo 1.75.0 (1d8b05cdd 2023-11-20)" -> "1.75.0"
            let version = version.split_whitespace().nth(1).map(|s| s.to_string());
            PackageManagerInfo {
                available: true,
                version,
            }
        }
        _ => PackageManagerInfo::default(),
    }
}

/// Detect apt and get version
fn detect_apt() -> PackageManagerInfo {
    let output = Command::new("apt").arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            // "apt 2.6.1 (amd64)" -> "2.6.1"
            let version = version.split_whitespace().nth(1).map(|s| s.to_string());
            PackageManagerInfo {
                available: true,
                version,
            }
        }
        _ => PackageManagerInfo::default(),
    }
}

/// Detect pip and get version
fn detect_pip() -> PackageManagerInfo {
    // Try pip3 first, then pip
    let output = Command::new("pip3")
        .arg("--version")
        .output()
        .or_else(|_| Command::new("pip").arg("--version").output());

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            // "pip 24.0 from /usr/lib/..." -> "24.0"
            let version = version.split_whitespace().nth(1).map(|s| s.to_string());
            PackageManagerInfo {
                available: true,
                version,
            }
        }
        _ => PackageManagerInfo::default(),
    }
}

/// Detect npm and get version
fn detect_npm() -> PackageManagerInfo {
    let output = Command::new("npm").arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            // "10.2.4" -> "10.2.4"
            let version = version.trim().to_string();
            PackageManagerInfo {
                available: true,
                version: Some(version),
            }
        }
        _ => PackageManagerInfo::default(),
    }
}

/// Detect brew and get version
fn detect_brew() -> PackageManagerInfo {
    let output = Command::new("brew").arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            // "Homebrew 4.2.0" -> "4.2.0"
            let version = version
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .map(|s| s.to_string());
            PackageManagerInfo {
                available: true,
                version,
            }
        }
        _ => PackageManagerInfo::default(),
    }
}

/// Detect go and get version
fn detect_go() -> PackageManagerInfo {
    let output = Command::new("go").arg("version").output();
    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            // "go version go1.21.5 linux/amd64" -> "1.21.5"
            let version = version
                .split_whitespace()
                .nth(2)
                .and_then(|s| s.strip_prefix("go"))
                .map(|s| s.to_string());
            PackageManagerInfo {
                available: true,
                version,
            }
        }
        _ => PackageManagerInfo::default(),
    }
}

/// Detect flatpak and get version
fn detect_flatpak() -> PackageManagerInfo {
    let output = Command::new("flatpak").arg("--version").output();
    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            // "Flatpak 1.14.4" -> "1.14.4"
            let version = version.split_whitespace().nth(1).map(|s| s.to_string());
            PackageManagerInfo {
                available: true,
                version,
            }
        }
        _ => PackageManagerInfo::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_status_for_all_sources() {
        let status = PackageManagerStatus::detect();

        // All sources should have an entry
        assert!(status.get("cargo").is_some());
        assert!(status.get("apt").is_some());
        assert!(status.get("pip").is_some());
        assert!(status.get("npm").is_some());
        assert!(status.get("brew").is_some());
        assert!(status.get("go").is_some());
        assert!(status.get("flatpak").is_some());
        assert!(status.get("manual").is_some());
    }

    #[test]
    fn test_manual_always_available() {
        let status = PackageManagerStatus::detect();
        assert!(status.is_available("manual"));
    }

    #[test]
    fn test_unknown_source_not_available() {
        let status = PackageManagerStatus::detect();
        assert!(!status.is_available("unknown"));
        assert!(status.version("unknown").is_none());
    }
}
