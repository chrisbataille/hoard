//! AI bundle suggestion commands
//!
//! Commands for AI-assisted bundle creation based on usage patterns.

use anyhow::Result;
use colored::Colorize;
use std::io::IsTerminal;

use crate::Database;

/// Suggest bundles using AI based on usage patterns
pub fn cmd_ai_suggest_bundle(count: usize) -> Result<()> {
    use crate::ai::{invoke_ai, parse_bundle_response, suggest_bundle_prompt};

    let db = Database::open()?;

    // Get all tools, existing bundles, and usage data
    let tools = db.list_tools(false, None)?;
    let bundles = db.list_bundles()?;
    let all_usage = db.get_all_usage()?;

    // Convert usage to HashMap for easy lookup
    let usage_data: std::collections::HashMap<String, i64> = all_usage
        .into_iter()
        .map(|(name, usage)| (name, usage.use_count))
        .collect();

    // Count tools already in bundles
    let bundled_tools: std::collections::HashSet<&str> = bundles
        .iter()
        .flat_map(|b| b.tools.iter().map(|s| s.as_str()))
        .collect();
    let unbundled_count = tools
        .iter()
        .filter(|t| !bundled_tools.contains(t.name.as_str()))
        .count();

    if unbundled_count < 3 {
        println!(
            "{} Not enough unbundled tools to suggest bundles (need at least 3, have {})",
            "!".yellow(),
            unbundled_count
        );
        return Ok(());
    }

    // Count tools with usage data
    let tools_with_usage = tools
        .iter()
        .filter(|t| usage_data.get(&t.name).map(|&c| c > 0).unwrap_or(false))
        .count();

    println!(
        "{} Analyzing {} unbundled tools ({} with usage data)...",
        ">".cyan(),
        unbundled_count,
        tools_with_usage
    );

    if !bundles.is_empty() {
        println!(
            "  {} Excluding {} tool{} already in {} bundle{}",
            ">".dimmed(),
            bundled_tools.len(),
            if bundled_tools.len() == 1 { "" } else { "s" },
            bundles.len(),
            if bundles.len() == 1 { "" } else { "s" }
        );
    }

    // Generate prompt and call AI
    let prompt = suggest_bundle_prompt(&tools, &bundles, &usage_data, count);
    let response = invoke_ai(&prompt)?;

    // Parse response
    let suggestions = parse_bundle_response(&response)?;

    if suggestions.is_empty() {
        println!("{} AI returned no bundle suggestions", "!".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "        SUGGESTED BUNDLES               ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Display suggestions and handle interactions
    for (i, suggestion) in suggestions.iter().enumerate() {
        display_bundle_suggestion(i + 1, suggestion, &usage_data);

        // Interactive mode if terminal is available
        if std::io::stdout().is_terminal() {
            let action = prompt_bundle_action(suggestion)?;
            match action {
                BundleAction::Create => {
                    create_bundle_from_suggestion(&db, suggestion)?;
                }
                BundleAction::Install => {
                    install_bundle_tools(&db, suggestion)?;
                }
                BundleAction::CreateAndInstall => {
                    create_bundle_from_suggestion(&db, suggestion)?;
                    install_bundle_tools(&db, suggestion)?;
                }
                BundleAction::Skip => {
                    println!("  {} Skipped", "→".dimmed());
                }
            }
            println!();
        }
    }

    if !std::io::stdout().is_terminal() {
        // Non-interactive mode - just show commands
        println!(
            "{} Create a bundle with: {}",
            ">".cyan(),
            "hoards bundle create <name> -d \"description\" <tools...>".yellow()
        );
    }

    Ok(())
}

/// Display a single bundle suggestion with usage data
fn display_bundle_suggestion(
    index: usize,
    suggestion: &crate::ai::BundleSuggestion,
    usage_data: &std::collections::HashMap<String, i64>,
) {
    println!(
        "{}. {} {}",
        index,
        format!("📦 {}", suggestion.name).cyan().bold(),
        format!("- {}", suggestion.description).dimmed()
    );

    // Show reasoning if available
    if let Some(reasoning) = &suggestion.reasoning {
        println!("   {}", reasoning.dimmed().italic());
    }

    println!();

    for tool in &suggestion.tools {
        let usage = usage_data.get(tool).unwrap_or(&0);
        let usage_str = if *usage > 0 {
            format!("({}x)", usage).green().to_string()
        } else {
            "(unused)".dimmed().to_string()
        };
        println!("   • {} {}", tool, usage_str);
    }
    println!();
}

#[derive(Debug, Clone, Copy)]
enum BundleAction {
    Create,
    Install,
    CreateAndInstall,
    Skip,
}

/// Prompt user for action on a bundle suggestion
fn prompt_bundle_action(suggestion: &crate::ai::BundleSuggestion) -> Result<BundleAction> {
    use dialoguer::Select;

    let options = vec![
        format!("[c] Create bundle '{}'", suggestion.name),
        "[i] Install missing tools only".to_string(),
        "[b] Both - create bundle and install tools".to_string(),
        "[s] Skip this suggestion".to_string(),
    ];

    let selection = Select::new()
        .with_prompt("Action")
        .items(&options)
        .default(3) // Default to skip
        .interact()?;

    Ok(match selection {
        0 => BundleAction::Create,
        1 => BundleAction::Install,
        2 => BundleAction::CreateAndInstall,
        _ => BundleAction::Skip,
    })
}

/// Create a bundle from an AI suggestion
fn create_bundle_from_suggestion(
    db: &Database,
    suggestion: &crate::ai::BundleSuggestion,
) -> Result<()> {
    use crate::cmd_bundle_create;

    // Check if bundle already exists
    let existing = db.list_bundles()?;
    if existing.iter().any(|b| b.name == suggestion.name) {
        println!(
            "  {} Bundle '{}' already exists",
            "!".yellow(),
            suggestion.name
        );
        return Ok(());
    }

    cmd_bundle_create(
        db,
        &suggestion.name,
        suggestion.tools.clone(),
        Some(suggestion.description.clone()),
    )?;

    Ok(())
}

/// Install tools from a bundle suggestion that aren't already installed
fn install_bundle_tools(db: &Database, suggestion: &crate::ai::BundleSuggestion) -> Result<()> {
    let mut installed_count = 0;
    let mut skipped_count = 0;

    for tool_name in &suggestion.tools {
        // Check if tool exists in database
        if let Some(tool) = db.get_tool_by_name(tool_name)? {
            // Check if already installed (use the tool's is_installed field)
            if tool.is_installed {
                skipped_count += 1;
                continue;
            }

            // Try to install
            println!("  {} Installing {}...", ">".cyan(), tool_name);
            if let Err(e) = crate::cmd_install(db, tool_name, None, None, false) {
                println!("    {} Failed: {}", "!".yellow(), e);
            } else {
                installed_count += 1;
            }
        } else {
            println!(
                "  {} Tool '{}' not in database - add it first",
                "!".yellow(),
                tool_name
            );
        }
    }

    if installed_count > 0 || skipped_count > 0 {
        println!(
            "  {} Installed: {}, Already installed: {}",
            "+".green(),
            installed_count,
            skipped_count
        );
    }

    Ok(())
}
