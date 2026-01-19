//! README popup operations for the TUI
//!
//! This module contains all methods related to the README popup functionality,
//! including fetching, displaying, scrolling, and link extraction.

use super::App;
use super::types::ReadmePopup;

impl App {
    // ========================================================================
    // README Popup Core
    // ========================================================================

    /// Start loading README for a discover result
    pub fn open_readme(&mut self, tool_name: String, url: Option<&str>) {
        if let Some(url) = url {
            // Try to get GitHub URL (directly or from package registry metadata)
            let github_url = if url.contains("github.com") {
                Some(url.to_string())
            } else if url.contains("crates.io") {
                // Try to get repository URL from crates.io API
                Self::fetch_crates_io_repo_url(&tool_name)
            } else if url.contains("npmjs.com") {
                // Try to get repository URL from npm API
                Self::fetch_npm_repo_url(&tool_name)
            } else {
                None
            };

            match github_url {
                Some(gh_url) if gh_url.contains("github.com") => {
                    // We have a GitHub URL - fetch README
                    let readme_url = Self::github_readme_url(&gh_url);

                    self.readme_popup = Some(ReadmePopup {
                        tool_name: tool_name.clone(),
                        content: String::new(),
                        scroll_offset: 0,
                        loading: true,
                        links: Vec::new(),
                        show_links: false,
                        selected_link: 0,
                    });

                    match Self::fetch_readme(&readme_url) {
                        Ok(content) => {
                            // Extract links from the content
                            let links = Self::extract_markdown_links(&content);
                            if let Some(popup) = &mut self.readme_popup {
                                popup.content = content;
                                popup.loading = false;
                                popup.links = links;
                            }
                        }
                        Err(e) => {
                            self.readme_popup = None;
                            self.notify_error(format!("Failed to fetch README: {}", e));
                        }
                    }
                }
                _ => {
                    // No GitHub URL available - open package page in browser
                    self.notify_info(format!("Opening {} in browser", tool_name));
                    self.open_url(url);
                }
            }
        } else {
            self.notify_warning(format!("No URL available for {}", tool_name));
        }
    }

    /// Close README popup
    pub fn close_readme(&mut self) {
        self.readme_popup = None;
    }

    /// Check if README popup is showing
    pub fn has_readme_popup(&self) -> bool {
        self.readme_popup.is_some()
    }

    // ========================================================================
    // README Scrolling
    // ========================================================================

    /// Scroll README up
    pub fn scroll_readme_up(&mut self, amount: u16) {
        if let Some(popup) = &mut self.readme_popup {
            popup.scroll_offset = popup.scroll_offset.saturating_sub(amount);
        }
    }

    /// Scroll README down
    pub fn scroll_readme_down(&mut self, amount: u16) {
        if let Some(popup) = &mut self.readme_popup {
            popup.scroll_offset = popup.scroll_offset.saturating_add(amount);
        }
    }

    // ========================================================================
    // README Link Picker
    // ========================================================================

    /// Toggle the link picker in README popup
    pub fn toggle_readme_links(&mut self) {
        if let Some(popup) = &mut self.readme_popup {
            if !popup.links.is_empty() {
                popup.show_links = !popup.show_links;
                popup.selected_link = 0;
            } else {
                self.notify_info("No links found in this README");
            }
        }
    }

    /// Select next link in picker
    pub fn select_next_link(&mut self) {
        if let Some(popup) = &mut self.readme_popup
            && popup.show_links
            && !popup.links.is_empty()
        {
            popup.selected_link = (popup.selected_link + 1).min(popup.links.len() - 1);
        }
    }

    /// Select previous link in picker
    pub fn select_prev_link(&mut self) {
        if let Some(popup) = &mut self.readme_popup
            && popup.show_links
            && popup.selected_link > 0
        {
            popup.selected_link -= 1;
        }
    }

    /// Open the currently selected link
    pub fn open_selected_link(&mut self) {
        if let Some(popup) = &self.readme_popup
            && popup.show_links
            && let Some((_, url)) = popup.links.get(popup.selected_link)
        {
            let url = url.clone();
            self.open_url(&url);
            self.notify_info(format!("Opening {}", url));
        }
        // Close link picker after opening
        if let Some(popup) = &mut self.readme_popup {
            popup.show_links = false;
        }
    }

    // ========================================================================
    // README URL Helpers
    // ========================================================================

