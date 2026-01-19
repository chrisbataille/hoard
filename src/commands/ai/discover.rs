//! AI discovery commands
//!
//! Commands for discovering and installing new tools using AI.

use anyhow::{Context, Result};
use colored::Colorize;

use crate::Database;
use crate::commands::install::{SafeCommand, get_safe_install_command, validate_package_name};

/// Discover tools based on natural language query
pub fn cmd_ai_discover(
    db: &Database,
    query: &str,
    limit: usize,
    no_stars: bool,
    dry_run: bool,
) -> Result<()> {
    use crate::ai::{ToolRecommendation, discovery_prompt, invoke_ai, parse_discovery_response};
    use crate::scanner::is_installed;
    use dialoguer::{MultiSelect, theme::ColorfulTheme};
    use indicatif::{ProgressBar, ProgressStyle};

    println!("{} Discovering tools for: {}", ">".cyan(), query.bold());

    // Gather installed tools for context
    let installed_tools: Vec<String> = db
        .get_all_tools()?
        .iter()
        .filter(|t| t.is_installed)
        .map(|t| t.name.clone())
        .collect();

    println!(
        "{} Context: {} installed tools",
        ">".dimmed(),
        installed_tools.len()
    );

    // Get enabled sources from config for AI to recommend from
    let config = crate::config::HoardConfig::load().unwrap_or_default();
    let enabled_sources: Vec<&str> = config.sources.enabled_sources();

    println!(
        "{} Enabled sources: {}",
        ">".dimmed(),
        enabled_sources.join(", ")
    );

    // Generate prompt and call AI with spinner
    let prompt = discovery_prompt(query, &installed_tools, &enabled_sources);

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Asking AI for recommendations...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let response = invoke_ai(&prompt)?;
    spinner.finish_and_clear();

    // Parse response
    let mut discovery = parse_discovery_response(&response)?;

    // Limit results
    if discovery.tools.len() > limit {
        discovery.tools.truncate(limit);
    }

    // Check installation status and optionally fetch GitHub stars
    if !no_stars && discovery.tools.iter().any(|t| t.github.is_some()) {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));

        let total = discovery.tools.len();
        for (i, tool) in discovery.tools.iter_mut().enumerate() {
            let binary = tool.binary.as_deref().unwrap_or(&tool.name);
            tool.installed = is_installed(binary);

            if let Some(ref github) = tool.github {
                spinner.set_message(format!("Fetching GitHub stars ({}/{})...", i + 1, total));
                if let Ok(stars) = fetch_github_stars(github) {
                    tool.stars = Some(stars);
                }
            }
        }
        spinner.finish_and_clear();
    } else {
        // Just check installation status
        for tool in &mut discovery.tools {
            let binary = tool.binary.as_deref().unwrap_or(&tool.name);
            tool.installed = is_installed(binary);
        }
    }

    // Display results
    println!();
    println!("{}", discovery.summary.bold());
    println!();

    // Group by category
    let essential: Vec<_> = discovery
        .tools
        .iter()
        .filter(|t| t.category == "essential")
        .collect();
    let recommended: Vec<_> = discovery
        .tools
        .iter()
        .filter(|t| t.category != "essential")
        .collect();

    if !essential.is_empty() {
        println!("{}", "Essential:".green().bold());
        for tool in &essential {
            print_tool_recommendation(tool);
        }
        println!();
    }

    if !recommended.is_empty() {
        println!("{}", "Recommended:".blue().bold());
        for tool in &recommended {
            print_tool_recommendation(tool);
        }
        println!();
    }

    // Filter to tools that can be installed
    let installable: Vec<&ToolRecommendation> =
        discovery.tools.iter().filter(|t| !t.installed).collect();

    if installable.is_empty() {
        println!(
            "{} All recommended tools are already installed!",
            "+".green()
        );
        return Ok(());
    }

    // In dry-run mode, show what could be installed but don't prompt
    if dry_run {
        println!("{}", "Available for installation:".bold());
        for (i, tool) in installable.iter().enumerate() {
            let stars = tool
                .stars
                .map(|s| format!(" ({}★)", format_stars(s)))
                .unwrap_or_default();
            println!(
                "  {}. {} - {}{}",
                i + 1,
                tool.name.cyan(),
                tool.description,
                stars
            );
            if let Some(ref github) = tool.github {
                println!("      GitHub: https://github.com/{}", github);
            }
            println!("      Install: {}", tool.install_cmd.dimmed());
        }
        println!();
        println!(
            "{} Run without {} to install interactively",
            ">".cyan(),
            "--dry-run".yellow()
        );
        return Ok(());
    }

    // Interactive selection
    let options: Vec<String> = installable
        .iter()
        .map(|t| {
            let stars = t
                .stars
                .map(|s| format!(" ({}★)", format_stars(s)))
                .unwrap_or_default();
            format!("{} - {}{}", t.name, t.description, stars)
        })
        .collect();

    println!("{}", "Select tools to install:".bold());
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .items(&options)
        .interact_opt()?;

    if let Some(indices) = selections {
        if indices.is_empty() {
            println!("{} No tools selected", ">".dimmed());
            return Ok(());
        }

        println!();
        for idx in indices {
            let tool = installable[idx];
            install_discovered_tool(db, tool)?;
        }
    }

    Ok(())
}

