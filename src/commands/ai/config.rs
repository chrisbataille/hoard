//! AI configuration commands
//!
//! Commands for setting up and testing AI providers.

use anyhow::Result;
use colored::Colorize;
use std::process::Command;

use crate::{AiProvider, HoardConfig};

/// Set the AI provider
pub fn cmd_ai_set(provider: &str) -> Result<()> {
    let ai_provider = AiProvider::from(provider);

    if ai_provider == AiProvider::None {
        println!(
            "{} Unknown provider '{}'. Valid options: claude, gemini, codex, opencode",
            "!".yellow(),
            provider
        );
        return Ok(());
    }

    // Check if the CLI tool is installed
    if !ai_provider.is_installed() {
        println!(
            "{} Warning: '{}' CLI not found in PATH",
            "!".yellow(),
            ai_provider.command().unwrap_or("unknown")
        );
        println!("  The provider will be saved, but AI features won't work until installed.");
    }

    let mut config = HoardConfig::load()?;
    config.set_ai_provider(ai_provider);
    config.save()?;

    println!("{} AI provider set to '{}'", "+".green(), ai_provider);
    println!(
        "  Config saved to: {}",
        HoardConfig::config_path()?.display()
    );

    Ok(())
}

/// Set the Claude model to use
pub fn cmd_ai_model(model: &str) -> Result<()> {
    use crate::config::ClaudeModel;

    let claude_model = ClaudeModel::from(model);

    let mut config = HoardConfig::load()?;

    // Warn if provider is not Claude
    if config.ai.provider != AiProvider::Claude {
        println!(
            "{} Note: Claude model set to '{}', but current provider is '{}'",
            "!".yellow(),
            claude_model,
            config.ai.provider
        );
        println!("  The model setting will only be used when provider is set to 'claude'.");
    }

    config.ai.claude_model = claude_model;
    config.save()?;

    println!("{} Claude model set to '{}'", "+".green(), claude_model);
    println!(
        "  Config saved to: {}",
        HoardConfig::config_path()?.display()
    );

    Ok(())
}

/// Show current AI configuration
pub fn cmd_ai_show() -> Result<()> {
    let config = HoardConfig::load()?;

    println!("{}", "AI Configuration".bold());
    println!("{}", "=".repeat(30));
    println!();

    let provider = &config.ai.provider;
    let status = if provider == &AiProvider::None {
        "not configured".red().to_string()
    } else if provider.is_installed() {
        "installed".green().to_string()
    } else {
        "not installed".yellow().to_string()
    };

    println!("Provider: {} [{}]", provider.to_string().cyan(), status);

    if let Some(cmd) = provider.command() {
        println!("Command:  {}", cmd);
    }

    // Show Claude model if provider is Claude
    if *provider == AiProvider::Claude {
        println!("Model:    {}", config.ai.claude_model.to_string().cyan());
    }

    println!();
    println!("Config file: {}", HoardConfig::config_path()?.display());

    Ok(())
}

/// Test the AI provider
pub fn cmd_ai_test() -> Result<()> {
    let config = HoardConfig::load()?;

    if config.ai.provider == AiProvider::None {
        println!("{} No AI provider configured", "!".yellow());
        println!("  Use {} to set one", "hoards ai set <provider>".cyan());
        return Ok(());
    }

    let provider = &config.ai.provider;
    let cmd = match provider.command() {
        Some(c) => c,
        None => {
            println!("{} No command for provider '{}'", "!".red(), provider);
            return Ok(());
        }
    };

    println!("{} Testing {} CLI...", ">".cyan(), provider);

    // Check if command exists
    if !provider.is_installed() {
        println!("{} '{}' not found in PATH", "!".red(), cmd);
        return Ok(());
    }

    // Try to get version or help to verify it works
    let output = Command::new(cmd).arg("--version").output();

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            let version = version.trim();
            if version.is_empty() {
                println!("{} {} is available", "+".green(), cmd);
            } else {
                println!("{} {} - {}", "+".green(), cmd, version.dimmed());
            }
        }
        Ok(_) => {
            // --version might not be supported, try --help
            let help_out = Command::new(cmd).arg("--help").output();
            match help_out {
                Ok(h) if h.status.success() || !h.stdout.is_empty() => {
                    println!("{} {} is available", "+".green(), cmd);
                }
                _ => {
                    println!(
                        "{} {} found but may not be working correctly",
                        "!".yellow(),
                        cmd
                    );
                }
            }
        }
        Err(e) => {
            println!("{} Failed to run '{}': {}", "!".red(), cmd, e);
        }
    }

    Ok(())
}
