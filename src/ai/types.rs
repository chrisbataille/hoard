//! Types for AI features
//!
//! This module contains all data structures used by AI features.

use serde::{Deserialize, Serialize};

/// A mapping from a traditional Unix tool to its modern replacement
#[derive(Debug, Clone)]
pub struct ToolReplacement {
    /// Traditional tool name (e.g., "grep")
    pub traditional: &'static str,
    /// Modern replacement tool name (e.g., "ripgrep")
    pub modern: &'static str,
    /// Binary name of the modern tool (e.g., "rg")
    pub modern_binary: &'static str,
    /// Suggested action/alias (e.g., "alias grep='rg'")
    pub tip: &'static str,
    /// Benefit description (e.g., "10x faster")
    pub benefit: &'static str,
}

/// An optimization tip from usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeTip {
    pub traditional: String,
    pub traditional_uses: i64,
    pub modern: String,
    pub modern_binary: String,
    pub benefit: String,
    pub action: String,
}

/// An underutilized installed tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderutilizedTool {
    pub name: String,
    pub description: Option<String>,
    pub stars: Option<u64>,
}

/// Result of usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub tips: Vec<AnalyzeTip>,
    pub underutilized: Vec<UnderutilizedTool>,
    pub ai_insight: Option<String>,
}

/// A tool that can be migrated to a different source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCandidate {
    pub name: String,
    pub from_source: String,
    pub from_version: String,
    pub to_source: String,
    pub to_version: String,
    pub to_package_name: String, // Package name on target source (may differ)
    pub benefit: Option<String>, // AI-generated
}

/// Result of migration analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub candidates: Vec<MigrationCandidate>,
    pub ai_summary: Option<String>,
}

/// Bundle suggestion from AI
#[derive(Debug)]
pub struct BundleSuggestion {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub reasoning: Option<String>,
}

/// Extracted tool information from a GitHub README
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTool {
    pub name: String,
    pub binary: Option<String>,
    pub source: String,
    pub install_command: Option<String>,
    pub description: String,
    pub category: String,
}

/// A command in a cheatsheet section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatsheetCommand {
    pub cmd: String,
    pub desc: String,
}

/// A section in a cheatsheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatsheetSection {
    pub name: String,
    pub commands: Vec<CheatsheetCommand>,
}

/// Generated cheatsheet for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cheatsheet {
    pub title: String,
    pub sections: Vec<CheatsheetSection>,
}

/// Cached cheatsheet with version info for invalidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCheatsheet {
    pub version: Option<String>,
    pub cheatsheet: Cheatsheet,
}

/// A tool recommendation from AI discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecommendation {
    pub name: String,
    #[serde(default)]
    pub binary: Option<String>,
    pub description: String,
    pub category: String, // "essential" or "recommended"
    pub reason: String,
    pub source: String,
    pub install_cmd: String,
    #[serde(default)]
    pub github: Option<String>,
    #[serde(skip)]
    pub stars: Option<u64>,
    #[serde(skip)]
    pub installed: bool,
}

/// Discovery response from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponse {
    pub summary: String,
    pub tools: Vec<ToolRecommendation>,
}
