//! AI extraction commands
//!
//! Commands for extracting tool info from GitHub READMEs using AI.

use anyhow::Result;
use colored::Colorize;

use crate::Database;

/// Extract tool info from GitHub README using AI
pub fn cmd_ai_extract(
    db: &Database,
    urls: Vec<String>,
    yes: bool,
    dry_run: bool,
    delay_ms: u64,
) -> Result<()> {
    use crate::ai::{
        ExtractedTool, extract_prompt, fetch_readme, fetch_repo_version, invoke_ai,
        parse_extract_response, parse_github_url,
    };
    use crate::db::CachedExtraction;
    use crate::{InstallSource, Tool};
    use dialoguer::Confirm;
    use std::thread;
    use std::time::Duration;

    if urls.is_empty() {
        println!("{} No URLs provided", "!".yellow());
        return Ok(());
    }

    println!(
        "{} Extracting tool info from {} URL{}...",
        ">".cyan(),
        urls.len(),
        if urls.len() == 1 { "" } else { "s" }
    );
    println!();

    let mut extracted: Vec<(String, String, ExtractedTool)> = Vec::new();
    let mut errors: Vec<(String, String)> = Vec::new();

    for (i, url) in urls.iter().enumerate() {
        // Rate limiting for batch mode
        if i > 0 && delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }

        // Parse URL
        let (owner, repo) = match parse_github_url(url) {
            Ok(parsed) => parsed,
            Err(e) => {
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        println!("{} {}/{}", ">".cyan(), owner, repo);

        // Check cache first
        let version = match fetch_repo_version(&owner, &repo) {
            Ok(v) => v,
            Err(e) => {
                println!("  {} Failed to get version: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        if let Ok(Some(cached)) = db.get_cached_extraction(&owner, &repo, &version) {
            println!("  {} Using cached extraction", "+".green());
            let tool = ExtractedTool {
                name: cached.name,
                binary: cached.binary,
                source: cached.source,
                install_command: cached.install_command,
                description: cached.description,
                category: cached.category,
            };
            extracted.push((owner, repo, tool));
            continue;
        }

        // Fetch README
        let readme = match fetch_readme(&owner, &repo) {
            Ok(r) => r,
            Err(e) => {
                println!("  {} Failed to fetch README: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        // Extract using AI
        let prompt = extract_prompt(&readme);
        println!("  {} Asking AI to extract...", ">".dimmed());

        let response = match invoke_ai(&prompt) {
            Ok(r) => r,
            Err(e) => {
                println!("  {} AI extraction failed: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        let tool = match parse_extract_response(&response) {
            Ok(t) => t,
            Err(e) => {
                println!("  {} Failed to parse response: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        // Cache the result
        let cached = CachedExtraction {
            repo_owner: owner.clone(),
            repo_name: repo.clone(),
            version: version.clone(),
            name: tool.name.clone(),
            binary: tool.binary.clone(),
            source: tool.source.clone(),
            install_command: tool.install_command.clone(),
            description: tool.description.clone(),
            category: tool.category.clone(),
            extracted_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = db.cache_extraction(&cached) {
            println!("  {} Cache write failed: {}", "!".yellow(), e);
        }

        println!("  {} Extracted successfully", "+".green());
        extracted.push((owner, repo, tool));
    }

    // Show results
    if !extracted.is_empty() {
        println!();
        println!("{}", "Extracted Tools:".bold());
        println!("{}", "=".repeat(50));

        for (owner, repo, tool) in &extracted {
            println!();
            println!("{} (from {}/{})", tool.name.cyan().bold(), owner, repo);
            if let Some(bin) = &tool.binary {
                println!("  Binary:      {}", bin);
            }
            println!("  Source:      {}", tool.source);
            if let Some(cmd) = &tool.install_command {
                println!("  Install:     {}", cmd);
            }
            println!("  Category:    {}", tool.category);
            println!("  Description: {}", tool.description.dimmed());
        }
    }

    // Handle errors
    if !errors.is_empty() {
        println!();
        println!("{}", "Errors:".red().bold());
        for (url, err) in &errors {
            println!("  {} {}: {}", "!".red(), url, err);
        }
    }

    // Add to database
    if !extracted.is_empty() && !dry_run {
        println!();

        let should_add = if yes {
            true
        } else {
            Confirm::new()
                .with_prompt(format!(
                    "Add {} tool{} to database?",
                    extracted.len(),
                    if extracted.len() == 1 { "" } else { "s" }
                ))
                .default(true)
                .interact()?
        };

        if should_add {
            let mut added = 0;
            for (_owner, _repo, ext) in &extracted {
                // Check if tool already exists
                if db.get_tool_by_name(&ext.name)?.is_some() {
                    println!("  {} {} already exists, skipping", "!".yellow(), ext.name);
                    continue;
                }

                let source = InstallSource::from(ext.source.as_str());
                let tool = Tool::new(&ext.name)
                    .with_source(source)
                    .with_description(&ext.description)
                    .with_category(&ext.category)
                    .with_binary(ext.binary.as_deref().unwrap_or(&ext.name))
                    .with_install_command(ext.install_command.as_deref().unwrap_or(""));

                if let Err(e) = db.insert_tool(&tool) {
                    println!("  {} Failed to add {}: {}", "!".red(), ext.name, e);
                } else {
                    println!("  {} Added {}", "+".green(), ext.name);
                    added += 1;
                }
            }

            println!();
            println!(
                "{} Added {} tool{} to database",
                "+".green(),
                added,
                if added == 1 { "" } else { "s" }
            );
        }
    } else if dry_run && !extracted.is_empty() {
        println!();
        println!(
            "{} Run without {} to add to database",
            ">".cyan(),
            "--dry-run".yellow()
        );
    }

    Ok(())
}
