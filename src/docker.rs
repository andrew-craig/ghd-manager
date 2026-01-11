use crate::error::{MonitorError, Result};
use bollard::container::{InspectContainerOptions, RestartContainerOptions, StartContainerOptions, StopContainerOptions};
use bollard::Docker;
use bollard::models::ContainerStateStatusEnum;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use tracing::{debug, error, info, warn};

/// Represents the status of a Docker container
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Paused,
    Restarting,
    Dead,
    Created,
    Removing,
    Unknown,
}

impl From<ContainerStateStatusEnum> for ContainerStatus {
    fn from(status: ContainerStateStatusEnum) -> Self {
        match status {
            ContainerStateStatusEnum::RUNNING => ContainerStatus::Running,
            ContainerStateStatusEnum::PAUSED => ContainerStatus::Paused,
            ContainerStateStatusEnum::RESTARTING => ContainerStatus::Restarting,
            ContainerStateStatusEnum::DEAD => ContainerStatus::Dead,
            ContainerStateStatusEnum::CREATED => ContainerStatus::Created,
            ContainerStateStatusEnum::EXITED => ContainerStatus::Stopped,
            ContainerStateStatusEnum::REMOVING => ContainerStatus::Removing,
            ContainerStateStatusEnum::EMPTY => ContainerStatus::Unknown,
        }
    }
}

impl std::fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerStatus::Running => write!(f, "running"),
            ContainerStatus::Stopped => write!(f, "stopped"),
            ContainerStatus::Paused => write!(f, "paused"),
            ContainerStatus::Restarting => write!(f, "restarting"),
            ContainerStatus::Dead => write!(f, "dead"),
            ContainerStatus::Created => write!(f, "created"),
            ContainerStatus::Removing => write!(f, "removing"),
            ContainerStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Information about a container
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub status: ContainerStatus,
    pub image: String,
}

/// Result of an update operation
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Manages Docker containers and compose operations
#[derive(Clone)]
pub struct DockerManager {
    docker: Docker,
    compose_file_path: String,
    compose_dir: String,
    container_names: Vec<String>,
}

impl DockerManager {
    /// Creates a new DockerManager instance
    ///
    /// # Arguments
    /// * `compose_file_path` - Path to the docker-compose.yml file
    /// * `container_names` - List of container names to manage
    pub fn new(compose_file_path: impl Into<String>, container_names: Vec<String>) -> Result<Self> {
        let compose_file_path = compose_file_path.into();

        // Extract directory from compose file path
        let compose_path = Path::new(&compose_file_path);
        let compose_dir = compose_path
            .parent()
            .ok_or_else(|| MonitorError::Docker("Invalid compose file path".to_string()))?
            .to_string_lossy()
            .to_string();

        // Connect to Docker daemon
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| MonitorError::Docker(format!("Failed to connect to Docker daemon: {}", e)))?;

