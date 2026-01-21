//! Version policy resolution and update decision logic

use crate::config::HoardConfig;
use crate::models::{Bundle, Tool, VersionPolicy};

/// Result of evaluating whether a tool should be updated
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateDecision {
    /// Tool should be updated (allowed by policy)
    Update,
    /// Major update available but skipped (stable policy)
    SkipMajor,
    /// Tool is pinned - no updates
    Pinned,
    /// Tool is up to date
    UpToDate,
    /// Cannot determine (missing version info)
    Unknown,
}

impl UpdateDecision {
    /// Get the display icon for this decision
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Update => "↑",
            Self::SkipMajor => "⚠",
            Self::Pinned => "📌",
            Self::UpToDate => "",
            Self::Unknown => "",
        }
    }

    /// Whether this decision indicates an update is available
    pub fn has_update(&self) -> bool {
        matches!(self, Self::Update | Self::SkipMajor)
    }
}

/// Resolve the effective version policy for a tool
///
/// Policy cascade (highest to lowest priority):
/// 1. Tool-level override
/// 2. Bundle policy (first matching bundle)
/// 3. Source-specific default
/// 4. Global default (Stable)
pub fn resolve_policy(tool: &Tool, bundles: &[Bundle], config: &HoardConfig) -> VersionPolicy {
    // 1. Tool-level override
    if let Some(policy) = &tool.version_policy {
        return policy.clone();
    }

    // 2. Bundle policy (first bundle that contains this tool with a policy)
    for bundle in bundles {
        if bundle.tools.contains(&tool.name)
            && let Some(policy) = &bundle.version_policy
        {
            return policy.clone();
        }
    }

    // 3. Source-specific default from config
    let source_name = tool.source.to_string();
    config.version_policy.policy_for_source(&source_name)
}

/// Determine if an update should be applied based on the version policy
pub fn should_update(
    current: Option<&str>,
    available: Option<&str>,
    policy: &VersionPolicy,
) -> UpdateDecision {
    // If pinned, never update
    if *policy == VersionPolicy::Pinned {
        return UpdateDecision::Pinned;
    }

    // Need both versions to make a decision
    let (current, available) = match (current, available) {
        (Some(c), Some(a)) => (c, a),
        _ => return UpdateDecision::Unknown,
    };

    // If versions are the same, no update needed
    if current == available {
        return UpdateDecision::UpToDate;
    }

    // Try to parse as semver
    match (parse_version(current), parse_version(available)) {
        (Some(curr), Some(avail)) => {
            // Check if there's actually an update available
            if avail <= curr {
                return UpdateDecision::UpToDate;
            }

            match policy {
                VersionPolicy::Latest => UpdateDecision::Update,
                VersionPolicy::Stable => {
                    // Skip major version bumps
                    if avail.major > curr.major {
                        UpdateDecision::SkipMajor
                    } else {
                        UpdateDecision::Update
                    }
                }
                VersionPolicy::Pinned => UpdateDecision::Pinned,
            }
        }
        // Fall back to string comparison if semver parsing fails
        _ => {
            if current != available {
                match policy {
                    VersionPolicy::Latest | VersionPolicy::Stable => UpdateDecision::Update,
                    VersionPolicy::Pinned => UpdateDecision::Pinned,
                }
            } else {
                UpdateDecision::UpToDate
            }
        }
    }
}

/// Parse a version string, stripping common prefixes
fn parse_version(version: &str) -> Option<semver::Version> {
    // Strip common prefixes
    let cleaned = version
        .trim()
        .strip_prefix('v')
        .or_else(|| version.strip_prefix('V'))
        .unwrap_or(version);

    // Try to parse as semver
    semver::Version::parse(cleaned).ok().or_else(|| {
        // Try to handle versions with extra components (e.g., "1.2.3.4" -> "1.2.3")
        let parts: Vec<&str> = cleaned.split('.').collect();
        if parts.len() >= 3 {
            let major = parts[0].parse::<u64>().ok()?;
            let minor = parts[1].parse::<u64>().ok()?;
            // Handle patch that might have additional suffix
            let patch_str = parts[2].split('-').next().unwrap_or(parts[2]);
            let patch_str = patch_str.split('+').next().unwrap_or(patch_str);
            let patch = patch_str.parse::<u64>().ok()?;
            Some(semver::Version::new(major, minor, patch))
        } else if parts.len() == 2 {
            // Handle "1.2" -> "1.2.0"
            let major = parts[0].parse::<u64>().ok()?;
            let minor = parts[1].parse::<u64>().ok()?;
            Some(semver::Version::new(major, minor, 0))
        } else {
            None
        }
    })
}

/// Classify the version change type
#[derive(Debug, Clone, PartialEq)]
pub enum VersionChange {
    Major,
    Minor,
    Patch,
    Unknown,
}

impl VersionChange {
    /// Get a human-readable label for the change type
    pub fn label(&self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::Unknown => "update",
        }
    }
}

/// Determine the type of version change
pub fn classify_change(current: Option<&str>, available: Option<&str>) -> VersionChange {
    let (current, available) = match (current, available) {
        (Some(c), Some(a)) => (c, a),
        _ => return VersionChange::Unknown,
    };

    match (parse_version(current), parse_version(available)) {
        (Some(curr), Some(avail)) => {
            if avail.major > curr.major {
                VersionChange::Major
            } else if avail.minor > curr.minor {
                VersionChange::Minor
            } else if avail.patch > curr.patch {
                VersionChange::Patch
            } else {
                VersionChange::Unknown
            }
        }
        _ => VersionChange::Unknown,
    }
}

