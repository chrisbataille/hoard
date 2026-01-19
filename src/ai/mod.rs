//! AI provider integration for smart features
//!
//! Provides functions to invoke configured AI CLI tools (claude, gemini, codex, opencode)
//! and parse their responses for categorization, description generation, and bundle suggestions.
//!
//! Prompts are loaded from `~/.config/hoards/prompts/` and can be customized by the user.
//! If a prompt file is missing, embedded defaults are used.

mod builders;
mod helpers;
mod parsers;
mod prompts;
mod replacements;
mod types;

#[cfg(test)]
mod tests;

use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::config::{AiProvider, HoardConfig};

// Re-export types
pub use types::{
    AnalysisResult, AnalyzeTip, BundleSuggestion, CachedCheatsheet, Cheatsheet, CheatsheetCommand,
    CheatsheetSection, DiscoveryResponse, ExtractedTool, MigrationCandidate, MigrationResult,
    ToolRecommendation, ToolReplacement, UnderutilizedTool,
};

// Re-export replacements data
pub use replacements::MODERN_REPLACEMENTS;

// Re-export prompt builders
pub use builders::{
    analyze_prompt, bundle_cheatsheet_prompt, categorize_prompt, cheatsheet_prompt,
    describe_prompt, discovery_prompt, extract_prompt, migrate_prompt, prompts_dir,
    suggest_bundle_prompt,
};

// Re-export parsers
pub use parsers::{
    format_cheatsheet, parse_analyze_response, parse_bundle_response, parse_categorize_response,
    parse_cheatsheet_response, parse_describe_response, parse_discovery_response,
    parse_extract_response, parse_migrate_response,
};

// Re-export helpers
pub use helpers::{
    fetch_readme, fetch_repo_version, get_help_output, get_tool_version, is_binary_installed,
    parse_github_url,
};

/// Invoke the configured AI provider with a prompt
pub fn invoke_ai(prompt: &str) -> Result<String> {
    let config = HoardConfig::load()?;
    let provider = &config.ai.provider;

    if *provider == AiProvider::None {
        bail!("No AI provider configured. Run 'hoards ai set <provider>' first.");
    }

    let cmd_name = provider
        .command()
        .context("Invalid AI provider configuration")?;

    if !provider.is_installed() {
        bail!(
            "AI provider '{}' is not installed. Please install it first.",
            cmd_name
        );
    }

    // Build the command based on provider
    let output = match provider {
        AiProvider::Claude => {
            // claude --model <model> -p "prompt" for non-interactive mode
            let model = config.ai.claude_model.as_cli_arg();
            Command::new(cmd_name)
                .arg("--model")
                .arg(model)
                .arg("-p")
                .arg(prompt)
                .output()
                .context("Failed to execute claude")?
        }
        AiProvider::Gemini => {
            // gemini "prompt"
            Command::new(cmd_name)
                .arg(prompt)
                .output()
                .context("Failed to execute gemini")?
        }
        AiProvider::Codex => {
            // codex -q "prompt" for quiet mode
            Command::new(cmd_name)
                .arg("-q")
                .arg(prompt)
                .output()
                .context("Failed to execute codex")?
        }
        AiProvider::Opencode => {
            // opencode "prompt"
            Command::new(cmd_name)
                .arg(prompt)
                .output()
                .context("Failed to execute opencode")?
        }
        AiProvider::None => unreachable!(),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("AI command failed: {}", stderr);
    }

    let response = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(response.trim().to_string())
}
