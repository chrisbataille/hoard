//! AI enrichment commands
//!
//! Commands for AI-assisted categorization and description generation.

use anyhow::Result;
use colored::Colorize;

use crate::Database;

/// Categorize tools using AI
pub fn cmd_ai_categorize(dry_run: bool) -> Result<()> {
    use crate::ai::{categorize_prompt, invoke_ai, parse_categorize_response};

    let db = Database::open()?;

    // Get tools without categories
    let all_tools = db.list_tools(false, None)?;
    let uncategorized: Vec<_> = all_tools
        .iter()
        .filter(|t| t.category.is_none())
        .cloned()
        .collect();

    if uncategorized.is_empty() {
        println!("{} All tools are already categorized", "+".green());
        return Ok(());
    }

    println!(
        "{} Found {} uncategorized tool{}",
        ">".cyan(),
        uncategorized.len(),
        if uncategorized.len() == 1 { "" } else { "s" }
    );

    // Get existing categories
    let categories: Vec<String> = all_tools
        .iter()
        .filter_map(|t| t.category.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Generate prompt and call AI
    let prompt = categorize_prompt(&uncategorized, &categories);

    println!("{} Asking AI to categorize...", ">".cyan());
    let response = invoke_ai(&prompt)?;

    // Parse response
    let categorizations = parse_categorize_response(&response)?;

    if categorizations.is_empty() {
        println!("{} AI returned no categorizations", "!".yellow());
        return Ok(());
    }

    // Apply or show results
    println!();
    for (tool_name, category) in &categorizations {
        if dry_run {
            println!(
                "  {} {} -> {}",
                "[dry]".yellow(),
                tool_name,
                category.cyan()
            );
        } else if let Err(e) = db.update_tool_category(tool_name, category) {
            println!("  {} {} : {}", "!".red(), tool_name, e);
        } else {
            println!("  {} {} -> {}", "+".green(), tool_name, category.cyan());
        }
    }

    if dry_run {
        println!();
        println!(
            "{} Run without {} to apply changes",
            ">".cyan(),
            "--dry-run".yellow()
        );
    } else {
        println!();
        println!(
            "{} Categorized {} tool{}",
            "+".green(),
            categorizations.len(),
            if categorizations.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

/// Generate descriptions for tools using AI
pub fn cmd_ai_describe(dry_run: bool, limit: Option<usize>) -> Result<()> {
    use crate::ai::{describe_prompt, invoke_ai, parse_describe_response};

    let db = Database::open()?;

    // Get tools without descriptions
    let all_tools = db.list_tools(false, None)?;
    let mut no_description: Vec<_> = all_tools
        .iter()
        .filter(|t| {
            t.description.is_none()
                || t.description
                    .as_ref()
                    .map(|d| d.is_empty())
                    .unwrap_or(false)
        })
        .cloned()
        .collect();

    if no_description.is_empty() {
        println!("{} All tools already have descriptions", "+".green());
        return Ok(());
    }

    // Apply limit if specified
    if let Some(max) = limit {
        no_description.truncate(max);
    }

    println!(
        "{} Found {} tool{} without descriptions",
        ">".cyan(),
        no_description.len(),
        if no_description.len() == 1 { "" } else { "s" }
    );

    // Generate prompt and call AI
    let prompt = describe_prompt(&no_description);

    println!("{} Asking AI to generate descriptions...", ">".cyan());
    let response = invoke_ai(&prompt)?;

    // Parse response
    let descriptions = parse_describe_response(&response)?;

    if descriptions.is_empty() {
        println!("{} AI returned no descriptions", "!".yellow());
        return Ok(());
    }

    // Apply or show results
    println!();
    for (tool_name, description) in &descriptions {
        if dry_run {
            println!("  {} {}", "[dry]".yellow(), tool_name.cyan());
            println!("       {}", description.dimmed());
        } else if let Err(e) = db.update_tool_description(tool_name, description) {
            println!("  {} {} : {}", "!".red(), tool_name, e);
        } else {
            println!("  {} {}", "+".green(), tool_name.cyan());
            println!("       {}", description.dimmed());
        }
    }

    if dry_run {
        println!();
        println!(
            "{} Run without {} to apply changes",
            ">".cyan(),
            "--dry-run".yellow()
        );
    } else {
        println!();
        println!(
            "{} Added descriptions for {} tool{}",
            "+".green(),
            descriptions.len(),
            if descriptions.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}
