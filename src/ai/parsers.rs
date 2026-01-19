//! Response parsing functions
//!
//! Functions to parse AI responses into structured data.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use comfy_table::{
    Attribute, Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL,
};

use super::types::{BundleSuggestion, Cheatsheet, DiscoveryResponse, ExtractedTool};

/// Extract a JSON object from a response that might contain extra text
pub fn extract_json_object(response: &str) -> Result<String> {
    let start = response
        .find('{')
        .context("No JSON object found in response")?;
    let end = response
        .rfind('}')
        .context("No closing brace found in response")?;

    if end <= start {
        bail!("Invalid JSON structure in response");
    }

    Ok(response[start..=end].to_string())
}

/// Extract a JSON array from a response that might contain extra text
pub fn extract_json_array(response: &str) -> Result<String> {
    let start = response
        .find('[')
        .context("No JSON array found in response")?;
    let end = response
        .rfind(']')
        .context("No closing bracket found in response")?;

    if end <= start {
        bail!("Invalid JSON structure in response");
    }

    Ok(response[start..=end].to_string())
}

/// Parse categorization response from AI
pub fn parse_categorize_response(response: &str) -> Result<HashMap<String, String>> {
    let json_str = extract_json_object(response)?;

    let map: HashMap<String, String> =
        serde_json::from_str(&json_str).context("Failed to parse AI response as JSON")?;

    Ok(map)
}

/// Parse description response from AI
pub fn parse_describe_response(response: &str) -> Result<HashMap<String, String>> {
    let json_str = extract_json_object(response)?;

    let map: HashMap<String, String> =
        serde_json::from_str(&json_str).context("Failed to parse AI response as JSON")?;

    Ok(map)
}

/// Parse bundle suggestion response from AI
pub fn parse_bundle_response(response: &str) -> Result<Vec<BundleSuggestion>> {
    let json_str = extract_json_array(response)?;

    #[derive(serde::Deserialize)]
    struct RawSuggestion {
        name: String,
        description: String,
        tools: Vec<String>,
        reasoning: Option<String>,
    }

    let raw: Vec<RawSuggestion> =
        serde_json::from_str(&json_str).context("Failed to parse AI response as JSON")?;

    Ok(raw
        .into_iter()
        .map(|r| BundleSuggestion {
            name: r.name,
            description: r.description,
            tools: r.tools,
            reasoning: r.reasoning,
        })
        .collect())
}

/// Parse extraction response from AI
pub fn parse_extract_response(response: &str) -> Result<ExtractedTool> {
    let json_str = extract_json_object(response)?;

    let tool: ExtractedTool =
        serde_json::from_str(&json_str).context("Failed to parse AI extraction response")?;

    // Validate required fields
    if tool.name.is_empty() {
        bail!("Extracted tool has no name");
    }
    if tool.description.is_empty() {
        bail!("Extracted tool has no description");
    }

    Ok(tool)
}

/// Parse cheatsheet response from AI
pub fn parse_cheatsheet_response(response: &str) -> Result<Cheatsheet> {
    let json_str = extract_json_object(response)?;
    let cheatsheet: Cheatsheet =
        serde_json::from_str(&json_str).context("Failed to parse AI cheatsheet response")?;
    Ok(cheatsheet)
}

/// Parse discovery response from AI
pub fn parse_discovery_response(response: &str) -> Result<DiscoveryResponse> {
    let json_str = extract_json_object(response)?;
    let discovery: DiscoveryResponse =
        serde_json::from_str(&json_str).context("Failed to parse discovery response")?;
    Ok(discovery)
}

/// Parse analyze insight response from AI
pub fn parse_analyze_response(response: &str) -> Result<String> {
    let json_str = extract_json_object(response)?;

    #[derive(serde::Deserialize)]
    struct AnalyzeInsight {
        insight: String,
    }

    let insight: AnalyzeInsight =
        serde_json::from_str(&json_str).context("Failed to parse analyze response")?;
    Ok(insight.insight)
}

/// Parse migration benefits response from AI
pub fn parse_migrate_response(response: &str) -> Result<HashMap<String, String>> {
    let json_str = extract_json_object(response)?;

    #[derive(serde::Deserialize)]
    struct MigrateBenefits {
        benefits: HashMap<String, String>,
    }

    let result: MigrateBenefits =
        serde_json::from_str(&json_str).context("Failed to parse migrate response")?;
    Ok(result.benefits)
}

/// Format a cheatsheet for terminal display using comfy-table
pub fn format_cheatsheet(cheatsheet: &Cheatsheet) -> String {
    let mut output = Vec::new();

    // Create title table
    let mut title_table = Table::new();
    title_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(72);

    title_table.add_row(vec![
        Cell::new(&cheatsheet.title)
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
    ]);

    output.push(title_table.to_string());
    output.push(String::new());

    // Create a table for each section
    for section in &cheatsheet.sections {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(72);

        // Section header
        table.set_header(vec![
            Cell::new(&section.name)
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
            Cell::new(""),
        ]);

        // Commands
        for cmd in &section.commands {
            table.add_row(vec![
                Cell::new(&cmd.cmd).fg(Color::Yellow),
                Cell::new(&cmd.desc),
            ]);
        }

        output.push(table.to_string());
        output.push(String::new());
    }

    // Remove trailing empty line
    if output.last().map(|s| s.is_empty()).unwrap_or(false) {
        output.pop();
    }

    output.join("\n")
}
