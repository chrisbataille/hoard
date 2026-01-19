//! Modern tool replacements data
//!
//! Known mappings of traditional Unix tools to their modern replacements.

use super::types::ToolReplacement;

/// Known mappings of traditional tools to modern replacements
pub const MODERN_REPLACEMENTS: &[ToolReplacement] = &[
    ToolReplacement {
        traditional: "grep",
        modern: "ripgrep",
        modern_binary: "rg",
        tip: "alias grep='rg'",
        benefit: "10x faster regex search",
    },
    ToolReplacement {
        traditional: "find",
        modern: "fd",
        modern_binary: "fd",
        tip: "fd <pattern>",
        benefit: "5x faster, simpler syntax",
    },
    ToolReplacement {
        traditional: "cat",
        modern: "bat",
        modern_binary: "bat",
        tip: "alias cat='bat'",
        benefit: "syntax highlighting, git integration",
    },
    ToolReplacement {
        traditional: "ls",
        modern: "eza",
        modern_binary: "eza",
        tip: "alias ls='eza'",
        benefit: "git status, icons, better colors",
    },
    ToolReplacement {
        traditional: "du",
        modern: "dust",
        modern_binary: "dust",
        tip: "dust",
        benefit: "intuitive visual output",
    },
    ToolReplacement {
        traditional: "df",
        modern: "duf",
        modern_binary: "duf",
        tip: "duf",
        benefit: "better formatting, colors",
    },
    ToolReplacement {
        traditional: "ps",
        modern: "procs",
        modern_binary: "procs",
        tip: "procs",
        benefit: "structured output, colors",
    },
    ToolReplacement {
        traditional: "top",
        modern: "btop",
        modern_binary: "btop",
        tip: "btop",
        benefit: "interactive TUI, resource graphs",
    },
    ToolReplacement {
        traditional: "htop",
        modern: "btop",
        modern_binary: "btop",
        tip: "btop",
        benefit: "more visual, better resource graphs",
    },
    ToolReplacement {
        traditional: "sed",
        modern: "sd",
        modern_binary: "sd",
        tip: "sd 'old' 'new' file",
        benefit: "simpler syntax, no escaping",
    },
    ToolReplacement {
        traditional: "diff",
        modern: "delta",
        modern_binary: "delta",
        tip: "git config core.pager delta",
        benefit: "syntax highlighting, side-by-side",
    },
    ToolReplacement {
        traditional: "man",
        modern: "tldr",
        modern_binary: "tldr",
        tip: "tldr <command>",
        benefit: "practical examples, concise",
    },
    ToolReplacement {
        traditional: "curl",
        modern: "xh",
        modern_binary: "xh",
        tip: "xh httpbin.org/get",
        benefit: "cleaner output, easier syntax",
    },
    ToolReplacement {
        traditional: "cut",
        modern: "choose",
        modern_binary: "choose",
        tip: "choose -f 1,3",
        benefit: "human-friendly field selection",
    },
    ToolReplacement {
        traditional: "ping",
        modern: "gping",
        modern_binary: "gping",
        tip: "gping google.com",
        benefit: "graphical ping visualization",
    },
];
