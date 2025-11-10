use reqwest::{Client, header};
use serde::Deserialize;
use crate::error::{Result, MonitorError};

#[derive(Debug, Deserialize)]
struct GitHubFileResponse {
    content: String,
    encoding: String,
}

pub struct GitHubClient {
    client: Client,
    api_token: String,
    repository_url: String,
    owner: String,
    repo: String,
}

impl GitHubClient {
    pub fn new(api_token: String, repository_url: String) -> Result<Self> {
        let (owner, repo) = Self::parse_repository_url(&repository_url)?;

        Ok(Self {
            client: Client::new(),
            api_token,
            repository_url,
            owner,
            repo,
        })
    }

    /// Parse GitHub repository URL to extract owner and repository name
    /// Supports formats:
    /// - https://github.com/owner/repo
    /// - https://github.com/owner/repo.git
    /// - git@github.com:owner/repo.git
    fn parse_repository_url(url: &str) -> Result<(String, String)> {
        let url = url.trim();

        // Handle SSH format: git@github.com:owner/repo.git
        if url.starts_with("git@github.com:") {
            let path = url.strip_prefix("git@github.com:")
                .ok_or_else(|| MonitorError::Config("Invalid SSH repository URL".to_string()))?;
            let path = path.strip_suffix(".git").unwrap_or(path);
            let parts: Vec<&str> = path.split('/').collect();

            if parts.len() != 2 {
                return Err(MonitorError::Config(
                    format!("Invalid repository URL format: {}", url)
                ));
            }

            return Ok((parts[0].to_string(), parts[1].to_string()));
        }

        // Handle HTTPS format: https://github.com/owner/repo
        if url.starts_with("https://github.com/") || url.starts_with("http://github.com/") {
            let path = url.strip_prefix("https://github.com/")
                .or_else(|| url.strip_prefix("http://github.com/"))
                .ok_or_else(|| MonitorError::Config("Invalid HTTPS repository URL".to_string()))?;
            let path = path.strip_suffix(".git").unwrap_or(path);
            let parts: Vec<&str> = path.split('/').collect();

            if parts.len() != 2 {
                return Err(MonitorError::Config(
                    format!("Invalid repository URL format: {}", url)
                ));
            }

            return Ok((parts[0].to_string(), parts[1].to_string()));
        }

        Err(MonitorError::Config(
            format!("Unsupported repository URL format: {}", url)
        ))
    }

    /// Fetch a single file from the GitHub repository
    ///
    /// # Arguments
    /// * `file_path` - Path to the file within the repository (e.g., "src/main.py")
    /// * `branch` - Branch name or commit SHA to fetch from
    ///
    /// # Returns
    /// The decoded content of the file as a String
    pub async fn fetch_file(&self, file_path: &str, branch: &str) -> Result<String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            self.owner, self.repo, file_path
        );

        let response = self.client
            .get(&url)
            .header(header::USER_AGENT, "github-monitor")
            .header(header::AUTHORIZATION, format!("Bearer {}", self.api_token))
            .header(header::ACCEPT, "application/vnd.github.v3+json")
            .query(&[("ref", branch)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MonitorError::GitHubApi(
                format!("Failed to fetch file '{}': {} - {}", file_path, status, error_text)
            ));
        }

        let file_data: GitHubFileResponse = response.json().await?;

        // GitHub API returns base64 encoded content
        if file_data.encoding != "base64" {
            return Err(MonitorError::GitHubApi(
                format!("Unexpected encoding: {}", file_data.encoding)
            ));
        }

        // Decode base64 content
        let content = file_data.content.replace('\n', "").replace('\r', "");
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &content
        ).map_err(|e| MonitorError::GitHubApi(
            format!("Failed to decode base64 content: {}", e)
        ))?;

        String::from_utf8(decoded).map_err(|e| MonitorError::GitHubApi(
            format!("Failed to convert file content to UTF-8: {}", e)
        ))
    }

    /// Fetch multiple files from the GitHub repository
    ///
    /// # Arguments
    /// * `file_paths` - List of file paths within the repository
    /// * `branch` - Branch name or commit SHA to fetch from
    ///
    /// # Returns
    /// A vector of tuples containing (file_path, file_content)
    pub async fn fetch_multiple_files(&self, file_paths: &[String], branch: &str) -> Result<Vec<(String, String)>> {
        let mut results = Vec::new();

        for file_path in file_paths {
            match self.fetch_file(file_path, branch).await {
                Ok(content) => {
                    results.push((file_path.clone(), content));
                }
                Err(e) => {
                    tracing::error!("Failed to fetch file '{}': {}", file_path, e);
                    // Continue with other files but log the error
                    // You could also choose to fail fast by returning the error here
                    return Err(e);
                }
            }
        }

        Ok(results)
    }

    /// Get the repository owner name
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Get the repository name
    pub fn repo(&self) -> &str {
        &self.repo
    }
}
