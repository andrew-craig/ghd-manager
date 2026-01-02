use serde::{Deserialize, Serialize};
use crate::error::{MonitorError, Result};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthenticationConfig,
    pub git: GitConfig,
    pub docker: DockerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Dashboard password (plaintext - will be hashed on load)
    pub password: String,
    /// Session timeout in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_session_timeout")]
    pub session_timeout: i64,
}

fn default_session_timeout() -> i64 {
    3600 // 1 hour
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Local repository path to manage
    pub repo_path: String,
    /// Git remote name (typically "origin")
    #[serde(default = "default_git_remote")]
    pub remote: String,
    /// Branch to track
    #[serde(default = "default_git_branch")]
    pub branch: String,
}

fn default_git_remote() -> String {
    "origin".to_string()
}

fn default_git_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Path to docker-compose.yml file
    pub compose_file: String,
    /// List of container names to manage
    pub containers: Vec<String>,
    /// Docker socket path (default: unix:///var/run/docker.sock)
    #[serde(default = "default_docker_socket")]
    pub socket: String,
}

fn default_docker_socket() -> String {
    "unix:///var/run/docker.sock".to_string()
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok(); // Load .env file if present (ignore errors if not found)

        let config = Config {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "3000".to_string())
                    .parse()
                    .map_err(|e| MonitorError::Config(format!("Invalid SERVER_PORT: {}", e)))?,
            },
            auth: AuthenticationConfig {
                password: env::var("DASHBOARD_PASSWORD")
                    .map_err(|_| MonitorError::Config("DASHBOARD_PASSWORD must be set in environment".to_string()))?,
                session_timeout: env::var("SESSION_TIMEOUT")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .map_err(|e| MonitorError::Config(format!("Invalid SESSION_TIMEOUT: {}", e)))?,
            },
            git: GitConfig {
                repo_path: env::var("GIT_REPO_PATH")
                    .map_err(|_| MonitorError::Config("GIT_REPO_PATH must be set in environment".to_string()))?,
                remote: env::var("GIT_REMOTE").unwrap_or_else(|_| "origin".to_string()),
                branch: env::var("GIT_BRANCH").unwrap_or_else(|_| "main".to_string()),
            },
            docker: DockerConfig {
                compose_file: env::var("DOCKER_COMPOSE_FILE")
                    .map_err(|_| MonitorError::Config("DOCKER_COMPOSE_FILE must be set in environment".to_string()))?,
                containers: env::var("DOCKER_CONTAINERS")
                    .map_err(|_| MonitorError::Config("DOCKER_CONTAINERS must be set in environment".to_string()))?
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                socket: env::var("DOCKER_SOCKET").unwrap_or_else(|_| "unix:///var/run/docker.sock".to_string()),
            },
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        // Validate server port
        if self.server.port == 0 {
            return Err(MonitorError::Config("Server port must be greater than 0".to_string()));
        }

        // Validate password is not empty
        if self.auth.password.trim().is_empty() {
            return Err(MonitorError::Config("Dashboard password cannot be empty".to_string()));
        }

        // Validate session timeout
        if self.auth.session_timeout <= 0 {
            return Err(MonitorError::Config("Session timeout must be greater than 0".to_string()));
        }

        // Validate git repo path exists
        if !std::path::Path::new(&self.git.repo_path).exists() {
            return Err(MonitorError::Config(format!(
                "Git repository path does not exist: {}",
                self.git.repo_path
            )));
        }

        // Validate docker compose file exists
        if !std::path::Path::new(&self.docker.compose_file).exists() {
            return Err(MonitorError::Config(format!(
                "Docker compose file does not exist: {}",
                self.docker.compose_file
            )));
        }

        // Validate at least one container is specified
        if self.docker.containers.is_empty() {
            return Err(MonitorError::Config("At least one Docker container must be specified".to_string()));
        }

        Ok(())
    }
}
