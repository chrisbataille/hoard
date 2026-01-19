//! Tests for AI module

use super::builders::extract_prompt;
use super::helpers::parse_github_url;
use super::parsers::{extract_json_array, extract_json_object, parse_extract_response};
use super::prompts::*;

#[test]
fn test_extract_json_object() {
    let response = r#"Here's the categorization:
{"ripgrep": "search", "bat": "files"}
Done!"#;
    let json = extract_json_object(response).unwrap();
    assert_eq!(json, r#"{"ripgrep": "search", "bat": "files"}"#);
}

#[test]
fn test_extract_json_array() {
    let response = r#"Here are my suggestions:
[{"name": "test", "description": "desc", "tools": ["a", "b"]}]
"#;
    let json = extract_json_array(response).unwrap();
    assert!(json.starts_with('['));
    assert!(json.ends_with(']'));
}

#[test]
fn test_default_prompts_have_placeholders() {
    assert!(DEFAULT_CATEGORIZE_PROMPT.contains("{{CATEGORIES}}"));
    assert!(DEFAULT_CATEGORIZE_PROMPT.contains("{{TOOLS}}"));
    assert!(DEFAULT_DESCRIBE_PROMPT.contains("{{TOOLS}}"));
    assert!(DEFAULT_SUGGEST_BUNDLE_PROMPT.contains("{{COUNT}}"));
    assert!(DEFAULT_SUGGEST_BUNDLE_PROMPT.contains("{{TOOLS}}"));
    assert!(DEFAULT_EXTRACT_PROMPT.contains("{{README}}"));
}

#[test]
fn test_parse_github_url_https() {
    let (owner, repo) = parse_github_url("https://github.com/BurntSushi/ripgrep").unwrap();
    assert_eq!(owner, "BurntSushi");
    assert_eq!(repo, "ripgrep");
}

#[test]
fn test_parse_github_url_https_with_git() {
    let (owner, repo) = parse_github_url("https://github.com/BurntSushi/ripgrep.git").unwrap();
    assert_eq!(owner, "BurntSushi");
    assert_eq!(repo, "ripgrep");
}

#[test]
fn test_parse_github_url_https_with_path() {
    let (owner, repo) =
        parse_github_url("https://github.com/BurntSushi/ripgrep/tree/master").unwrap();
    assert_eq!(owner, "BurntSushi");
    assert_eq!(repo, "ripgrep");
}

#[test]
fn test_parse_github_url_ssh() {
    let (owner, repo) = parse_github_url("git@github.com:BurntSushi/ripgrep.git").unwrap();
    assert_eq!(owner, "BurntSushi");
    assert_eq!(repo, "ripgrep");
}

#[test]
fn test_parse_github_url_shorthand() {
    let (owner, repo) = parse_github_url("BurntSushi/ripgrep").unwrap();
    assert_eq!(owner, "BurntSushi");
    assert_eq!(repo, "ripgrep");
}

#[test]
fn test_parse_github_url_invalid() {
    assert!(parse_github_url("not-a-url").is_err());
    assert!(parse_github_url("https://gitlab.com/foo/bar").is_err());
}

#[test]
fn test_parse_extract_response() {
    let response = r#"Here's the extracted info:
{"name": "ripgrep", "binary": "rg", "source": "cargo", "install_command": "cargo install ripgrep", "description": "Fast regex search", "category": "search"}
"#;
    let tool = parse_extract_response(response).unwrap();
    assert_eq!(tool.name, "ripgrep");
    assert_eq!(tool.binary, Some("rg".to_string()));
    assert_eq!(tool.source, "cargo");
    assert_eq!(tool.category, "search");
}

#[test]
fn test_parse_extract_response_minimal() {
    let response =
        r#"{"name": "foo", "source": "pip", "description": "A tool", "category": "misc"}"#;
    let tool = parse_extract_response(response).unwrap();
    assert_eq!(tool.name, "foo");
    assert_eq!(tool.binary, None);
    assert_eq!(tool.install_command, None);
}

#[test]
fn test_extract_prompt_truncates_long_readme() {
    let long_readme = "x".repeat(10000);
    let prompt = extract_prompt(&long_readme);
    assert!(prompt.contains("[README truncated]"));
    assert!(prompt.len() < 10000);
}
