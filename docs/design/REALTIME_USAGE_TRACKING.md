# Real-time Usage Tracking Design

## Overview

Add configurable usage tracking with two modes:
1. **History scan** (current): Manual `usage scan` parses shell history files
2. **Preexec hook** (new): Real-time tracking via shell hooks

## User Flow

### First-Time Setup

When user runs any `usage` command and tracking mode is not configured:

```
$ hoards usage show

> Usage tracking is not configured. How would you like to track tool usage?

  1. History scan (manual) - Run 'hoards usage scan' periodically
  2. Shell hook (automatic) - Track commands in real-time

> Selection: 2

> Detected shell: bash
> Bash requires bash-preexec for hooks. Install it now? [Y/n]: y

+ Installing bash-preexec...
+ Add this to your ~/.bashrc:

  [[ -f ~/.bash-preexec.sh ]] && source ~/.bash-preexec.sh
  preexec() { hoards usage log "${1%% *}" &>/dev/null & }

> Configuration saved. Run 'hoards usage init' anytime to see setup instructions.
```

### Changing Configuration

```bash
# View current setting
hoards usage config

# Change tracking mode
hoards usage config --mode hook
hoards usage config --mode scan

# Reset counters (separate action)
hoards usage reset
```

## Configuration

### Config Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HoardConfig {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub usage: UsageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageConfig {
    /// Tracking mode: "scan" or "hook"
    pub mode: Option<UsageMode>,
    /// Shell for hook mode (fish, bash, zsh)
    pub shell: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UsageMode {
    Scan,
    Hook,
}
```

### Config File Example

```toml
# ~/.config/hoards/config.toml

[ai]
provider = "claude"

[usage]
mode = "hook"
shell = "fish"
```

## CLI Changes

```rust
#[derive(Subcommand)]
pub enum UsageCommands {
    /// Scan shell history and update usage counts
    Scan { ... },

    /// Show usage statistics
    Show { ... },

    /// Show usage for a specific tool
    Tool { ... },

    /// Log a single command usage (for shell hooks)
    Log {
        /// Command that was executed
        command: String,
    },

    /// Show shell hook setup instructions
    Init {
        /// Shell type (auto-detected if omitted)
        #[arg(value_parser = ["fish", "bash", "zsh"])]
        shell: Option<String>,
    },

    /// View or change usage tracking configuration
    Config {
        /// Set tracking mode
        #[arg(long, value_parser = ["scan", "hook"])]
        mode: Option<String>,
    },