        Ok(Self {
            docker,
            compose_file_path,
            compose_dir,
            container_names,
        })
    }

    /// Validates that the compose file exists and containers are known to Docker
    pub async fn validate(&self) -> Result<()> {
        info!("Validating Docker configuration");

        // Check if compose file exists
        let compose_path = Path::new(&self.compose_file_path);
        if !compose_path.exists() {
            return Err(MonitorError::Docker(format!(
                "Docker compose file not found: {}",
                self.compose_file_path
            )));
        }

        // Validate Docker connection by listing containers
        let containers = self.docker
            .list_containers::<String>(None)
            .await
            .map_err(|e| MonitorError::Docker(format!("Failed to connect to Docker daemon: {}", e)))?;

        debug!("Found {} total containers", containers.len());

        // Check if configured container names exist
        let mut container_map: HashMap<String, bool> = HashMap::new();
        for container in &containers {
            if let Some(names) = &container.names {
                for name in names {
                    let clean_name = name.trim_start_matches('/').to_string();
                    container_map.insert(clean_name, true);
                }
            }
        }

        let mut missing_containers = Vec::new();
        for name in &self.container_names {
            if !container_map.contains_key(name) {
                missing_containers.push(name.clone());
            }
        }

        if !missing_containers.is_empty() {
            warn!(
                "Some configured containers not found in Docker: {:?}",
                missing_containers
            );
            warn!("These containers may not be created yet. They will be available after first compose up.");
        }

        info!("Docker validation completed successfully");
        Ok(())
    }

    /// Gets the status of a specific container
    pub async fn get_container_status(&self, container_name: &str) -> Result<ContainerInfo> {
        debug!("Getting status for container: {}", container_name);

        let inspect = self.docker
            .inspect_container(container_name, None::<InspectContainerOptions>)
            .await
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to inspect container '{}': {}",
                container_name, e
            )))?;

        let state = inspect.state.ok_or_else(|| {
            MonitorError::Docker(format!("Container '{}' has no state", container_name))
        })?;

        let status = state.status.unwrap_or(ContainerStateStatusEnum::EMPTY).into();

        let info = ContainerInfo {
            name: inspect.name.unwrap_or_default().trim_start_matches('/').to_string(),
            status,
            image: inspect.image.unwrap_or_default(),
        };

        debug!("Container {} status: {}", container_name, info.status);
        Ok(info)
    }

    /// Gets the status of all managed containers
    pub async fn get_all_container_status(&self) -> Result<Vec<ContainerInfo>> {
        debug!("Getting status for all managed containers");

        let mut infos = Vec::new();
        for name in &self.container_names {
            match self.get_container_status(name).await {
                Ok(info) => infos.push(info),
                Err(e) => {
                    warn!("Failed to get status for container '{}': {}", name, e);
                    // Continue with other containers even if one fails
                }
            }
        }

        Ok(infos)
    }

    /// Starts a specific container
    pub async fn start_container(&self, container_name: &str) -> Result<()> {
        info!("Starting container: {}", container_name);

        self.docker
            .start_container(container_name, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to start container '{}': {}",
                container_name, e
            )))?;

        info!("Successfully started container: {}", container_name);
        Ok(())
    }

    /// Stops a specific container
    pub async fn stop_container(&self, container_name: &str) -> Result<()> {
        info!("Stopping container: {}", container_name);

        // Give container 10 seconds to gracefully stop before force kill
        let options = StopContainerOptions { t: 10 };

        self.docker
            .stop_container(container_name, Some(options))
            .await
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to stop container '{}': {}",
                container_name, e
            )))?;

        info!("Successfully stopped container: {}", container_name);
        Ok(())
    }

    /// Restarts a specific container
    pub async fn restart_container(&self, container_name: &str) -> Result<()> {
        info!("Restarting container: {}", container_name);

        // Give container 10 seconds to gracefully stop before force kill
        let options = RestartContainerOptions { t: 10 };

        self.docker
            .restart_container(container_name, Some(options))
            .await
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to restart container '{}': {}",
                container_name, e
            )))?;

        info!("Successfully restarted container: {}", container_name);
        Ok(())
    }

    /// Starts all managed containers
    pub async fn start_all_containers(&self) -> Result<()> {
        info!("Starting all managed containers");

        for name in &self.container_names {
            if let Err(e) = self.start_container(name).await {
                error!("Failed to start container '{}': {}", name, e);
                return Err(e);
            }
        }

        info!("Successfully started all containers");
        Ok(())
    }

    /// Stops all managed containers
    pub async fn stop_all_containers(&self) -> Result<()> {
        info!("Stopping all managed containers");

        for name in &self.container_names {
            if let Err(e) = self.stop_container(name).await {
                error!("Failed to stop container '{}': {}", name, e);
                return Err(e);
            }
        }

        info!("Successfully stopped all containers");
        Ok(())
    }

    /// Restarts all managed containers
    pub async fn restart_all_containers(&self) -> Result<()> {
        info!("Restarting all managed containers");

        for name in &self.container_names {
            if let Err(e) = self.restart_container(name).await {
                error!("Failed to restart container '{}': {}", name, e);
                return Err(e);
            }
        }

        info!("Successfully restarted all containers");
        Ok(())
    }

    /// Pulls and restarts a single container using docker-compose
    pub async fn update_container(&self, container_name: &str) -> Result<UpdateResult> {
        info!("Pulling and restarting container: {}", container_name);

        // First, stop the container
        if let Err(e) = self.stop_container(container_name).await {
            warn!("Failed to stop container before pull: {}", e);
        }

        // Step 1: Pull the latest image for the specific service
        let pull_output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(&self.compose_file_path)
            .arg("pull")
            .arg(container_name)
            .current_dir(&self.compose_dir)
            .output()
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to execute docker compose pull: {}",
                e
            )))?;

        let mut combined_output = String::from_utf8_lossy(&pull_output.stdout).to_string();
        let pull_stderr = String::from_utf8_lossy(&pull_output.stderr).to_string();

        if !pull_output.status.success() {
            error!("Docker compose pull failed for '{}': {}", container_name, pull_stderr);
            return Ok(UpdateResult {
                success: false,
                output: combined_output,
                error: Some(pull_stderr),
            });
        }

        // Step 2: Start the container with the new image (without building)
        let up_output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(&self.compose_file_path)
            .arg("up")
            .arg("-d")
            .arg(container_name)
            .current_dir(&self.compose_dir)
            .output()
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to execute docker compose up: {}",
                e
            )))?;

        let up_stdout = String::from_utf8_lossy(&up_output.stdout).to_string();
        let up_stderr = String::from_utf8_lossy(&up_output.stderr).to_string();

        combined_output.push_str("\n");
        combined_output.push_str(&up_stdout);

        if !up_output.status.success() {
            error!("Docker compose up failed for '{}': {}", container_name, up_stderr);
            return Ok(UpdateResult {
                success: false,
                output: combined_output,
                error: Some(up_stderr),
            });
        }

        info!("Successfully pulled and restarted container: {}", container_name);
        Ok(UpdateResult {
            success: true,
            output: combined_output,
            error: None,
        })
    }

    /// Pulls and restarts all containers using docker-compose
    pub async fn update_all_containers(&self) -> Result<UpdateResult> {
        info!("Pulling and restarting all containers");

        // Execute: docker compose down && docker compose pull && docker compose up -d

        // Step 1: Down
        let down_output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(&self.compose_file_path)
            .arg("down")
            .current_dir(&self.compose_dir)
            .output()
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to execute docker compose down: {}",
                e
            )))?;

        if !down_output.status.success() {
            let stderr = String::from_utf8_lossy(&down_output.stderr).to_string();
            error!("Docker compose down failed: {}", stderr);
            return Ok(UpdateResult {
                success: false,
                output: String::from_utf8_lossy(&down_output.stdout).to_string(),
                error: Some(stderr),
            });
        }

        debug!("Docker compose down completed");

        // Step 2: Pull latest images
        let pull_output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(&self.compose_file_path)
            .arg("pull")
            .current_dir(&self.compose_dir)
            .output()
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to execute docker compose pull: {}",
                e
            )))?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr).to_string();
            error!("Docker compose pull failed: {}", stderr);
            return Ok(UpdateResult {
                success: false,
                output: format!("{}\n{}",
                    String::from_utf8_lossy(&down_output.stdout),
                    String::from_utf8_lossy(&pull_output.stdout)
                ),
                error: Some(stderr),
            });
        }

        debug!("Docker compose pull completed");

        // Step 3: Up without build
        let up_output = Command::new("docker")
            .arg("compose")
            .arg("-f")
            .arg(&self.compose_file_path)
            .arg("up")
            .arg("-d")
            .current_dir(&self.compose_dir)
            .output()
            .map_err(|e| MonitorError::Docker(format!(
                "Failed to execute docker compose up: {}",
                e
            )))?;

        let stdout = String::from_utf8_lossy(&up_output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&up_output.stderr).to_string();

        if !up_output.status.success() {
            error!("Docker compose up failed: {}", stderr);
            return Ok(UpdateResult {
                success: false,
                output: format!("{}\n{}\n{}",
                    String::from_utf8_lossy(&down_output.stdout),
                    String::from_utf8_lossy(&pull_output.stdout),
                    stdout
                ),
                error: Some(stderr),
            });
        }

        info!("Successfully pulled and restarted all containers");
        Ok(UpdateResult {
            success: true,
            output: format!("{}\n{}\n{}",
                String::from_utf8_lossy(&down_output.stdout),
                String::from_utf8_lossy(&pull_output.stdout),
                stdout
            ),
            error: None,
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_display() {
        assert_eq!(ContainerStatus::Running.to_string(), "running");
        assert_eq!(ContainerStatus::Stopped.to_string(), "stopped");
        assert_eq!(ContainerStatus::Paused.to_string(), "paused");
    }

    #[test]
    fn test_container_status_from_enum() {
        let status: ContainerStatus = ContainerStateStatusEnum::RUNNING.into();
        assert_eq!(status, ContainerStatus::Running);

        let status: ContainerStatus = ContainerStateStatusEnum::EXITED.into();
        assert_eq!(status, ContainerStatus::Stopped);
    }
}
