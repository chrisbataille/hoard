//! AI migration commands
//!
//! Commands for migrating tools between package sources.

use anyhow::Result;
use colored::Colorize;
use std::io::IsTerminal;
use std::process::Command;

use crate::commands::install::{get_safe_install_command, get_safe_uninstall_command};
use crate::{AiProvider, Database, HoardConfig};

/// Migrate tools between package sources
///
/// Find tools that have newer versions on other package sources and offer to migrate them.
pub fn cmd_ai_migrate(
    db: &Database,
    from: Option<String>,
    to: Option<String>,
    dry_run: bool,
    json_output: bool,
    no_ai: bool,
) -> Result<()> {
    use crate::ai::{
        MigrationCandidate, MigrationResult, invoke_ai, migrate_prompt, parse_migrate_response,
    };
    use crate::updates::{get_installed_version, get_migration_candidates};
    use dialoguer::{MultiSelect, Select, theme::ColorfulTheme};
    use indicatif::{ProgressBar, ProgressStyle};
    use std::time::Duration;

    // 1. Gather installed tools with versions
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Gathering tool versions...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    // Get only installed tools
    let tools = db.list_tools(true, None)?;
    let mut tools_with_versions: Vec<(String, String, String)> = Vec::new();

    for tool in &tools {
        let source = tool.source.to_string().to_lowercase();
        if let Some(version) = get_installed_version(&tool.name, &source) {
            tools_with_versions.push((tool.name.clone(), version, source));
        }
    }

    spinner.finish_and_clear();

    if tools_with_versions.is_empty() {
        println!("{} No tools with version information found.", "!".yellow());
        return Ok(());
    }

    // 2. Find migration candidates
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Checking for migration candidates...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let upgrades = get_migration_candidates(&tools_with_versions, from.as_deref(), to.as_deref());

    spinner.finish_and_clear();

    if upgrades.is_empty() {
        println!("{} No migration candidates found.", "!".yellow());
        if from.is_some() || to.is_some() {
            println!("  Try without --from/--to filters to see all possibilities.");
        }
        return Ok(());
    }

    // 3. Build MigrationCandidate list
    let mut candidates: Vec<MigrationCandidate> = upgrades
        .iter()
        .map(|u| MigrationCandidate {
            name: u.name.clone(),
            from_source: u.current_source.clone(),
            from_version: u.current_version.clone(),
            to_source: u.better_source.clone(),
            to_version: u.better_version.clone(),
            to_package_name: get_target_package_name(&u.name, &u.better_source),
            benefit: None,
        })
        .collect();

    // 4. Optional: Get AI benefits
    let ai_summary: Option<String> = if !no_ai {
        let config = HoardConfig::load()?;
        if config.ai.provider != AiProvider::None && config.ai.provider.is_installed() {
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            spinner.set_message("Getting AI insights...");
            spinner.enable_steady_tick(Duration::from_millis(80));

            let tools_for_prompt: Vec<(String, String, String, String, String)> = candidates
                .iter()
                .map(|c| {
                    (
                        c.name.clone(),
                        c.from_source.clone(),
                        c.from_version.clone(),
                        c.to_source.clone(),
                        c.to_version.clone(),
                    )
                })
                .collect();

            let prompt = migrate_prompt(&tools_for_prompt);
            match invoke_ai(&prompt) {
                Ok(response) => {
                    spinner.finish_and_clear();
                    if let Ok(benefits) = parse_migrate_response(&response) {
                        // Apply benefits to candidates
                        for candidate in &mut candidates {
                            if let Some(benefit) = benefits.get(&candidate.name) {
                                candidate.benefit = Some(benefit.clone());
                            }
                        }
                    }
                    Some("AI-generated benefits included".to_string())
                }
                Err(_) => {
                    spinner.finish_and_clear();
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // 5. Build result
    let result = MigrationResult {
        candidates: candidates.clone(),
        ai_summary,
    };

    // 6. Output
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Display formatted output
    display_migration_table(&result);

    if dry_run {
        println!();
        println!("{} Dry run - no changes made", "!".yellow());
        print_migration_commands(&result);
        return Ok(());
    }

    // 7. Interactive selection (only in TTY)
    if !std::io::stdout().is_terminal() {
        print_migration_commands(&result);
        return Ok(());
    }

    if result.candidates.is_empty() {
        return Ok(());
    }

    // Prompt for action
    let options = vec![
        "[m] Migrate all tools".to_string(),
        "[s] Select tools to migrate".to_string(),
        "[c] Cancel".to_string(),
    ];

    let selection = Select::new()
        .with_prompt("Action")
        .items(&options)
        .default(2)
        .interact()?;

    match selection {
        0 => execute_migration(db, &result.candidates)?,
        1 => {
            let labels: Vec<String> = result
                .candidates
                .iter()
                .map(|c| {
                    format!(
                        "{} ({} {} → {} {})",
                        c.name, c.from_source, c.from_version, c.to_source, c.to_version
                    )
                })
                .collect();

            let selected = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select tools to migrate")
                .items(&labels)
                .interact_opt()?;

            if let Some(indices) = selected {
                if indices.is_empty() {
                    println!("No tools selected.");
                } else {
                    let selected_candidates: Vec<MigrationCandidate> = indices
                        .iter()
                        .map(|&i| result.candidates[i].clone())
                        .collect();
                    execute_migration(db, &selected_candidates)?;
                }
            }
        }
        _ => println!("Migration cancelled."),
    }

    Ok(())
}

/// Get the package name for a tool on the target source
/// (may differ from the tool name, e.g., "fd" installs as "fd-find" on cargo)
fn get_target_package_name(tool_name: &str, target_source: &str) -> String {
    match target_source {
        "cargo" => match tool_name {
            "fd" | "fd-find" => "fd-find".to_string(),
            "dust" => "du-dust".to_string(),
            "delta" | "git-delta" => "git-delta".to_string(),
            "tldr" | "tealdeer" => "tealdeer".to_string(),
            _ => tool_name.to_string(),
        },
        "pip" => match tool_name {
            "yt-dlp" => "yt-dlp".to_string(),
            _ => tool_name.to_string(),
        },
        _ => tool_name.to_string(),
    }
}

/// Display migration candidates in a table
fn display_migration_table(result: &crate::ai::MigrationResult) {
    use comfy_table::{Table, presets::UTF8_BORDERS_ONLY};

    println!();
    println!("🔄 Migration Analysis");
    println!();

    if result.candidates.is_empty() {
        println!("  No migration candidates found.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);
    table.set_header(vec!["Tool", "From", "Version", "To", "Version", "Benefit"]);

    for c in &result.candidates {
        table.add_row(vec![
            &c.name,
            &c.from_source,
            &c.from_version,
            &c.to_source,
            &c.to_version,
            c.benefit.as_deref().unwrap_or("-"),
        ]);
    }

    println!("{}", table);
    println!();
    println!(
        "{} {} tool{} can be migrated to newer versions",
        ">".cyan(),
        result.candidates.len(),
        if result.candidates.len() == 1 {
            ""
        } else {
            "s"
        }
    );
}

/// Print migration commands for non-interactive use
fn print_migration_commands(result: &crate::ai::MigrationResult) {
    if result.candidates.is_empty() {
        return;
    }

    // Group by source pair
    let mut by_source: std::collections::HashMap<
        (String, String),
        Vec<&crate::ai::MigrationCandidate>,
    > = std::collections::HashMap::new();

    for c in &result.candidates {
        by_source
            .entry((c.from_source.clone(), c.to_source.clone()))
            .or_default()
            .push(c);
    }

    println!();
    println!("{}", "Migration commands:".bold());
    println!();

    for ((from, to), tools) in &by_source {
        let package_names: Vec<&str> = tools.iter().map(|t| t.to_package_name.as_str()).collect();

        // Install command
        let install_cmd = match to.as_str() {
            "cargo" => format!("cargo install {}", package_names.join(" ")),
            "pip" => format!("pip install {}", package_names.join(" ")),
            "npm" => format!("npm install -g {}", package_names.join(" ")),
            _ => format!("# Install from {}: {}", to, package_names.join(" ")),
        };

        // Uninstall command
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        let uninstall_cmd = match from.as_str() {
            "apt" => format!("sudo apt remove {}", tool_names.join(" ")),
            "snap" => format!("sudo snap remove {}", tool_names.join(" ")),
            "cargo" => format!("cargo uninstall {}", tool_names.join(" ")),
            "pip" => format!("pip uninstall -y {}", tool_names.join(" ")),
            "npm" => format!("npm uninstall -g {}", tool_names.join(" ")),
            _ => format!("# Uninstall from {}: {}", from, tool_names.join(" ")),
        };

        println!("  {} {} → {}:", ">".cyan(), from, to);
        println!("    1. {}", install_cmd.green());
        println!("    2. {}", uninstall_cmd.yellow());
        println!();
    }
}

/// Execute migration for selected candidates
fn execute_migration(db: &Database, candidates: &[crate::ai::MigrationCandidate]) -> Result<()> {
    use crate::commands::install::handle_running_process;
    use indicatif::{ProgressBar, ProgressStyle};
    use std::time::Duration;

    for candidate in candidates {
        println!();
        println!(
            "{} Migrating {} ({} → {})",
            ">".cyan(),
            candidate.name.bold(),
            candidate.from_source,
            candidate.to_source
        );

        // Check if process is running (get binary name from DB if available)
        let binary_name = db
            .get_tool_by_name(&candidate.name)
            .ok()
            .flatten()
            .and_then(|t| t.binary_name)
            .unwrap_or_else(|| candidate.name.clone());

        if !handle_running_process(&binary_name)? {
            println!(
                "  {} Migration cancelled for {}",
                "!".yellow(),
                candidate.name
            );
            continue;
        }

        // 1. Install from new source using SafeCommand
        let safe_install_cmd = match get_safe_install_command(
            &candidate.to_package_name,
            &candidate.to_source,
            None,
        ) {
            Ok(Some(cmd)) => cmd,
            Ok(None) => {
                println!(
                    "  {} Cannot auto-install from {}",
                    "!".yellow(),
                    candidate.to_source
                );
                continue;
            }
            Err(e) => {
                println!("  {} Invalid package name: {}", "!".red(), e);
                continue;
            }
        };

        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(format!("Installing from {}...", candidate.to_source));
        spinner.enable_steady_tick(Duration::from_millis(80));

        let install_result = safe_install_cmd.execute();

        spinner.finish_and_clear();

        match install_result {
            Ok(status) if status.success() => {
                println!("  {} Installed from {}", "+".green(), candidate.to_source);
            }
            Ok(_status) => {
                println!(
                    "  {} Failed to install from {}",
                    "!".red(),
                    candidate.to_source
                );
                continue;
            }
            Err(e) => {
                println!("  {} Install failed: {}", "!".red(), e);
                continue;
            }
        }

        // 2. Uninstall from old source using SafeCommand
        let safe_uninstall_cmd =
            match get_safe_uninstall_command(&candidate.name, &candidate.from_source) {
                Ok(Some(cmd)) => cmd,
                Ok(None) => {
                    println!(
                        "  {} Skipping uninstall from {} (manual removal needed)",
                        "!".yellow(),
                        candidate.from_source
                    );
                    continue;
                }
                Err(e) => {
                    println!("  {} Invalid package name: {}", "!".red(), e);
                    continue;
                }
            };

        // Check if this is a sudo command (apt, snap)
        let needs_sudo = safe_uninstall_cmd.program == "sudo";

        // For sudo commands, we need to inherit stdio so the user can enter password
        let uninstall_success = if needs_sudo {
            println!(
                "  {} Removing from {} (may prompt for password)...",
                ">".cyan(),
                candidate.from_source
            );
            match Command::new(safe_uninstall_cmd.program)
                .args(&safe_uninstall_cmd.args)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
            {
                Ok(status) => status.success(),
                Err(e) => {
                    println!("  {} Uninstall warning: {}", "!".yellow(), e);
                    false
                }
            }
        } else {
            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            spinner.set_message(format!("Removing from {}...", candidate.from_source));
            spinner.enable_steady_tick(Duration::from_millis(80));

            let result = safe_uninstall_cmd.execute();

            spinner.finish_and_clear();

            match result {
                Ok(status) => status.success(),
                Err(e) => {
                    println!("  {} Uninstall warning: {}", "!".yellow(), e);
                    false
                }
            }
        };

        if uninstall_success {
            println!("  {} Removed from {}", "+".green(), candidate.from_source);
        } else {
            println!(
                "  {} Could not remove from {} (may need manual cleanup)",
                "!".yellow(),
                candidate.from_source
            );
        }

        // 3. Update database
        if let Err(e) = db.update_tool_source(&candidate.name, &candidate.to_source) {
            println!("  {} Could not update database: {}", "!".yellow(), e);
        } else {
            println!("  {} Database updated", "+".green());
        }

        println!("  {} Migrated {} successfully", "✓".green(), candidate.name);
    }

    Ok(())
}
