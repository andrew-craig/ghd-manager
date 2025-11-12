use std::path::{Path, PathBuf};
use std::process::Command;
use crate::error::{MonitorError, Result};

/// Manages Python package dependencies using the uv package manager
pub struct PackageManager {
    venv_path: Option<String>,
    working_dir: String,
}

impl PackageManager {
    /// Create a new PackageManager instance
    ///
    /// # Arguments
    /// * `venv_path` - Optional path to Python virtual environment
    /// * `working_dir` - Working directory where the application resides
    pub fn new(venv_path: Option<String>, working_dir: String) -> Self {
        Self {
            venv_path,
            working_dir,
        }
    }

    /// Detect if any dependency files were changed
    ///
    /// Checks if the changed files list includes common Python dependency files:
    /// - requirements.txt
    /// - pyproject.toml
    /// - setup.py
    /// - setup.cfg
    /// - Pipfile
    ///
    /// # Arguments
    /// * `changed_files` - List of file paths that were changed
    ///
    /// # Returns
    /// `true` if any dependency files were modified, `false` otherwise
    pub fn detect_dependency_changes(&self, changed_files: &[String]) -> Result<bool> {
        tracing::info!("Checking for dependency file changes in {} files", changed_files.len());

        let dependency_files = [
            "requirements.txt",
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "Pipfile",
            "Pipfile.lock",
        ];

        for file in changed_files {
            let file_name = Path::new(file).file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if dependency_files.contains(&file_name) {
                tracing::info!("Dependency file detected: {}", file);
                return Ok(true);
            }

            // Also check if it's requirements*.txt (e.g., requirements-dev.txt)
            if file_name.starts_with("requirements") && file_name.ends_with(".txt") {
                tracing::info!("Dependency file detected: {}", file);
                return Ok(true);
            }
        }

        tracing::info!("No dependency file changes detected");
        Ok(false)
    }

    /// Update packages using uv sync
    ///
    /// This method runs `uv sync` to synchronize the virtual environment with the
    /// dependency specifications. It will upgrade existing packages and install
    /// new ones as needed.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the operation fails
    pub async fn update_packages(&self) -> Result<()> {
        tracing::info!("Starting package update with uv sync");

        // Check if uv is available
        self.verify_uv_available()?;

        // Determine which dependency file to use
        let dep_file = self.find_dependency_file()?;
        tracing::info!("Using dependency file: {:?}", dep_file);

        // Run uv sync to synchronize packages
        let output = self.run_uv_command(&["sync"])?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::info!("Package sync completed successfully");
            tracing::debug!("UV sync output: {}", stdout);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::error!("UV sync failed. Stdout: {} Stderr: {}", stdout, stderr);
            Err(MonitorError::PackageManager(
                format!("Failed to sync packages: {}", stderr)
            ))
        }
    }

    /// Install packages from dependency files
    ///
    /// This method installs packages using `uv pip install`. It will attempt to
    /// install from requirements.txt or pyproject.toml as appropriate.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the operation fails
    pub async fn install_packages(&self) -> Result<()> {
        tracing::info!("Starting package installation with uv pip install");

        // Check if uv is available
        self.verify_uv_available()?;

        // Determine which dependency file to use
        let dep_file = self.find_dependency_file()?;
        tracing::info!("Installing from dependency file: {:?}", dep_file);

        let output = match dep_file {
            DependencyFile::RequirementsTxt => {
                // Use uv pip install -r requirements.txt
                self.run_uv_command(&["pip", "install", "-r", "requirements.txt"])?
            }
            DependencyFile::PyprojectToml => {
                // Use uv sync for pyproject.toml (it handles installation automatically)
                self.run_uv_command(&["sync"])?
            }
            DependencyFile::SetupPy => {
                // Install the package in editable mode
                self.run_uv_command(&["pip", "install", "-e", "."])?
            }
        };

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::info!("Package installation completed successfully");
            tracing::debug!("UV install output: {}", stdout);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::error!("UV install failed. Stdout: {} Stderr: {}", stdout, stderr);
            Err(MonitorError::PackageManager(
                format!("Failed to install packages: {}", stderr)
            ))
        }
    }

    /// Execute a uv command with proper environment setup
    ///
    /// # Arguments
    /// * `args` - Command arguments to pass to uv
    ///
    /// # Returns
    /// The output of the command execution
    fn run_uv_command(&self, args: &[&str]) -> Result<std::process::Output> {
        tracing::debug!("Running uv command: uv {}", args.join(" "));

        let mut cmd = Command::new("uv");
        cmd.args(args);
        cmd.current_dir(&self.working_dir);

        // Set up environment variables for virtual environment if specified
        if let Some(venv_path) = &self.venv_path {
            let venv_full_path = if Path::new(venv_path).is_absolute() {
                PathBuf::from(venv_path)
            } else {
                PathBuf::from(&self.working_dir).join(venv_path)
            };

            tracing::debug!("Using virtual environment: {:?}", venv_full_path);

            // Set VIRTUAL_ENV environment variable
            cmd.env("VIRTUAL_ENV", &venv_full_path);

            // Add venv bin directory to PATH
            let venv_bin = venv_full_path.join("bin");
            if let Ok(current_path) = std::env::var("PATH") {
                let new_path = format!("{}:{}", venv_bin.display(), current_path);
                cmd.env("PATH", new_path);
            }
        }

        // Execute the command
        let output = cmd.output()
            .map_err(|e| MonitorError::PackageManager(
                format!("Failed to execute uv command: {}", e)
            ))?;

        Ok(output)
    }

    /// Verify that uv is available on the system
    fn verify_uv_available(&self) -> Result<()> {
        let output = Command::new("uv")
            .arg("--version")
            .output()
            .map_err(|e| MonitorError::PackageManager(
                format!("UV package manager not found. Please install uv: {}", e)
            ))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            tracing::debug!("UV version: {}", version.trim());
            Ok(())
        } else {
            Err(MonitorError::PackageManager(
                "UV package manager is not properly installed".to_string()
            ))
        }
    }

    /// Find which dependency file exists in the working directory
    fn find_dependency_file(&self) -> Result<DependencyFile> {
        let working_path = Path::new(&self.working_dir);

        // Check for pyproject.toml first (modern standard)
        if working_path.join("pyproject.toml").exists() {
            return Ok(DependencyFile::PyprojectToml);
        }

        // Check for requirements.txt
        if working_path.join("requirements.txt").exists() {
            return Ok(DependencyFile::RequirementsTxt);
        }

        // Check for setup.py
        if working_path.join("setup.py").exists() {
            return Ok(DependencyFile::SetupPy);
        }

        Err(MonitorError::PackageManager(
            "No Python dependency file found (pyproject.toml, requirements.txt, or setup.py)".to_string()
        ))
    }

    /// Validate that packages were installed correctly
    ///
    /// This runs a simple check to ensure the Python environment can be accessed
    pub async fn validate_installation(&self) -> Result<()> {
        tracing::info!("Validating package installation");

        let output = self.run_uv_command(&["pip", "list"])?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            tracing::debug!("Installed packages:\n{}", stdout);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(MonitorError::PackageManager(
                format!("Failed to validate installation: {}", stderr)
            ))
        }
    }
}

/// Types of Python dependency files
#[derive(Debug, Clone, Copy)]
enum DependencyFile {
    RequirementsTxt,
    PyprojectToml,
    SetupPy,
}
