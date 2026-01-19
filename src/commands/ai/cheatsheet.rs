//! AI cheatsheet commands
//!
//! Commands for generating tool and bundle cheatsheets using AI.

use anyhow::Result;
use colored::Colorize;

use crate::Database;

/// Generate a cheatsheet for a tool using AI
pub fn cmd_ai_cheatsheet(tool_name: &str, refresh: bool) -> Result<()> {
    use crate::ai::{
        cheatsheet_prompt, format_cheatsheet, get_help_output, invoke_ai, parse_cheatsheet_response,
    };

    let db = Database::open()?;

    // Get the tool from database to find binary name
    let tool = db
        .get_tool_by_name(tool_name)?
        .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found in database", tool_name))?;

    let binary = tool.binary_name.as_deref().unwrap_or(&tool.name);

    // Check cache first (unless refresh requested)
    // Version checking happens inside get_cached_cheatsheet
    if !refresh && let Some(cached) = get_cached_cheatsheet(&db, tool_name, binary)? {
        println!("{}", format_cheatsheet(&cached));
        println!();
        println!(
            "{} Cached cheatsheet. Use {} to regenerate.",
            ">".dimmed(),
            "--refresh".yellow()
        );
        return Ok(());
    }

    println!(
        "{} Generating cheatsheet for {}...",
        ">".cyan(),
        tool_name.bold()
    );

    // Get --help output
    let help_output = get_help_output(binary).map_err(|e| {
        anyhow::anyhow!(
            "Could not get help for '{}': {}. Is it installed?",
            binary,
            e
        )
    })?;

    // Generate prompt and call AI
    let prompt = cheatsheet_prompt(tool_name, &help_output);
    let response = invoke_ai(&prompt)?;

    // Parse response
    let cheatsheet = parse_cheatsheet_response(&response)?;

    // Cache the result with version info
    cache_cheatsheet(&db, tool_name, binary, &cheatsheet)?;

    // Display
    println!();
    println!("{}", format_cheatsheet(&cheatsheet));

    Ok(())
}

/// Get cached cheatsheet from database, checking version for invalidation
fn get_cached_cheatsheet(
    db: &Database,
    tool_name: &str,
    binary: &str,
) -> Result<Option<crate::ai::Cheatsheet>> {
    use crate::ai::{CachedCheatsheet, get_tool_version};

    let cache_key = format!("cheatsheet:{}", tool_name);

    match db.get_ai_cache(&cache_key)? {
        Some(json) => {
            // Try to parse as CachedCheatsheet (new format with version)
            if let Ok(cached) = serde_json::from_str::<CachedCheatsheet>(&json) {
                // Check if version matches
                let current_version = get_tool_version(binary);
                if cached.version == current_version {
                    return Ok(Some(cached.cheatsheet));
                }
                // Version changed, invalidate cache
                return Ok(None);
            }

            // Fallback: try to parse as plain Cheatsheet (old format)
            // This will be re-cached with version on next generation
            if let Ok(cheatsheet) = serde_json::from_str::<crate::ai::Cheatsheet>(&json) {
                return Ok(Some(cheatsheet));
            }

            Ok(None)
        }
        None => Ok(None),
    }
}

/// Cache a cheatsheet in the database with version info
fn cache_cheatsheet(
    db: &Database,
    tool_name: &str,
    binary: &str,
    cheatsheet: &crate::ai::Cheatsheet,
) -> Result<()> {
    use crate::ai::{CachedCheatsheet, get_tool_version};

    let cache_key = format!("cheatsheet:{}", tool_name);
    let cached = CachedCheatsheet {
        version: get_tool_version(binary),
        cheatsheet: cheatsheet.clone(),
    };
    let json = serde_json::to_string(&cached)?;
    db.set_ai_cache(&cache_key, &json)?;
    Ok(())
}

/// Invalidate cached cheatsheet for a tool (call after install/upgrade)
pub fn invalidate_cheatsheet_cache(db: &Database, tool_name: &str) -> Result<()> {
    let cache_key = format!("cheatsheet:{}", tool_name);
    db.delete_ai_cache(&cache_key)?;
    Ok(())
}

