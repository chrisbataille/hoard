//! Version policy management commands

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::config::HoardConfig;
use crate::db::Database;
use crate::models::VersionPolicy;
use crate::version_policy::{policy_source, resolve_policy};

/// Set version policy for a tool
pub fn cmd_policy_set(db: &Database, name: &str, policy_str: &str) -> Result<()> {
    let policy = parse_policy(policy_str)?;

    // Check if tool exists
    let _tool = db
        .get_tool_by_name(name)?
        .context(format!("Tool '{}' not found", name))?;

    db.set_tool_version_policy(name, Some(&policy))?;

    println!(
        "{} Set version policy for {} to {}",
        "✓".green(),
        name.cyan(),
        format!("{}", policy).yellow()
    );

    // Show effective policy info
    let config = HoardConfig::load().unwrap_or_default();
    let bundles = db.list_bundles()?;
    let updated_tool = db.get_tool_by_name(name)?.unwrap();
    let effective = resolve_policy(&updated_tool, &bundles, &config);
    let source = policy_source(&updated_tool, &bundles, &config);
    println!("  Effective policy: {} (from: {})", effective, source);

    Ok(())
}

/// Clear version policy for a tool (use inherited policy)
pub fn cmd_policy_clear(db: &Database, name: &str) -> Result<()> {
    // Check if tool exists
    let _tool = db
        .get_tool_by_name(name)?
        .context(format!("Tool '{}' not found", name))?;

    db.set_tool_version_policy(name, None)?;

    println!(
        "{} Cleared version policy for {} (now using inherited policy)",
        "✓".green(),
        name.cyan()
    );

    // Show effective policy info
    let config = HoardConfig::load().unwrap_or_default();
    let bundles = db.list_bundles()?;
    let updated_tool = db.get_tool_by_name(name)?.unwrap();
    let effective = resolve_policy(&updated_tool, &bundles, &config);
    let source = policy_source(&updated_tool, &bundles, &config);
    println!("  Effective policy: {} (from: {})", effective, source);

    Ok(())
}

/// Set default policy for a package source
pub fn cmd_policy_set_source(source: &str, policy_str: &str) -> Result<()> {
    let policy = parse_policy(policy_str)?;
    let mut config = HoardConfig::load()?;

    config
        .version_policy
        .set_source_policy(source, policy.clone());
    config.save()?;

    println!(
        "{} Set default version policy for {} source to {}",
        "✓".green(),
        source.cyan(),
        format!("{}", policy).yellow()
    );

    Ok(())
}

/// Clear source-specific policy
pub fn cmd_policy_clear_source(source: &str) -> Result<()> {
    let mut config = HoardConfig::load()?;

    config.version_policy.clear_source_policy(source);
    config.save()?;

    println!(
        "{} Cleared source-specific policy for {} (now using global default)",
        "✓".green(),
        source.cyan()
    );

    Ok(())
}

/// Set global default policy
pub fn cmd_policy_set_default(policy_str: &str) -> Result<()> {
    let policy = parse_policy(policy_str)?;
    let mut config = HoardConfig::load()?;

    config.version_policy.default = policy.clone();
    config.save()?;

    println!(
        "{} Set global default version policy to {}",
        "✓".green(),
        format!("{}", policy).yellow()
    );

    Ok(())
}

/// Show all version policies
pub fn cmd_policy_show(db: &Database) -> Result<()> {
    let config = HoardConfig::load()?;

    println!("{}", "Version Policies".bold());
    println!();

    // Global default
    println!("{}:", "Global Default".cyan());
    println!("  {}", config.version_policy.default);
    println!();

    // Source-specific
    if !config.version_policy.sources.is_empty() {
        println!("{}:", "Source Defaults".cyan());
        for (source, policy) in &config.version_policy.sources {
            println!("  {}: {}", source, policy);
        }
        println!();
    }

    // Bundle policies
    let bundles = db.list_bundles()?;
    let bundles_with_policy: Vec<_> = bundles
        .iter()
        .filter(|b| b.version_policy.is_some())
        .collect();

    if !bundles_with_policy.is_empty() {
        println!("{}:", "Bundle Policies".cyan());
        for bundle in bundles_with_policy {
            if let Some(policy) = &bundle.version_policy {
                println!(
                    "  {}: {} ({} tools)",
                    bundle.name,
                    policy,
                    bundle.tools.len()
                );
            }
        }
        println!();
    }

    // Tool-specific policies
    let tools = db.list_tools(false, None)?;
    let tools_with_policy: Vec<_> = tools
        .iter()
        .filter(|t| t.version_policy.is_some())
        .collect();

    if !tools_with_policy.is_empty() {
        println!("{}:", "Tool Overrides".cyan());
        for tool in tools_with_policy {
            if let Some(policy) = &tool.version_policy {
                println!("  {}: {}", tool.name, policy);
            }
        }
        println!();
    }

    // Summary
    println!("{}:", "Policy Summary".cyan());
    println!("  latest - Accept any version update (major, minor, patch)");
    println!("  stable - Only accept minor and patch updates (skip major)");
    println!("  pinned - Never update, keep current version");

    Ok(())
}

/// Set version policy for a bundle
pub fn cmd_policy_set_bundle(db: &Database, name: &str, policy_str: &str) -> Result<()> {
    let policy = parse_policy(policy_str)?;

    // Check if bundle exists
    let _bundle = db
        .get_bundle(name)?
        .context(format!("Bundle '{}' not found", name))?;

    db.set_bundle_version_policy(name, Some(&policy))?;

    println!(
        "{} Set version policy for bundle {} to {}",
        "✓".green(),
        name.cyan(),
        format!("{}", policy).yellow()
    );

    Ok(())
}

/// Clear version policy for a bundle
pub fn cmd_policy_clear_bundle(db: &Database, name: &str) -> Result<()> {
    // Check if bundle exists
    let _bundle = db
        .get_bundle(name)?
        .context(format!("Bundle '{}' not found", name))?;

    db.set_bundle_version_policy(name, None)?;

    println!(
        "{} Cleared version policy for bundle {} (tools now use source/global defaults)",
        "✓".green(),
        name.cyan()
    );

    Ok(())
}

/// Parse policy string to VersionPolicy
fn parse_policy(s: &str) -> Result<VersionPolicy> {
    match s.to_lowercase().as_str() {
        "latest" => Ok(VersionPolicy::Latest),
        "stable" => Ok(VersionPolicy::Stable),
        "pinned" | "pin" => Ok(VersionPolicy::Pinned),
        _ => bail!(
            "Invalid policy '{}'. Valid options: latest, stable, pinned",
            s
        ),
    }
}