    /// Reset all usage counters to zero
    Reset {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}
```

## Implementation Details

### Setup Flow (`ensure_usage_configured`)

Called at start of `usage show`, `usage scan`, `usage tool`:

```rust
fn ensure_usage_configured(config: &mut HoardConfig) -> Result<()> {
    if config.usage.mode.is_some() {
        return Ok(()); // Already configured
    }

    // Interactive setup
    println!("{} Usage tracking is not configured.", ">".cyan());
    println!();
    println!("How would you like to track tool usage?");
    println!("  1. History scan (manual) - Run 'hoards usage scan' periodically");
    println!("  2. Shell hook (automatic) - Track commands in real-time");
    println!();

    let selection = dialoguer::Select::new()
        .items(&["History scan", "Shell hook"])
        .default(0)
        .interact()?;

    let mode = if selection == 0 {
        UsageMode::Scan
    } else {
        // Detect shell and handle bash-preexec
        let shell = detect_shell();
        config.usage.shell = Some(shell.clone());

        if shell == "bash" {
            offer_bash_preexec_install()?;
        }

        print_hook_instructions(&shell);
        UsageMode::Hook
    };

    config.usage.mode = Some(mode);
    config.save()?;

    Ok(())
}
```

### Bash-preexec Installation

```rust
fn offer_bash_preexec_install() -> Result<()> {
    println!();
    println!("{} Bash requires bash-preexec for shell hooks.", "!".yellow());
    println!("  https://github.com/rcaloras/bash-preexec");
    println!();

    let install = dialoguer::Confirm::new()
        .with_prompt("Install bash-preexec now?")
        .default(true)
        .interact()?;

    if install {
        // Use hoards install for consistency
        cmd_install(&Database::open()?, "bash-preexec", false, false)?;
        println!();
        println!("{} bash-preexec installed.", "+".green());
    }

    Ok(())
}
```

### `usage log` Command

Fast, silent, for shell hooks:

```rust
pub fn cmd_usage_log(db: &Database, command: &str) -> Result<()> {
    let cmd = extract_base_command(command);
    if cmd.is_empty() {
        return Ok(());
    }

    // Fast single-query lookup
    if let Some(tool_name) = db.match_command_to_tool(&cmd)? {
        let now = Utc::now().to_rfc3339();
        db.record_usage(&tool_name, 1, Some(&now))?;
    }

    Ok(())
}

fn extract_base_command(input: &str) -> String {
    // Reuse logic from history.rs extract_command()
    crate::history::extract_command(input)
        .unwrap_or_default()
}
```

### `usage init` Command

Shows setup instructions based on config:

```rust
pub fn cmd_usage_init(config: &HoardConfig, shell_override: Option<String>) -> Result<()> {
    let mode = config.usage.mode.as_ref()
        .ok_or_else(|| anyhow!("Run 'hoards usage show' first to configure tracking"))?;

    if *mode == UsageMode::Scan {
        println!("Usage tracking is set to 'scan' mode.");
        println!("Run 'hoards usage scan' to update usage counts from shell history.");
        return Ok(());
    }

    let shell = shell_override
        .or_else(|| config.usage.shell.clone())
        .unwrap_or_else(detect_shell);

    print_hook_instructions(&shell);
    Ok(())
}
```

### `usage config` Command

```rust
pub fn cmd_usage_config(config: &mut HoardConfig, mode: Option<String>) -> Result<()> {
    match mode {
        None => {
            // Show current config
            match &config.usage.mode {
                Some(UsageMode::Scan) => println!("Tracking mode: scan (manual)"),
                Some(UsageMode::Hook) => {
                    let shell = config.usage.shell.as_deref().unwrap_or("unknown");
                    println!("Tracking mode: hook (automatic)");
                    println!("Shell: {}", shell);
                }
                None => println!("Tracking mode: not configured"),
            }
        }
        Some(new_mode) => {
            // Change mode (doesn't reset counters)
            let mode = match new_mode.as_str() {
                "scan" => UsageMode::Scan,
                "hook" => {
                    let shell = detect_shell();
                    config.usage.shell = Some(shell.clone());
                    if shell == "bash" {
                        offer_bash_preexec_install()?;
                    }
                    print_hook_instructions(&shell);
                    UsageMode::Hook
                }
                _ => bail!("Invalid mode: {}", new_mode),
            };
            config.usage.mode = Some(mode);
            config.save()?;
            println!("{} Configuration updated.", "+".green());
        }
    }
    Ok(())
}
```

### `usage reset` Command

```rust
pub fn cmd_usage_reset(db: &Database, force: bool) -> Result<()> {
    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt("Reset all usage counters to zero?")
            .default(false)
            .interact()?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }
    }

    db.clear_usage()?;
    println!("{} Usage counters reset.", "+".green());
    Ok(())
}
```

## Shell Hook Instructions

### Fish
```fish
# Add to ~/.config/fish/config.fish
function __hoard_log --on-event fish_preexec
    command hoards usage log "$argv[1]" &>/dev/null &
    disown 2>/dev/null
end
```

### Bash (with bash-preexec)
```bash
# Add to ~/.bashrc (after sourcing bash-preexec)
[[ -f ~/.bash-preexec.sh ]] && source ~/.bash-preexec.sh
preexec() {
    command hoards usage log "$1" &>/dev/null &
}
```

### Zsh
```zsh
# Add to ~/.zshrc
preexec() {
    command hoards usage log "$1" &>/dev/null &
}
```

## Database Changes

New method for fast command lookup:

```rust
impl Database {
    /// Match a command to a tracked tool by binary or name
    pub fn match_command_to_tool(&self, cmd: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT name FROM tools WHERE binary = ?1
             UNION
             SELECT name FROM tools WHERE name = ?1
             LIMIT 1",
            [cmd],
            |row| row.get(0)
        );

        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
```

Add index for performance:
```sql
CREATE INDEX IF NOT EXISTS idx_tools_binary ON tools(binary);
```

## Files to Modify

| File | Changes |
|------|---------|
| `src/config.rs` | Add `UsageConfig`, `UsageMode` |
| `src/cli.rs` | Add `Log`, `Init`, `Config`, `Reset` to `UsageCommands` |
| `src/commands/usage.rs` | Add new command handlers |
| `src/db.rs` | Add `match_command_to_tool()` method |
| `src/main.rs` | Wire up new subcommands |
| `src/lib.rs` | Export new items |

## Migration

- Existing users: config has `usage.mode = None`, prompted on next `usage` command
- No database schema changes
- Counters preserved when changing modes

## Summary

| Command | Purpose |
|---------|---------|
| `usage show` | Show stats (prompts for config if missing) |
| `usage scan` | Manual history scan |
| `usage log <cmd>` | Log single command (for hooks) |
| `usage init [shell]` | Show hook setup instructions |
| `usage config [--mode]` | View/change tracking mode |
| `usage reset [-f]` | Reset all counters to zero |