/// Install a tool discovered via AI, using proper extraction when possible
fn install_discovered_tool(db: &Database, tool: &crate::ai::ToolRecommendation) -> Result<()> {
    use crate::ai::{
        ExtractedTool, extract_prompt, fetch_readme, fetch_repo_version, invoke_ai,
        parse_extract_response, parse_github_url,
    };
    use crate::db::CachedExtraction;
    use crate::models::{InstallSource, Tool};
    use indicatif::{ProgressBar, ProgressStyle};

    use super::cheatsheet::invalidate_cheatsheet_cache;

    println!("{} Installing {}...", ">".cyan(), tool.name.bold());

    // If tool has a GitHub URL, try to extract proper info first
    let extracted = if let Some(ref github) = tool.github {
        let github_url = format!("https://github.com/{}", github);
        match parse_github_url(&github_url) {
            Ok((owner, repo)) => {
                // Check cache first
                let version = fetch_repo_version(&owner, &repo).unwrap_or_default();

                if let Ok(Some(cached)) = db.get_cached_extraction(&owner, &repo, &version) {
                    println!("  {} Using cached extraction", "+".green());
                    Some(ExtractedTool {
                        name: cached.name,
                        binary: cached.binary,
                        source: cached.source,
                        install_command: cached.install_command,
                        description: cached.description,
                        category: cached.category,
                    })
                } else {
                    // Try to extract from README with spinner
                    let spinner = ProgressBar::new_spinner();
                    spinner.set_style(
                        ProgressStyle::default_spinner()
                            .template("  {spinner:.cyan} {msg}")
                            .unwrap(),
                    );
                    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
                    spinner.set_message("Fetching README from GitHub...");

                    match fetch_readme(&owner, &repo) {
                        Ok(readme) => {
                            spinner.set_message("Extracting tool info with AI...");
                            let prompt = extract_prompt(&readme);
                            match invoke_ai(&prompt).and_then(|r| parse_extract_response(&r)) {
                                Ok(ext) => {
                                    spinner.finish_and_clear();
                                    // Cache it
                                    let cached = CachedExtraction {
                                        repo_owner: owner.clone(),
                                        repo_name: repo.clone(),
                                        version: version.clone(),
                                        name: ext.name.clone(),
                                        binary: ext.binary.clone(),
                                        source: ext.source.clone(),
                                        install_command: ext.install_command.clone(),
                                        description: ext.description.clone(),
                                        category: ext.category.clone(),
                                        extracted_at: chrono::Utc::now().to_rfc3339(),
                                    };
                                    let _ = db.cache_extraction(&cached);
                                    println!("  {} Extracted install info", "+".green());
                                    Some(ext)
                                }
                                Err(e) => {
                                    spinner.finish_and_clear();
                                    println!("  {} Extraction failed: {}", "!".yellow(), e);
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            spinner.finish_and_clear();
                            println!("  {} Could not fetch README: {}", "!".yellow(), e);
                            None
                        }
                    }
                }
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Determine install details
    let (name, source, install_cmd, description, category, binary) =
        if let Some(ref ext) = extracted {
            (
                ext.name.clone(),
                ext.source.clone(),
                ext.install_command.clone(),
                ext.description.clone(),
                ext.category.clone(),
                ext.binary.clone(),
            )
        } else {
            (
                tool.name.clone(),
                tool.source.clone(),
                Some(tool.install_cmd.clone()),
                tool.description.clone(),
                tool.category.clone(),
                tool.binary.clone(),
            )
        };

    // Try to use safe install command if we have a known source
    let final_cmd = if let Some(safe_cmd) = get_safe_install_command(&name, &source, None)? {
        println!("  {} Using: {}", ">".dimmed(), safe_cmd);
        Some(safe_cmd)
    } else if let Some(ref cmd) = install_cmd {
        println!("  {} Using: {}", ">".dimmed(), cmd);
        None // Will use shell command
    } else {
        println!("  {} No install command available", "!".red());
        return Ok(());
    };

    // Execute installation with spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("  {spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Installing {}...", name));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let success = if let Some(safe_cmd) = final_cmd {
        match safe_cmd.execute() {
            Ok(status) => status.success(),
            Err(e) => {
                spinner.finish_and_clear();
                println!("  {} Install error: {}", "!".red(), e);
                false
            }
        }
    } else if let Some(ref cmd) = install_cmd {
        // Try to parse the install command and construct a SafeCommand
        // This is safer than executing arbitrary shell commands
        let parsed_cmd = parse_install_cmd_to_safe_command(cmd);
        if let Some(safe_cmd) = parsed_cmd {
            match safe_cmd.execute() {
                Ok(status) => status.success(),
                Err(e) => {
                    spinner.finish_and_clear();
                    println!("  {} Install error: {}", "!".red(), e);
                    false
                }
            }
        } else {
            // Cannot safely execute this command - inform user
            spinner.finish_and_clear();
            println!(
                "  {} Cannot auto-install: unrecognized command format",
                "!".yellow()
            );
            println!("  {} Manual install required: {}", ">".dimmed(), cmd);
            false
        }
    } else {
        false
    };

    spinner.finish_and_clear();

    if success {
        println!("  {} Installed {}", "+".green(), name);

        // Add to database with full metadata
        if db.get_tool_by_name(&name)?.is_none() {
            let mut new_tool = Tool::new(&name)
                .with_description(&description)
                .with_source(InstallSource::from(source.as_str()))
                .with_category(&category)
                .installed();

            if let Some(ref bin) = binary {
                new_tool = new_tool.with_binary(bin);
            }
            if let Some(ref cmd) = install_cmd {
                new_tool = new_tool.with_install_command(cmd);
            }

            db.insert_tool(&new_tool)?;
            println!("  {} Added to database", "+".green());
        } else {
            db.set_tool_installed(&name, true)?;
        }

        // Invalidate any cached cheatsheet
        let _ = invalidate_cheatsheet_cache(db, &name);
    } else {
        println!("  {} Failed to install {}", "!".red(), name);
    }

    Ok(())
}

/// Print a single tool recommendation
fn print_tool_recommendation(tool: &crate::ai::ToolRecommendation) {
    let status = if tool.installed {
        "✓".green().to_string()
    } else {
        " ".to_string()
    };

    let stars = tool
        .stars
        .map(|s| format!(" {}★", format_stars(s)).dimmed().to_string())
        .unwrap_or_default();

    println!(
        "  {} {:<15} {}{}",
        status,
        tool.name.cyan(),
        tool.description,
        stars
    );
    println!("      {} {}", "→".dimmed(), tool.reason.dimmed());
}

/// Format star count (e.g., 12345 -> "12.3K")
pub fn format_stars(stars: u64) -> String {
    if stars >= 1000 {
        format!("{:.1}K", stars as f64 / 1000.0)
    } else {
        stars.to_string()
    }
}

/// Fetch GitHub stars for a repo
fn fetch_github_stars(repo: &str) -> Result<u64> {
    // Use the GitHub API
    let url = format!("https://api.github.com/repos/{}", repo);

    let mut response = ureq::get(&url)
        .header("User-Agent", "hoards-cli")
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .context("Failed to fetch GitHub info")?;

    let body = response.body_mut().read_to_string()?;
    let json: serde_json::Value = serde_json::from_str(&body)?;
    let stars = json["stargazers_count"].as_u64().unwrap_or(0);

    Ok(stars)
}

/// Parse an install command string into a SafeCommand
/// Returns None if the command cannot be safely parsed
pub fn parse_install_cmd_to_safe_command(cmd: &str) -> Option<SafeCommand> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // Validate that we're dealing with a known package manager
    match parts.as_slice() {
        // cargo install <package>
        ["cargo", "install", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "cargo",
                args: vec!["install".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        // pip install <package>
        ["pip", "install", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "pip",
                args: vec!["install".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        ["pip3", "install", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "pip3",
                args: vec!["install".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        // pip install --upgrade <package>
        ["pip", "install", "--upgrade", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "pip",
                args: vec!["install".into(), "--upgrade".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        ["pip3", "install", "--upgrade", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "pip3",
                args: vec!["install".into(), "--upgrade".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        // npm install -g <package>
        ["npm", "install", "-g", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "npm",
                args: vec!["install".into(), "-g".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        // brew install <package>
        ["brew", "install", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "brew",
                args: vec!["install".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        // sudo apt install -y <package>
        ["sudo", "apt", "install", "-y", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "sudo",
                args: vec![
                    "apt".into(),
                    "install".into(),
                    "-y".into(),
                    (*package).into(),
                ],
                display: cmd.into(),
            })
        }
        // sudo snap install <package>
        ["sudo", "snap", "install", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "sudo",
                args: vec!["snap".into(), "install".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        // flatpak install -y <package>
        ["flatpak", "install", "-y", package] => {
            validate_package_name(package).ok()?;
            Some(SafeCommand {
                program: "flatpak",
                args: vec!["install".into(), "-y".into(), (*package).into()],
                display: cmd.into(),
            })
        }
        // Unrecognized command pattern
        _ => None,
    }
}
