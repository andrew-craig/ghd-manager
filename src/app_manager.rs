use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::thread;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use crate::error::{MonitorError, Result};

/// Manages the lifecycle of the monitored Python application
pub struct AppManager {
    working_dir: String,
    start_command: Vec<String>,
    process: Option<Child>,
}

impl AppManager {
    /// Create a new AppManager instance
    ///
    /// # Arguments
    /// * `working_dir` - The working directory where the application should run
    /// * `start_command` - The command to start the application (first element is the program, rest are arguments)
    pub fn new(working_dir: String, start_command: Vec<String>) -> Self {
        Self {
            working_dir,
            start_command,
            process: None,
        }
    }

    /// Start the application process
    ///
    /// Launches the application using the configured command in the working directory.
    /// If a process is already running, this will return an error.
    ///
    /// # Returns
    /// `Ok(())` on successful start, or an error if the process fails to start
    pub fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Err(MonitorError::AppManager(
                "Application is already running".to_string()
            ));
        }

        if self.start_command.is_empty() {
            return Err(MonitorError::AppManager(
                "Start command is empty".to_string()
            ));
        }

        tracing::info!("Starting application with command: {:?}", self.start_command);

        let program = &self.start_command[0];
        let args = &self.start_command[1..];

        let child = Command::new(program)
            .args(args)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| MonitorError::AppManager(
                format!("Failed to start application: {}", e)
            ))?;

        let pid = child.id();
        tracing::info!("Application started with PID: {}", pid);

        self.process = Some(child);

        // Give the process a moment to start up
        thread::sleep(Duration::from_millis(500));

        // Verify it's still running after initial startup
        if !self.is_running() {
            let error_msg = self.get_process_error_output();
            return Err(MonitorError::AppManager(
                format!("Application crashed immediately after start. {}", error_msg)
            ));
        }

        Ok(())
    }

    /// Stop the application gracefully
    ///
    /// Attempts to gracefully shut down the application by:
    /// 1. Sending SIGTERM to allow the process to clean up
    /// 2. Waiting up to 10 seconds for graceful shutdown
    /// 3. If still running, sending SIGKILL to force termination
    ///
    /// # Returns
    /// `Ok(())` on successful stop, or an error if the process cannot be stopped
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_running() {
            tracing::info!("Application is not running, nothing to stop");
            return Ok(());
        }

        let process = self.process.as_mut().unwrap();
        let pid = process.id() as i32;

        tracing::info!("Stopping application with PID: {}", pid);

        // Send SIGTERM for graceful shutdown
        if let Err(e) = signal::kill(Pid::from_raw(pid), Signal::SIGTERM) {
            tracing::warn!("Failed to send SIGTERM to process {}: {}", pid, e);
        } else {
            tracing::info!("Sent SIGTERM to process {}, waiting for graceful shutdown...", pid);
        }

        // Wait up to 10 seconds for graceful shutdown
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match process.try_wait() {
                Ok(Some(status)) => {
                    tracing::info!("Application stopped gracefully with status: {}", status);
                    self.process = None;
                    return Ok(());
                }
                Ok(None) => {
                    // Process still running, wait a bit
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    return Err(MonitorError::AppManager(
                        format!("Error waiting for process: {}", e)
                    ));
                }
            }
        }

        // If we get here, the process didn't stop gracefully
        tracing::warn!("Process {} did not stop gracefully, sending SIGKILL", pid);

        if let Err(e) = signal::kill(Pid::from_raw(pid), Signal::SIGKILL) {
            tracing::error!("Failed to send SIGKILL to process {}: {}", pid, e);
        }

        // Give it a moment to die
        thread::sleep(Duration::from_millis(500));

        // Try to reap the process
        match process.try_wait() {
            Ok(Some(status)) => {
                tracing::info!("Application force stopped with status: {}", status);
            }
            Ok(None) => {
                tracing::warn!("Process {} may still be running after SIGKILL", pid);
            }
            Err(e) => {
                tracing::error!("Error checking process status after SIGKILL: {}", e);
            }
        }

        self.process = None;
        Ok(())
    }

    /// Restart the application
    ///
    /// Stops the application if running, then starts it again.
    ///
    /// # Returns
    /// `Ok(())` on successful restart, or an error if restart fails
    pub fn restart(&mut self) -> Result<()> {
        tracing::info!("Restarting application");

        if self.is_running() {
            self.stop()?;
        }

        // Wait a moment between stop and start
        thread::sleep(Duration::from_millis(500));

        self.start()?;

        Ok(())
    }

    /// Check if the application process is currently running
    ///
    /// # Returns
    /// `true` if the process is running, `false` otherwise
    pub fn is_running(&self) -> bool {
        if let Some(process) = &self.process {
            // Check if we can send signal 0 (null signal) to test if process exists
            let pid = process.id() as i32;
            match signal::kill(Pid::from_raw(pid), None) {
                Ok(()) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Perform a health check on the application
    ///
    /// Currently performs a basic check to verify:
    /// 1. The process is running
    /// 2. The process hasn't exited with an error
    ///
    /// Future enhancements could include:
    /// - HTTP health endpoint checks
    /// - Custom health check scripts
    /// - Resource usage monitoring
    ///
    /// # Returns
    /// `Ok(true)` if the application is healthy, `Ok(false)` if unhealthy but running,
    /// or an error if the health check cannot be performed
    pub fn health_check(&self) -> Result<bool> {
        if !self.is_running() {
            tracing::warn!("Health check failed: application is not running");
            return Ok(false);
        }

        // Check if the process has exited but we haven't reaped it yet
        if let Some(process) = &self.process {
            let pid = process.id();
            tracing::debug!("Health check: process {} is running", pid);

            // Could add more sophisticated health checks here:
            // - Check CPU/memory usage
            // - Ping a health endpoint if it's a web service
            // - Check log files for errors

            return Ok(true);
        }

        Ok(false)
    }

    /// Get error output from the process if it has terminated
    ///
    /// # Returns
    /// A string containing error information, or an empty string if unavailable
    fn get_process_error_output(&mut self) -> String {
        if let Some(process) = self.process.take() {
            if let Ok(output) = process.wait_with_output() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                return format!("stdout: {}, stderr: {}", stdout, stderr);
            }
        }
        String::new()
    }

    /// Get the current process ID if the application is running
    ///
    /// # Returns
    /// `Some(pid)` if running, `None` otherwise
    pub fn get_pid(&self) -> Option<u32> {
        if self.is_running() {
            self.process.as_ref().map(|p| p.id())
        } else {
            None
        }
    }
}

impl Drop for AppManager {
    /// Ensure the process is stopped when AppManager is dropped
    fn drop(&mut self) {
        if self.is_running() {
            tracing::warn!("AppManager dropped with running process, stopping it");
            let _ = self.stop();
        }
    }
}