    /// Open a URL in the system browser (with suppressed output)
    pub(super) fn open_url(&self, url: &str) {
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open")
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open")
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    /// Extract links from markdown content
    fn extract_markdown_links(content: &str) -> Vec<(String, String)> {
        let mut links = Vec::new();

        // Match [text](url) pattern
        let link_regex = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        for cap in link_regex.captures_iter(content) {
            let text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let url = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            if !url.is_empty() {
                links.push((text.to_string(), url.to_string()));
            }
        }

        // Also match bare URLs
        let url_regex = regex::Regex::new(r"https?://[^\s\)\]<>]+").unwrap();
        for mat in url_regex.find_iter(content) {
            let url = mat.as_str();
            // Skip if this URL is already in a markdown link
            if !links.iter().any(|(_, u)| u == url) {
                links.push((url.to_string(), url.to_string()));
            }
        }

        links
    }

    // ========================================================================
    // GitHub API Helpers
    // ========================================================================

    /// Fetch repository URL from crates.io API
    fn fetch_crates_io_repo_url(crate_name: &str) -> Option<String> {
        use crate::http::HTTP_AGENT;

        let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
        let response = HTTP_AGENT.get(&url).call().ok()?;

        if response.status() != 200 {
            return None;
        }

        let json: serde_json::Value = response.into_body().read_json().ok()?;
        json["crate"]["repository"]
            .as_str()
            .filter(|r| r.contains("github.com"))
            .map(String::from)
    }

    /// Fetch repository URL from npm API
    fn fetch_npm_repo_url(package_name: &str) -> Option<String> {
        use crate::http::HTTP_AGENT;

        let url = format!("https://registry.npmjs.org/{}", package_name);
        let response = HTTP_AGENT.get(&url).call().ok()?;

        if response.status() != 200 {
            return None;
        }

        let json: serde_json::Value = response.into_body().read_json().ok()?;

        // npm stores repo info in repository.url field
        json["repository"]["url"]
            .as_str()
            .filter(|r| r.contains("github.com"))
            // npm URLs often have "git+https://" or "git://" prefix
            .map(|r| {
                r.trim_start_matches("git+")
                    .trim_start_matches("git://")
                    .replace(".git", "")
            })
            .map(|r| {
                if r.starts_with("https://") {
                    r
                } else {
                    format!("https://{}", r)
                }
            })
    }

    /// Convert a GitHub repo URL to raw README URL, resolving redirects via API
    fn github_readme_url(repo_url: &str) -> String {
        // Extract user/repo from URL
        let repo_path = repo_url
            .strip_prefix("https://github.com/")
            .or_else(|| repo_url.strip_prefix("http://github.com/"))
            .unwrap_or(repo_url)
            .trim_end_matches('/')
            .trim_end_matches(".git");

        // Try to resolve actual repo location via GitHub API (handles renames/transfers)
        let resolved_path =
            Self::resolve_github_repo(repo_path).unwrap_or_else(|| repo_path.to_string());

        // Get default branch (usually main or master)
        let branch =
            Self::get_github_default_branch(&resolved_path).unwrap_or_else(|| "HEAD".to_string());

        // Find actual README filename via API
        let readme_name =
            Self::find_readme_filename(&resolved_path).unwrap_or_else(|| "README.md".to_string());

        format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            resolved_path, branch, readme_name
        )
    }

    /// Find the actual README filename in a repo
    fn find_readme_filename(repo_path: &str) -> Option<String> {
        use crate::http::HTTP_AGENT;

        let api_url = format!("https://api.github.com/repos/{}/contents/", repo_path);
        let response = HTTP_AGENT
            .get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "hoards-cli")
            .call()
            .ok()?;

        if response.status() != 200 {
            return None;
        }

        let json: serde_json::Value = response.into_body().read_json().ok()?;
        let files = json.as_array()?;

        // Look for README files (case-insensitive, various extensions)
        let readme_patterns = [
            "readme.md",
            "readme.adoc",
            "readme.rst",
            "readme.txt",
            "readme",
        ];

        for file in files {
            if let Some(name) = file["name"].as_str() {
                let lower_name = name.to_lowercase();
                for pattern in &readme_patterns {
                    if lower_name == *pattern {
                        return Some(name.to_string());
                    }
                }
            }
        }

        None
    }

    /// Resolve GitHub repo path handling renames and transfers
    fn resolve_github_repo(repo_path: &str) -> Option<String> {
        use crate::http::HTTP_AGENT;

        // First try API (faster, works for most repos)
        let api_url = format!("https://api.github.com/repos/{}", repo_path);
        let response = HTTP_AGENT
            .get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "hoards-cli")
            .call()
            .ok()?;

        if response.status() == 200 {
            let json: serde_json::Value = response.into_body().read_json().ok()?;
            if let Some(name) = json["full_name"].as_str() {
                return Some(name.to_string());
            }
        }

        // API failed - manually check for redirect on the web URL
        let web_url = format!("https://github.com/{}", repo_path);

        // Use a new agent without redirect following
        let no_redirect_agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(5)))
            .max_redirects(0)
            .build()
            .new_agent();

        if let Ok(response) = no_redirect_agent
            .get(&web_url)
            .header("User-Agent", "hoards-cli")
            .call()
        {
            let status = response.status();
            if (status == 301 || status == 302)
                && let Some(location) = response.headers().get("location")
                && let Ok(location_str) = location.to_str()
                && let Some(path) = location_str.strip_prefix("https://github.com/")
            {
                let clean_path = path.trim_end_matches('/');
                if clean_path.matches('/').count() == 1 {
                    return Some(clean_path.to_string());
                }
            }
        }

        None
    }

    /// Get the default branch for a GitHub repo
    fn get_github_default_branch(repo_path: &str) -> Option<String> {
        use crate::http::HTTP_AGENT;

        let api_url = format!("https://api.github.com/repos/{}", repo_path);
        let response = HTTP_AGENT
            .get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "hoards-cli")
            .call()
            .ok()?;

        if response.status() != 200 {
            return None;
        }

        let json: serde_json::Value = response.into_body().read_json().ok()?;
        json["default_branch"].as_str().map(String::from)
    }

    /// Fetch README content from URL
    fn fetch_readme(url: &str) -> anyhow::Result<String> {
        use crate::http::HTTP_AGENT;

        let response = HTTP_AGENT
            .get(url)
            .call()
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if response.status() != 200 {
            // Try lowercase readme.md
            let alt_url = url.replace("README.md", "readme.md");
            let response = HTTP_AGENT
                .get(&alt_url)
                .call()
                .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

            if response.status() != 200 {
                anyhow::bail!("README not found (status {})", response.status());
            }

            return response
                .into_body()
                .read_to_string()
                .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e));
        }

        response
            .into_body()
            .read_to_string()
            .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e))
    }
}
