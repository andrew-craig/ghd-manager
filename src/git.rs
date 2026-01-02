use crate::error::{MonitorError, Result};
use std::path::Path;
use std::process::Command;
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct GitManager {
    repo_path: String,
    remote: String,
    branch: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GitStatus {
    pub local_commit: String,
    pub remote_commit: String,
    pub updates_available: bool,
    pub current_branch: String,
}

impl GitManager {
    pub fn new(repo_path: impl Into<String>, remote: impl Into<String>, branch: impl Into<String>) -> Self {
        Self {
            repo_path: repo_path.into(),
            remote: remote.into(),
            branch: branch.into(),
        }
    }

    pub fn fetch(&self) -> Result<()> {
        info!("Fetching updates from {}/{}", self.remote, self.branch);

        let output = Command::new("git")
            .arg("fetch")
            .arg(&self.remote)
            .arg(&self.branch)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| MonitorError::Git(format!("Failed to execute git fetch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Git fetch failed: {}", stderr);
            return Err(MonitorError::Git(format!("Git fetch failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Git fetch output: {}", stdout);
        info!("Successfully fetched from {}/{}", self.remote, self.branch);

        Ok(())
    }

    pub fn pull(&self) -> Result<PullResult> {
        info!("Pulling updates from {}/{}", self.remote, self.branch);

        let output = Command::new("git")
            .arg("pull")
            .arg("--ff-only")
            .arg(&self.remote)
            .arg(&self.branch)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| MonitorError::Git(format!("Failed to execute git pull: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Git pull failed: {}", stderr);
            return Err(MonitorError::Git(format!("Git pull failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("Git pull output: {}", stdout);

        let already_up_to_date = stdout.contains("Already up to date") || stdout.contains("Already up-to-date");
        let files_changed = self.parse_files_changed(&stdout);

        let result = PullResult {
            already_up_to_date,
            files_changed,
            output: stdout,
        };

        info!("Pull completed: {} files changed", result.files_changed);
        Ok(result)
    }

    pub fn get_status(&self) -> Result<GitStatus> {
        debug!("Getting git status for repository at {}", self.repo_path);

        let local_commit = self.get_local_commit()?;
        let remote_commit = self.get_remote_commit()?;
        let current_branch = self.get_current_branch()?;

        let updates_available = local_commit != remote_commit;

        let status = GitStatus {
            local_commit,
            remote_commit,
            updates_available,
            current_branch,
        };

        debug!("Git status: {:?}", status);
        Ok(status)
    }

    fn get_local_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| MonitorError::Git(format!("Failed to get local commit: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MonitorError::Git(format!("Failed to get local commit: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_remote_commit(&self) -> Result<String> {
        let remote_ref = format!("{}/{}", self.remote, self.branch);

        let output = Command::new("git")
            .arg("rev-parse")
            .arg(&remote_ref)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| MonitorError::Git(format!("Failed to get remote commit: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MonitorError::Git(format!("Failed to get remote commit for {}: {}", remote_ref, stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| MonitorError::Git(format!("Failed to get current branch: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MonitorError::Git(format!("Failed to get current branch: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn parse_files_changed(&self, output: &str) -> usize {
        for line in output.lines() {
            if line.contains("file changed") || line.contains("files changed") {
                if let Some(num_str) = line.split_whitespace().next() {
                    if let Ok(num) = num_str.parse::<usize>() {
                        return num;
                    }
                }
            }
        }
        0
    }

    pub fn validate_repository(&self) -> Result<()> {
        let repo_path = Path::new(&self.repo_path);

        if !repo_path.exists() {
            return Err(MonitorError::Git(format!("Repository path does not exist: {}", self.repo_path)));
        }

        let git_dir = repo_path.join(".git");
        if !git_dir.exists() {
            return Err(MonitorError::Git(format!("Not a git repository: {}", self.repo_path)));
        }

        let output = Command::new("git")
            .arg("remote")
            .arg("get-url")
            .arg(&self.remote)
            .current_dir(&self.repo_path)
            .output()
            .map_err(|e| MonitorError::Git(format!("Failed to validate remote: {}", e)))?;

        if !output.status.success() {
            return Err(MonitorError::Git(format!("Remote '{}' not found in repository", self.remote)));
        }

        info!("Repository validation successful: {}", self.repo_path);
        Ok(())
    }

}

#[derive(Debug, Clone)]
pub struct PullResult {
    pub already_up_to_date: bool,
    pub files_changed: usize,
    pub output: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_files_changed() {
        let manager = GitManager::new("/tmp", "origin", "main");

        let output1 = " 3 files changed, 42 insertions(+), 7 deletions(-)";
        assert_eq!(manager.parse_files_changed(output1), 3);

        let output2 = " 1 file changed, 5 insertions(+)";
        assert_eq!(manager.parse_files_changed(output2), 1);

        let output3 = "Already up to date.";
        assert_eq!(manager.parse_files_changed(output3), 0);
    }
}
