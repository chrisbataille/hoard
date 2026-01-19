//! Embedded default prompts for AI features
//!
//! These prompts are used when no custom prompts are found in ~/.config/hoards/prompts/

pub const DEFAULT_CATEGORIZE_PROMPT: &str = r#"You are helping categorize CLI tools. Here are the existing categories: {{CATEGORIES}}

Categorize these tools into the most appropriate category. If none fit well, use "misc".
Only respond with a JSON object mapping tool names to categories, nothing else.
Example: {"ripgrep": "search", "bat": "files", "htop": "system"}

Tools to categorize:
{{TOOLS}}
"#;

pub const DEFAULT_DESCRIBE_PROMPT: &str = r#"Generate brief descriptions (max 100 chars each) for these CLI tools.
Only respond with a JSON object mapping tool names to descriptions, nothing else.
Example: {"ripgrep": "Fast regex search tool, replacement for grep", "bat": "Cat clone with syntax highlighting"}

Tools needing descriptions:
{{TOOLS}}
"#;

pub const DEFAULT_SUGGEST_BUNDLE_PROMPT: &str = r#"Analyze this user's CLI tools and suggest {{COUNT}} logical bundles based on their ACTUAL USAGE PATTERNS.

Guidelines:
1. PRIORITIZE tools the user actually uses (higher usage count = more important)
2. Group tools that share workflows or complement each other
3. Each bundle should tell a story (e.g., "Modern Unix", "Git Power Tools", "Rust Development")
4. Include 3-6 tools per bundle for practical utility
5. Focus on installed tools with usage > 0 when possible

IMPORTANT: Do NOT suggest tools that are already in existing bundles:
{{EXISTING_BUNDLES}}

Respond ONLY with a JSON array. Each object must have:
- "name": short bundle name (kebab-case, e.g., "modern-unix")
- "description": one-line description explaining the theme
- "tools": array of tool names from the list below
- "reasoning": brief explanation of why these tools belong together

Example:
[{"name": "modern-unix", "description": "Modern replacements for traditional Unix tools", "tools": ["ripgrep", "fd", "eza", "bat"], "reasoning": "User heavily uses ripgrep (847x) and fd (423x), suggesting preference for modern alternatives"}]

Available tools with usage data (format: name [category] (usage count): description):
{{TOOLS}}
"#;

pub const DEFAULT_EXTRACT_PROMPT: &str = r#"Extract CLI tool information from this GitHub README.

Return a JSON object with these fields:
- "name": tool name (required)
- "binary": binary name if different from tool name (optional, null if same)
- "source": installation source, one of: "cargo", "pip", "npm", "apt", "brew", "snap", "flatpak", "manual" (required)
- "install_command": the install command, e.g. "cargo install ripgrep" (optional)
- "description": brief description, max 100 chars (required)
- "category": suggested category from: dev, shell, files, search, git, network, system, editor, data, security, misc (required)

Example response:
{"name": "ripgrep", "binary": "rg", "source": "cargo", "install_command": "cargo install ripgrep", "description": "Fast regex search tool, replacement for grep", "category": "search"}

README content:
{{README}}
"#;

pub const DEFAULT_CHEATSHEET_PROMPT: &str = r#"Create a concise CLI cheatsheet for the tool "{{TOOL_NAME}}" based on its --help output.

Guidelines:
1. Group commands by category (BASIC USAGE, FILE FILTERING, OUTPUT, etc.)
2. Show the most useful/common commands first
3. Keep descriptions very short (2-4 words max)
4. Include 3-5 categories with 3-5 commands each
5. Use the actual binary name in examples

Respond with JSON:
{
  "title": "tool-name (binary) - Short description",
  "sections": [
    {
      "name": "CATEGORY NAME",
      "commands": [
        {"cmd": "binary -flag arg", "desc": "Brief description"}
      ]
    }
  ]
}

Tool --help output:
{{HELP_OUTPUT}}
"#;

pub const DEFAULT_BUNDLE_CHEATSHEET_PROMPT: &str = r#"Create a workflow-oriented cheatsheet for a bundle of related CLI tools.

IMPORTANT: Organize by WORKFLOW/TASK, not by individual tool. Group related commands from different tools together based on what task they accomplish.

Bundle name: {{BUNDLE_NAME}}
Tools in bundle: {{TOOL_LIST}}

Guidelines:
1. Create categories based on workflows (e.g., "PROJECT SETUP", "DAILY WORKFLOW", "CODE QUALITY", "DEBUGGING")
2. Mix commands from different tools when they relate to the same workflow
3. Show the most common workflow patterns first
4. Keep descriptions very short (2-4 words max)
5. Include 4-6 categories with 3-6 commands each
6. Prefix commands with the tool name if ambiguous

Respond with JSON:
{
  "title": "Bundle Name - Workflow description",
  "sections": [
    {
      "name": "WORKFLOW CATEGORY",
      "commands": [
        {"cmd": "tool command -flag", "desc": "Brief description"}
      ]
    }
  ]
}

Tool help outputs:
{{HELP_OUTPUTS}}
"#;

pub const DEFAULT_DISCOVERY_PROMPT: &str = r#"You are a CLI tool expert. Based on the user's description of what they're working on, recommend relevant command-line tools.

User's context: {{QUERY}}

Already installed tools: {{INSTALLED_TOOLS}}

IMPORTANT - Only recommend tools from these package sources: {{ENABLED_SOURCES}}
Do NOT recommend tools that cannot be installed from the enabled sources above.

Guidelines:
1. Recommend 5-10 highly relevant tools
2. Categorize as "essential" (must-have) or "recommended" (nice-to-have)
3. Don't recommend tools they already have installed
4. Focus on well-maintained, popular tools
5. Include the exact install command for each
6. Be specific about why each tool is relevant
7. ONLY use sources from the enabled list above

Respond with JSON:
{
  "summary": "Brief description of the recommendations",
  "tools": [
    {
      "name": "tool-name",
      "binary": "binary-name",
      "description": "What it does (1 sentence)",
      "category": "essential|recommended",
      "reason": "Why it's relevant to their query",
      "source": "cargo|pip|npm|apt|brew",
      "install_cmd": "cargo install tool-name",
      "github": "owner/repo"
    }
  ]
}
"#;

pub const DEFAULT_ANALYZE_PROMPT: &str = r#"Analyze this CLI usage data and provide a brief personalized insight.

Traditional tool usage (from shell history):
{{TRADITIONAL_USAGE}}

Modern replacement tools installed:
{{MODERN_TOOLS}}

Unused installed tools with high potential:
{{UNUSED_TOOLS}}

Provide a brief (2-3 sentence) personalized insight about:
1. The user's apparent workflow patterns
2. Which specific unused tools would benefit them most based on their usage

Respond with JSON:
{"insight": "Your personalized analysis here"}
"#;

pub const DEFAULT_MIGRATE_PROMPT: &str = r#"For each tool being migrated between package sources, provide a brief benefit description (5-10 words) explaining why the newer version is better.

Tools being migrated:
{{TOOLS}}

For each tool, explain the key improvement in the newer version (e.g., new features, performance improvements, bug fixes).

Respond with JSON:
{"benefits": {"tool_name": "brief benefit description", ...}}
"#;