/// Get policy source description for display
pub fn policy_source(tool: &Tool, bundles: &[Bundle], config: &HoardConfig) -> String {
    // Check tool override first
    if tool.version_policy.is_some() {
        return "tool override".to_string();
    }

    // Check bundles
    for bundle in bundles {
        if bundle.tools.contains(&tool.name) && bundle.version_policy.is_some() {
            return format!("bundle: {}", bundle.name);
        }
    }

    // Check source config
    let source_name = tool.source.to_string();
    if config
        .version_policy
        .sources
        .contains_key(&source_name.to_lowercase())
    {
        return format!("{} default", source_name);
    }

    "global default".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::InstallSource;

    #[test]
    fn test_parse_version_semver() {
        assert_eq!(parse_version("1.2.3"), Some(semver::Version::new(1, 2, 3)));
        assert_eq!(parse_version("v1.2.3"), Some(semver::Version::new(1, 2, 3)));
        assert_eq!(parse_version("V1.2.3"), Some(semver::Version::new(1, 2, 3)));
    }

    #[test]
    fn test_parse_version_two_parts() {
        assert_eq!(parse_version("1.2"), Some(semver::Version::new(1, 2, 0)));
    }

    #[test]
    fn test_parse_version_four_parts() {
        assert_eq!(
            parse_version("1.2.3.4"),
            Some(semver::Version::new(1, 2, 3))
        );
    }

    #[test]
    fn test_should_update_latest_policy() {
        let decision = should_update(Some("1.0.0"), Some("2.0.0"), &VersionPolicy::Latest);
        assert_eq!(decision, UpdateDecision::Update);
    }

    #[test]
    fn test_should_update_stable_minor() {
        let decision = should_update(Some("1.0.0"), Some("1.1.0"), &VersionPolicy::Stable);
        assert_eq!(decision, UpdateDecision::Update);
    }

    #[test]
    fn test_should_update_stable_major() {
        let decision = should_update(Some("1.0.0"), Some("2.0.0"), &VersionPolicy::Stable);
        assert_eq!(decision, UpdateDecision::SkipMajor);
    }

    #[test]
    fn test_should_update_pinned() {
        let decision = should_update(Some("1.0.0"), Some("2.0.0"), &VersionPolicy::Pinned);
        assert_eq!(decision, UpdateDecision::Pinned);
    }

    #[test]
    fn test_should_update_up_to_date() {
        let decision = should_update(Some("1.0.0"), Some("1.0.0"), &VersionPolicy::Latest);
        assert_eq!(decision, UpdateDecision::UpToDate);
    }

    #[test]
    fn test_should_update_missing_version() {
        let decision = should_update(None, Some("1.0.0"), &VersionPolicy::Latest);
        assert_eq!(decision, UpdateDecision::Unknown);

        let decision = should_update(Some("1.0.0"), None, &VersionPolicy::Latest);
        assert_eq!(decision, UpdateDecision::Unknown);
    }

    #[test]
    fn test_classify_change_major() {
        assert_eq!(
            classify_change(Some("1.0.0"), Some("2.0.0")),
            VersionChange::Major
        );
    }

    #[test]
    fn test_classify_change_minor() {
        assert_eq!(
            classify_change(Some("1.0.0"), Some("1.1.0")),
            VersionChange::Minor
        );
    }

    #[test]
    fn test_classify_change_patch() {
        assert_eq!(
            classify_change(Some("1.0.0"), Some("1.0.1")),
            VersionChange::Patch
        );
    }

    #[test]
    fn test_resolve_policy_tool_override() {
        let tool = Tool::new("test")
            .with_source(InstallSource::Cargo)
            .with_version_policy(VersionPolicy::Pinned);
        let bundles = vec![];
        let config = HoardConfig::default();

        assert_eq!(
            resolve_policy(&tool, &bundles, &config),
            VersionPolicy::Pinned
        );
    }

    #[test]
    fn test_resolve_policy_bundle_override() {
        let tool = Tool::new("test").with_source(InstallSource::Cargo);
        let bundle =
            Bundle::new("dev", vec!["test".to_string()]).with_version_policy(VersionPolicy::Latest);
        let bundles = vec![bundle];
        let config = HoardConfig::default();

        assert_eq!(
            resolve_policy(&tool, &bundles, &config),
            VersionPolicy::Latest
        );
    }

    #[test]
    fn test_resolve_policy_global_default() {
        let tool = Tool::new("test").with_source(InstallSource::Cargo);
        let bundles = vec![];
        let config = HoardConfig::default();

        assert_eq!(
            resolve_policy(&tool, &bundles, &config),
            VersionPolicy::Stable
        );
    }

    #[test]
    fn test_update_decision_icons() {
        assert_eq!(UpdateDecision::Update.icon(), "↑");
        assert_eq!(UpdateDecision::SkipMajor.icon(), "⚠");
        assert_eq!(UpdateDecision::Pinned.icon(), "📌");
        assert_eq!(UpdateDecision::UpToDate.icon(), "");
    }
}