/// Generate a workflow-oriented cheatsheet for all tools in a bundle
pub fn cmd_ai_bundle_cheatsheet(bundle_name: &str, refresh: bool) -> Result<()> {
    use crate::ai::{
        bundle_cheatsheet_prompt, format_cheatsheet, get_help_output, get_tool_version, invoke_ai,
        parse_cheatsheet_response,
    };

    let db = Database::open()?;

    // Get the bundle
    let bundle = db
        .get_bundle(bundle_name)?
        .ok_or_else(|| anyhow::anyhow!("Bundle '{}' not found", bundle_name))?;

    if bundle.tools.is_empty() {
        println!("Bundle '{}' has no tools", bundle_name);
        return Ok(());
    }

    // Collect tool info and versions for cache key
    let mut tools_info: Vec<(String, String, Option<String>)> = Vec::new(); // (name, binary, version)
    for tool_name in &bundle.tools {
        if let Some(tool) = db.get_tool_by_name(tool_name)? {
            let binary = tool
                .binary_name
                .as_deref()
                .unwrap_or(&tool.name)
                .to_string();
            let version = get_tool_version(&binary);
            tools_info.push((tool_name.clone(), binary, version));
        }
    }

    if tools_info.is_empty() {
        println!("No tools from bundle '{}' found in database", bundle_name);
        return Ok(());
    }

    // Check cache (unless refresh requested)
    // Cache key includes bundle name and all tool versions
    if !refresh && let Some(cached) = get_cached_bundle_cheatsheet(&db, bundle_name, &tools_info)? {
        println!("{}", format_cheatsheet(&cached));
        println!();
        println!(
            "{} Cached bundle cheatsheet ({} tools). Use {} to regenerate.",
            ">".dimmed(),
            tools_info.len(),
            "--refresh".yellow()
        );
        return Ok(());
    }

    println!(
        "{} Generating workflow cheatsheet for bundle '{}' ({} tools)...",
        ">".cyan(),
        bundle_name.bold(),
        tools_info.len()
    );

    // Collect help outputs for all tools
    let mut tools_help: Vec<(String, String)> = Vec::new();
    for (name, binary, _) in &tools_info {
        match get_help_output(binary) {
            Ok(help) => {
                println!("  {} {}", "+".green(), name);
                tools_help.push((name.clone(), help));
            }
            Err(e) => {
                println!("  {} {} (skipped: {})", "!".yellow(), name, e);
            }
        }
    }

    if tools_help.is_empty() {
        return Err(anyhow::anyhow!(
            "Could not get help for any tools in bundle"
        ));
    }

    // Generate prompt and call AI
    let prompt = bundle_cheatsheet_prompt(bundle_name, &tools_help);
    let response = invoke_ai(&prompt)?;

    // Parse response
    let cheatsheet = parse_cheatsheet_response(&response)?;

    // Cache the result with version info
    cache_bundle_cheatsheet(&db, bundle_name, &tools_info, &cheatsheet)?;

    // Display
    println!();
    println!("{}", format_cheatsheet(&cheatsheet));

    Ok(())
}

/// Get cached bundle cheatsheet, checking all tool versions
fn get_cached_bundle_cheatsheet(
    db: &Database,
    bundle_name: &str,
    tools_info: &[(String, String, Option<String>)],
) -> Result<Option<crate::ai::Cheatsheet>> {
    let cache_key = format!("cheatsheet:bundle:{}", bundle_name);

    match db.get_ai_cache(&cache_key)? {
        Some(json) => {
            // Parse as CachedBundleCheatsheet which includes version map
            if let Ok(cached) = serde_json::from_str::<CachedBundleCheatsheet>(&json) {
                // Check same number of tools (bundle might have changed)
                if cached.versions.len() != tools_info.len() {
                    return Ok(None);
                }

                // Check all versions match (empty string = no version)
                let versions_match = tools_info.iter().all(|(name, _, current_ver)| {
                    let cached_ver = cached.versions.get(name);
                    let current = current_ver.as_deref().unwrap_or("");
                    cached_ver.map(|s| s.as_str()) == Some(current)
                });

                if versions_match {
                    return Ok(Some(cached.cheatsheet));
                }
            }
            Ok(None)
        }
        None => Ok(None),
    }
}

/// Cache a bundle cheatsheet with all tool versions
fn cache_bundle_cheatsheet(
    db: &Database,
    bundle_name: &str,
    tools_info: &[(String, String, Option<String>)],
    cheatsheet: &crate::ai::Cheatsheet,
) -> Result<()> {
    let cache_key = format!("cheatsheet:bundle:{}", bundle_name);

    // Store all tools - use empty string for tools without version info
    let versions: std::collections::HashMap<String, String> = tools_info
        .iter()
        .map(|(name, _, version)| (name.clone(), version.clone().unwrap_or_default()))
        .collect();

    let cached = CachedBundleCheatsheet {
        versions,
        cheatsheet: cheatsheet.clone(),
    };

    let json = serde_json::to_string(&cached)?;
    db.set_ai_cache(&cache_key, &json)?;
    Ok(())
}

/// Cached bundle cheatsheet with version info for all tools
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CachedBundleCheatsheet {
    versions: std::collections::HashMap<String, String>,
    cheatsheet: crate::ai::Cheatsheet,
}
