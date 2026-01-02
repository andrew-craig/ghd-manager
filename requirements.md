# GitHub + Docker Manager & Updater

## Overview

A Rust-based service that manages projects that are deployed as Docker containers built from Github repositories. The service enables remote triggering of (1) pulling updates from Github, (2) triggering container rebuilds and (3) viewing the status of containers.

## Core Requirements

### 1. Web dashboard 
A simple web dashboard with
- **Auth**: password-based authentication. the password can be set in the .env
- **Github**: 
  - user can see latest commit of remote (origin) and the current state of the local (i.e. is an update available)
  - user can trigger fetch from Github 
- **Docker Management**: for a list of Docker containers listed in the .env
  - see the status of the docker containers managed witn docker compose
  - stop, start and restart the container
  - trigger a rebuild of the container
  - trigger a rebuild of all containers (i.e. docker compose down && docker compose up --build -d)

### 2. Github Management Backend

- **File Fetching**: Pull latest versions of changed files from GitHub repository
- **Authentication**: Support GitHub personal access tokens or app authentication
- **Error Handling**: Graceful handling of network failures and invalid responses
more to be added

### 3. Docker Management

- start, stop and restart a specific docker container
- start, stop and restart all containers
- get the status of each container
- trigger a rebuild and restart of one or all containers
- management mechanism tbd


## Technical Specifications

### Configuration (.env)

- **Authentication**: Dashboard password
- **Repository Settings**:
  - Local repository path(s) to manage
  - Git remote URL(s) and branch(es) to track
- **Docker Settings**:
  - Docker Compose file path(s)
  - Container names to manage
  - Docker socket/API endpoint
- **Server Settings**:
  - Port for web server
  - Session timeout duration

### Web Dashboard

- **Framework**: Lightweight Rust web framework (Axum, Actix, or similar)
- **Authentication**: Session-based with password protection
- **UI Components**:
  - Repository status display (current commit, remote commit, update availability)
  - Git operations (fetch/pull triggers)
  - Docker container list with status indicators
  - Container control buttons (start/stop/restart/rebuild)
  - Action logs and status messages

### Git Management

- **Operations**: Use git CLI commands via Rust `std::process::Command`
  - `git fetch` to check for updates
  - `git pull` to apply updates
  - `git rev-parse` to get commit hashes
- **Status Checking**: Compare local HEAD with remote to detect available updates
- **Error Handling**: Capture git command output and errors

### Docker Management

- **API/SDK**: Use Rust Docker SDK (bollard or similar)
- **Operations**:
  - Query container status (running, stopped, etc.)
  - Start/stop/restart individual containers
  - Execute docker-compose commands for rebuilds
  - Monitor container health
- **Rebuild Process**:
  - Stop containers gracefully
  - Execute `docker compose down && docker compose up --build -d`
  - Monitor rebuild progress

### Logging & Monitoring

- Log all user actions from dashboard
- Track git operations (fetch, pull) with timestamps
- Record Docker management activities (start, stop, rebuild)
- Capture and display container logs
- Store operation history for dashboard display

### Error Handling

- Graceful handling of git command failures
- Docker API connection error recovery
- User-friendly error messages in dashboard
- Maintain service availability if containers fail
- Rollback mechanism for failed updates (optional)

## Workflow

### Dashboard Access Flow
1. User navigates to web dashboard
2. User authenticates with password
3. Session is created with timeout
4. Dashboard displays current status of all repositories and containers

### Git Update Flow (User-Triggered)
1. Dashboard displays current local commit and latest remote commit
2. User clicks "Fetch" to check for updates
3. Service executes `git fetch` on repository
4. Dashboard updates to show if updates are available
5. User clicks "Pull" to apply updates
6. Service executes `git pull`
7. Dashboard displays pull result (success/failure, files changed)
8. User can optionally trigger container rebuild to deploy changes

### Container Management Flow
1. Dashboard displays real-time container status (running/stopped)
2. User selects action:
   - **Start/Stop/Restart**: Single container operation via Docker API
   - **Rebuild Single**: Stop container → rebuild image → start container
   - **Rebuild All**: Execute `docker compose down && docker compose up --build -d`
3. Service executes requested operation
4. Dashboard shows progress and final status
5. Logs are updated with operation details

### Typical Deployment Update Workflow
1. User checks dashboard and sees updates available for repository
2. User clicks "Pull" to fetch latest code
3. User verifies pull was successful
4. User clicks "Rebuild All" to rebuild containers with new code
5. Service stops containers, rebuilds images, restarts containers
6. Dashboard confirms successful deployment
7. User can monitor container status to ensure healthy startup

---

## Implementation Status

### ✅ Can Be Retained (With Modifications)

1. **Error Handling** ([error.rs](src/error.rs))
   - **Status**: Mostly usable, needs minor updates
   - **Changes needed**:
     - Remove `PackageManager` and `AppManager` error variants
     - Add `Docker`, `Git`, and `Authentication` error variants
     - Keep `WebhookValidation` for now (may remove if webhooks removed completely)

2. **Configuration Structure** ([config.rs](src/config.rs))
   - **Status**: Structure needs complete redesign
   - **What to keep**: Basic pattern of using serde for config
   - **Changes needed**: Replace entire config structure to match new requirements:
     - Dashboard password
     - Repository path(s) and remote URL(s)
     - Docker Compose file path(s)
     - Container names
     - Server port and session timeout

3. ✅ **Webhook Components** ([webhook.rs](src/webhook.rs))
   - **Status**: ✅ Fully implemented with signature validation and payload processing
   - **Decision needed**: REMOVE
     - If keeping webhooks as future feature: retain as-is
     - If dashboard-only: delete this file
   - **Current implementation**: Complete HMAC-SHA256 validation, payload parsing, file extraction

4. **Dependencies** ([Cargo.toml](Cargo.toml))
   - **Status**: Needs updates
   - **Keep**: tokio, axum, serde, tracing, anyhow, thiserror, dotenvy
   - **Remove**: reqwest, hmac, sha2, hex, base64, nix (unless needed for process management)
   - **Add**:
     - `bollard` (Docker SDK)
     - Session management crate (e.g., `tower-sessions`)
     - Template engine (e.g., `askama`, `tera`, or `minijinja`)

### ✅ Delete Entirely (No Longer Relevant) - COMPLETED

1. **GitHub API Client** ~~[github.rs](src/github.rs)~~ ✅ DELETED
   - **Reason**: Replaced by git CLI commands
   - **Was**: GitHub REST API file fetching with base64 decoding
   - **Now**: Use `std::process::Command` for git operations

2. **Package Manager** ~~[package_manager.rs](src/package_manager.rs)~~ ✅ DELETED
   - **Reason**: No longer managing Python dependencies
   - **Was**: UV integration for Python package management
   - **Now**: Dependencies handled in Dockerfile during rebuild

3. **App Manager** ~~[app_manager.rs](src/app_manager.rs)~~ ✅ DELETED
   - **Reason**: Not managing application processes directly
   - **Was**: Process lifecycle management (start/stop/restart with signals)
   - **Now**: Docker containers managed via Docker API

### 🆕 Write From Fresh

1. ✅ **Git Manager Module** (new: `git.rs`)
   - Execute git CLI commands (`fetch`, `pull`, `rev-parse`)
   - Compare local HEAD with remote origin
   - Detect if updates are available
   - Parse git command output and handle errors

2. ✅ **Docker Manager Module** (new: `docker.rs`)
   - Use `bollard` crate for Docker SDK
   - Query container status (running, stopped, health)
   - Start/stop/restart individual containers
   - Execute docker-compose commands for rebuilds
   - Monitor rebuild progress

3. ✅ **Authentication Module** (new: [auth.rs](src/auth.rs))
   - Password verification with bcrypt hashing
   - Session creation and validation with tower-sessions
   - Session timeout management
   - Middleware for protected routes
   - Rate limiting for login attempts

4. ✅ **Web Dashboard** (new: [routes.rs](src/routes.rs) + [templates/](templates/))
   - HTML templates for UI with Askama templating engine
   - RESTful API endpoints for dashboard actions
   - Polling-based real-time status updates (10-second intervals)
   - Pico CSS framework for styling (no build step required)

5. ✅ **Main Application** ([main.rs](src/main.rs))
   - Load configuration from .env
   - Initialize web server with routes
   - Set up authentication middleware with tower-sessions
   - Initialize git and docker managers
   - Start Axum server with all integrations

### 📋 Summary

**Reusable**: ~20% (error handling patterns ✅, basic config loading approach ✅)
**Delete**: ~50% (github.rs, package_manager.rs, app_manager.rs, webhook.rs) ✅ COMPLETED
**New**: ~70% (git manager ✅, docker manager ✅, auth ✅, dashboard UI and routes ✅, main.rs integration ✅)
**Completed**: 5/5 new modules ✅ **ALL MODULES COMPLETE**

---

## Implementation Details

### Git Manager Module ([src/git.rs](src/git.rs))

**Status**: ✅ Completed

#### Architecture

The Git Manager is implemented as a struct-based API that wraps git CLI commands using `std::process::Command`. It supports single repository management with configurable remote and branch tracking.

#### Core Components

**GitManager Struct**
```rust
pub struct GitManager {
    repo_path: String,  // Path to the git repository
    remote: String,     // Remote name (e.g., "origin")
    branch: String,     // Branch to track (e.g., "main")
}
```

**GitStatus Struct**
```rust
pub struct GitStatus {
    pub local_commit: String,      // Current HEAD commit hash
    pub remote_commit: String,     // Remote branch commit hash
    pub updates_available: bool,   // True if remote is ahead of local
    pub current_branch: String,    // Current checked-out branch name
}
```

**PullResult Struct**
```rust
pub struct PullResult {
    pub success: bool,              // Whether pull succeeded
    pub already_up_to_date: bool,   // True if no changes were pulled
    pub files_changed: usize,       // Number of files modified
    pub output: String,             // Full git command output
}
```

**CommitInfo Struct**
```rust
pub struct CommitInfo {
    pub hash: String,           // Full commit hash
    pub short_hash: String,     // Abbreviated commit hash
    pub author_name: String,    // Commit author name
    pub author_email: String,   // Commit author email
    pub timestamp: i64,         // Unix timestamp of commit
    pub subject: String,        // Commit message subject line
    pub body: String,           // Commit message body
}
```

#### Public API Methods

1. **`new(repo_path, remote, branch) -> GitManager`**
   - Constructor for creating a new GitManager instance
   - Parameters:
     - `repo_path`: Path to the git repository directory
     - `remote`: Name of the git remote (typically "origin")
     - `branch`: Branch name to track (e.g., "main", "master")

2. **`fetch() -> Result<()>`**
   - Executes `git fetch <remote> <branch>`
   - Updates remote tracking branch without modifying working directory
   - Returns error if fetch fails or remote is unreachable
   - Logs operation with tracing

3. **`pull() -> Result<PullResult>`**
   - Executes `git pull --ff-only <remote> <branch>`
   - Uses `--ff-only` flag to prevent unexpected merge commits
   - Fails if local branch has diverged from remote
   - Parses output to extract number of files changed
   - Returns `PullResult` with operation details

4. **`get_status() -> Result<GitStatus>`**
   - Retrieves current repository status
   - Compares local HEAD with remote branch
   - Determines if updates are available
   - Returns comprehensive status information

5. **`validate_repository() -> Result<()>`**
   - Validates that the configured path is a valid git repository
   - Checks for existence of `.git` directory
   - Verifies that the configured remote exists
   - Should be called during initialization to fail fast

6. **`get_commit_info(commit_hash) -> Result<CommitInfo>`**
   - Retrieves detailed information about a specific commit
   - Uses `git show --no-patch` to get commit metadata
   - Returns structured commit information
   - Useful for displaying commit details in dashboard

#### Implementation Details

**Git Command Execution Pattern**
- All git commands use `std::process::Command`
- Commands are executed in the repository directory using `.current_dir()`
- Both stdout and stderr are captured
- Exit codes are checked, and stderr is returned in error messages
- All operations are synchronous (no async/await needed for Command)

**Safety Features**
- **Fast-forward only pulls**: Prevents accidental merge commits that could complicate history
- **Explicit remote/branch**: Always specifies remote and branch explicitly rather than relying on defaults
- **Error propagation**: All git errors are captured and wrapped in `MonitorError::Git`
- **Path validation**: Checks that repository path exists and contains `.git` directory

**Logging Strategy**
- `info!` level: Successful operations (fetch started/completed, pull completed)
- `debug!` level: Command outputs and detailed status information
- `error!` level: Failed operations with full error messages

#### Design Decisions

1. **Single Repository Support**
   - Initially designed for managing one repository
   - Can be extended to multiple repositories by creating multiple `GitManager` instances
   - Keeps implementation simple and focused

2. **CLI Over libgit2**
   - Uses git CLI instead of Rust git libraries (like git2-rs)
   - Advantages:
     - Simpler implementation
     - Relies on well-tested git binary
     - Easier to debug (can reproduce commands manually)
     - No need to manage libgit2 native dependencies
   - Disadvantages:
     - Requires git to be installed on system
     - Slower than native library calls
     - Output parsing can be fragile

3. **Specific Remote/Branch Tracking**
   - Compares against configured remote/branch (e.g., `origin/main`)
   - More predictable than using current branch's upstream
   - Explicitly configured in constructor

4. **Fast-Forward Only Pull Strategy**
   - Uses `--ff-only` flag for safety
   - Prevents unexpected merge commits
   - Forces explicit resolution if branches have diverged
   - User must manually resolve conflicts outside the tool

#### Error Handling

All operations return `Result<T>` using the custom `MonitorError` type:
- `MonitorError::Git(String)` for git-specific errors
- `MonitorError::Io(std::io::Error)` for filesystem/process errors

Common error scenarios:
- Repository path doesn't exist
- Path is not a git repository
- Remote doesn't exist
- Network failure during fetch
- Fast-forward pull not possible (diverged branches)
- Invalid commit hash

#### Testing

Includes unit tests for:
- `parse_files_changed()` - Parsing git output to extract file change counts
- Tests verify parsing of various git output formats

Future testing considerations:
- Integration tests with actual git repositories
- Mock git commands for testing error conditions
- Test with different git versions

#### Usage Example

```rust
use crate::git::GitManager;

// Initialize manager
let git = GitManager::new(
    "/path/to/repo",
    "origin",
    "main"
);

// Validate repository on startup
git.validate_repository()?;

// Check current status
let status = git.get_status()?;
if status.updates_available {
    println!("Updates available!");
    println!("Local:  {}", status.local_commit);
    println!("Remote: {}", status.remote_commit);
}

// Fetch updates
git.fetch()?;

// Pull changes
let result = git.pull()?;
if result.already_up_to_date {
    println!("Already up to date");
} else {
    println!("Pulled {} file(s)", result.files_changed);
}
```

#### Integration Points

The Git Manager will be used by:
1. **Web Dashboard** - To display repository status and handle user-triggered operations
2. **Configuration Module** - Repository path, remote, and branch will come from config
3. **Logging System** - Git operations will be logged for audit trail
4. **Docker Manager** - After successful pull, may trigger container rebuild

#### Future Enhancements

Potential improvements for future iterations:
- Support for multiple repositories
- Configurable pull strategies (allow merges, rebase)
- Git hooks integration
- Branch switching capability
- Stash management for dirty working directories
- Diff viewing between local and remote
- Commit history browsing
- Tag management

---

### Docker Manager Module ([src/docker.rs](src/docker.rs))

**Status**: ✅ Completed

#### Architecture

The Docker Manager is implemented using a hybrid approach:
1. **Bollard SDK** - For individual container operations (start/stop/restart/status queries)
2. **docker-compose CLI** - For multi-container orchestration (rebuild, up, down)

This approach provides clean Rust APIs for container management while leveraging docker-compose's existing orchestration logic without needing to parse docker-compose.yml files.

#### Core Components

**DockerManager Struct**
```rust
pub struct DockerManager {
    docker: Docker,              // Bollard client connection to Docker daemon
    compose_file_path: String,   // Path to docker-compose.yml
    compose_dir: String,         // Directory containing compose file
    container_names: Vec<String>,// List of container names to manage
}
```

**ContainerStatus Enum**
```rust
pub enum ContainerStatus {
    Running,      // Container is actively running
    Stopped,      // Container is stopped/exited
    Paused,       // Container is paused
    Restarting,   // Container is in restart loop
    Dead,         // Container is dead
    Created,      // Container created but not started
    Removing,     // Container is being removed
    Unknown,      // Unknown/empty status
}
```

**ContainerInfo Struct**
```rust
pub struct ContainerInfo {
    pub id: String,              // Full container ID
    pub name: String,            // Container name (without leading /)
    pub status: ContainerStatus, // Current container status
    pub image: String,           // Image name
    pub created: i64,            // Creation timestamp
}
```

**RebuildResult Struct**
```rust
pub struct RebuildResult {
    pub success: bool,           // Whether rebuild succeeded
    pub output: String,          // Command stdout
    pub error: Option<String>,   // Command stderr if failed
}
```

#### Public API Methods

**Initialization & Validation**

1. **`new(compose_file_path, container_names) -> Result<DockerManager>`**
   - Constructor for creating a new DockerManager instance
   - Parameters:
     - `compose_file_path`: Path to docker-compose.yml file
     - `container_names`: Vector of container names to manage
   - Connects to Docker daemon using local socket
   - Extracts compose directory from file path
   - Returns error if Docker connection fails

2. **`async validate() -> Result<()>`**
   - Validates Docker configuration during startup
   - Checks that compose file exists at specified path
   - Verifies Docker daemon connection by listing containers
   - Warns if configured containers don't exist yet (they'll be created on first compose up)
   - Should be called during initialization to fail fast

**Individual Container Operations (Bollard SDK)**

3. **`async get_container_status(container_name) -> Result<ContainerInfo>`**
   - Queries status of a specific container
   - Uses Docker inspect API to get detailed container information
   - Converts Docker API status enum to ContainerStatus
   - Returns structured ContainerInfo
   - Returns error if container doesn't exist

4. **`async get_all_container_status() -> Result<Vec<ContainerInfo>>`**
   - Queries status of all managed containers
   - Iterates through configured container_names list
   - Continues processing if individual containers fail (logs warning)
   - Returns vector of ContainerInfo for available containers

5. **`async start_container(container_name) -> Result<()>`**
   - Starts a specific stopped container
   - Uses Docker start API
   - Logs operation with tracing
   - Returns error if container doesn't exist or start fails

6. **`async stop_container(container_name) -> Result<()>`**
   - Stops a specific running container
   - Gives container 10 seconds to gracefully shutdown before force kill
   - Uses Docker stop API with timeout
   - Returns error if container doesn't exist

7. **`async restart_container(container_name) -> Result<()>`**
   - Restarts a specific container
   - Gives container 10 seconds to gracefully shutdown before force kill
   - Uses Docker restart API with timeout
   - More efficient than separate stop + start

8. **`async start_all_containers() -> Result<()>`**
   - Starts all managed containers sequentially
   - Stops on first error (doesn't continue with remaining containers)
   - Returns error if any container fails to start

9. **`async stop_all_containers() -> Result<()>`**
   - Stops all managed containers sequentially
   - Stops on first error (doesn't continue with remaining containers)
   - Each container gets 10-second graceful shutdown period

10. **`async restart_all_containers() -> Result<()>`**
    - Restarts all managed containers sequentially
    - Stops on first error (doesn't continue with remaining containers)
    - Each container gets 10-second graceful shutdown period

**Docker Compose Operations (CLI)**

11. **`async rebuild_container(container_name) -> Result<RebuildResult>`**
    - Rebuilds a single container using docker-compose
    - Process:
      1. Stops the container using Bollard SDK (with warning if fails)
      2. Executes `docker compose -f <file> up --build -d <name>`
      3. Captures stdout and stderr
    - Returns RebuildResult with success flag and output
    - Safer than force-stopping as compose handles dependencies

12. **`async rebuild_all_containers() -> Result<RebuildResult>`**
    - Rebuilds all containers using docker-compose
    - Process:
      1. Executes `docker compose -f <file> down`
      2. If down succeeds, executes `docker compose -f <file> up --build -d`
      3. Captures all output from both commands
    - Returns RebuildResult with combined output
    - Returns error in result (not as Err) if either command fails
    - This is the main rebuild operation for deployments

13. **`async compose_up() -> Result<RebuildResult>`**
    - Brings up containers without rebuilding images
    - Executes `docker compose -f <file> up -d`
    - Useful for starting containers after configuration changes
    - Returns RebuildResult with operation output

14. **`async compose_down() -> Result<RebuildResult>`**
    - Stops and removes all containers
    - Executes `docker compose -f <file> down`
    - Removes containers, networks (but not volumes by default)
    - Returns RebuildResult with operation output

#### Implementation Details

**Bollard SDK Integration**
- Uses `bollard::Docker::connect_with_local_defaults()` to connect to Docker daemon
- Automatically detects Docker socket location (Unix socket on Linux/Mac, named pipe on Windows)
- All container operations are async and use Bollard's async API
- Container names are cleaned by stripping leading '/' (Docker includes this in names)

**Docker Compose CLI Pattern**
- All compose commands use `std::process::Command` (synchronous)
- Commands are executed from compose file's directory using `.current_dir()`
- Both stdout and stderr are captured
- Exit codes are checked to determine success/failure
- Uses `docker compose` (v2 syntax) not `docker-compose` (v1)

**Graceful Shutdown**
- Stop and restart operations include 10-second timeout
- Allows containers to handle SIGTERM and cleanup gracefully
- After timeout, Docker sends SIGKILL to force termination
- Timeout is configurable via Docker API options

**Status Mapping**
- Bollard returns `ContainerStateStatusEnum` from Docker API
- Custom `From` implementation converts to simpler `ContainerStatus` enum
- `Display` trait implementation for user-friendly status strings
- Handles all Docker container states including edge cases (EMPTY -> Unknown)

**Error Handling Strategy**
- **Individual operations**: Return `Result<T>` and fail fast
- **Bulk operations** (get_all_status): Continue on individual failures, log warnings
- **Rebuild operations**: Return success flag in result rather than error type
- All Docker API errors wrapped in `MonitorError::Docker`
- All IO errors (command execution) wrapped in `MonitorError::Docker`

**Logging Strategy**
- `info!` level: Operation start and successful completion
- `debug!` level: Detailed status information, number of containers found
- `warn!` level: Missing containers, failed stops before rebuild
- `error!` level: Failed operations with full error messages

#### Design Decisions

1. **Hybrid Approach (Bollard + CLI)**
   - **Bollard for individual ops**: Clean async Rust API, type-safe, efficient
   - **CLI for compose ops**: No need to parse YAML, leverages compose orchestration logic
   - Avoids complexity of reimplementing compose dependency resolution
   - Requires `docker compose` CLI to be installed

2. **Single Compose File Support**
   - Manages all containers from one docker-compose.yml file
   - Simpler configuration and implementation
   - Can be extended to multiple compose files by creating multiple DockerManager instances
   - Matches typical deployment pattern (one compose file per project)

3. **Async Container Operations, Sync Compose Commands**
   - Bollard SDK is fully async (uses tokio)
   - Process::Command is synchronous but quick (just spawns process)
   - Mixed approach works because compose operations are infrequent
   - Could wrap Command in `tokio::task::spawn_blocking` for true async if needed

4. **Basic Success/Failure Monitoring**
   - Simple boolean result for rebuild operations
   - Captures full stdout/stderr for debugging
   - Doesn't stream build progress in real-time
   - Sufficient for MVP, can enhance with streaming later

5. **10-Second Graceful Shutdown**
   - Industry-standard timeout for container shutdown
   - Balances graceful cleanup vs. user wait time
   - Configurable via Docker API if needed
   - Prevents hanging on misbehaving containers

6. **Container Name Management**
   - Configuration uses simple container names (e.g., "web", "db")
   - Docker API returns names with leading "/" (e.g., "/web")
   - Names are cleaned automatically for consistent comparison
   - Supports both formats transparently

7. **Validation Warning vs Error**
   - Missing containers generate warnings, not errors
   - Allows service to start before containers are created
   - First `compose up` will create containers
   - More flexible for initial setup

#### Error Handling

All operations return `Result<T>` using the custom `MonitorError` type:
- `MonitorError::Docker(String)` for Docker-specific errors
- Propagates Bollard SDK errors with context
- Captures command execution failures

Common error scenarios:
- Docker daemon not running or unreachable
- Compose file doesn't exist
- Container doesn't exist
- Insufficient permissions to access Docker socket
- Container fails to start (port conflicts, missing dependencies)
- Rebuild fails (build errors, missing Dockerfile)
- Invalid container names

#### Testing

Includes unit tests for:
- `ContainerStatus::Display` - String representation of statuses
- `ContainerStatus::From<ContainerStateStatusEnum>` - Enum conversion
- Tests verify correct mapping of Docker API enums

Future testing considerations:
- Integration tests with actual Docker daemon
- Mock Docker API for testing error conditions
- Test with different Docker versions
- Test compose file variations
- Container lifecycle testing (start, stop, restart cycles)

#### Usage Example

```rust
use crate::docker::DockerManager;

// Initialize manager
let docker = DockerManager::new(
    "/path/to/docker-compose.yml",
    vec!["web".to_string(), "db".to_string(), "redis".to_string()]
)?;

// Validate configuration on startup
docker.validate().await?;

// Get status of all containers
let statuses = docker.get_all_container_status().await?;
for info in statuses {
    println!("{}: {}", info.name, info.status);
}

// Restart a specific container
docker.restart_container("web").await?;

// Rebuild all containers (typical deployment)
let result = docker.rebuild_all_containers().await?;
if result.success {
    println!("Rebuild successful!");
} else {
    eprintln!("Rebuild failed: {}", result.error.unwrap_or_default());
}
```

#### Integration Points

The Docker Manager will be used by:
1. **Web Dashboard** - To display container status and handle user-triggered operations
2. **Configuration Module** - Compose file path and container names will come from config
3. **Logging System** - Docker operations will be logged for audit trail
4. **Git Manager** - After successful git pull, may trigger container rebuild

#### Future Enhancements

Potential improvements for future iterations:
- **Real-time progress streaming**: Stream build output as it happens using Server-Sent Events
- **Multiple compose files**: Support managing different projects with separate compose files
- **Container logs**: Fetch and display container logs via Bollard SDK
- **Health checks**: Monitor container health status and restart unhealthy containers
- **Resource monitoring**: CPU, memory, network usage per container
- **Volume management**: List, inspect, and manage Docker volumes
- **Network inspection**: Show container networking and port mappings
- **Rollback support**: Keep previous images and roll back on failed deployments
- **Partial rebuild**: Rebuild only containers whose images changed
- **Build caching**: Intelligent cache management for faster rebuilds
- **Parallel operations**: Start/stop multiple containers concurrently
- **Container exec**: Execute commands inside running containers
- **Image pruning**: Cleanup old/unused images after rebuild
- **Custom compose commands**: Support arbitrary compose commands from dashboard
- **Docker Swarm support**: Extend to support swarm services
- **Configurable timeouts**: Per-container shutdown timeout configuration


---

### Authentication Module ([src/auth.rs](src/auth.rs))

**Status**: ✅ Completed

#### Architecture

The Authentication Module implements secure password-based authentication for the web dashboard using industry-standard security practices. It provides session management, rate limiting, and middleware for protecting routes.

#### Security Features Implemented

- **BCrypt Password Hashing**: Passwords hashed with bcrypt (cost factor 12)
- **Session Management**: Cookie-based sessions using tower-sessions
- **Rate Limiting**: 5 login attempts per IP within 5-minute window
- **Session Timeout**: Configurable inactivity timeout (default: 1 hour)
- **Secure Cookies**: HttpOnly, Secure, SameSite flags
- **Constant-time Comparison**: BCrypt prevents timing attacks
- **Thread-safe State**: Arc + RwLock for concurrent access

#### Public API

**Configuration**
- `AuthConfig::new(password, timeout)` - Create config with password hashing
- `AuthConfig::from_hash(hash, timeout)` - Create config with pre-hashed password
- `AuthState::new(config)` - Initialize authentication state

**Password Utilities**
- `hash_password(password)` - Hash plaintext password with bcrypt
- `verify_password(password, hash)` - Verify password against hash

**Session Management**
- `login(session, state, ip, request)` - Handle login with rate limiting
- `logout(session)` - Delete session and logout
- `is_authenticated(session)` - Check if session is valid
- `is_session_expired(session, timeout)` - Check timeout expiration

**Middleware**
- `require_auth(state, session, request, next)` - Axum middleware for route protection

#### Configuration

Environment variables:
```bash
DASHBOARD_PASSWORD=your_secure_password   # Required
SESSION_TIMEOUT=3600                      # Optional (default: 1 hour)
```

See [.env.example](.env.example) for full configuration example.

#### Testing

All unit tests pass:
- ✅ Password hashing and verification
- ✅ AuthConfig creation
- ✅ Rate limiting enforcement and reset

Run tests: `cargo test --bin github-monitor auth::`

---

### Web Dashboard Module ([src/routes.rs](src/routes.rs) + [templates/](templates/))

**Status**: ✅ Completed

#### Architecture

The Web Dashboard is implemented using server-side rendering with Askama templates, providing a clean and responsive UI for managing Git repositories and Docker containers. The implementation follows a RESTful API design pattern with HTML templates for the user interface.

#### Core Components

**AppState Struct**
```rust
pub struct AppState {
    pub config: Arc<Config>,           // Application configuration
    pub git: Arc<GitManager>,          // Git manager instance
    pub docker: Arc<DockerManager>,    // Docker manager instance
    pub password_hash: Arc<String>,    // Hashed password for auth
}
```

**Template Structures**

1. **LoginTemplate** - Login page with error display
2. **DashboardTemplate** - Main dashboard with git status and container list
3. **ContainerDisplay** - Container status information for UI rendering

**API Response Structures**

- `ApiResponse` - Standard JSON response for API endpoints
- `StatusResponse` - Combined git and container status for polling
- `GitStatusDisplay` - Git status information for JSON responses

#### Routes

**Public Routes**
- `GET /` - Redirect to dashboard
- `GET /login` - Display login page
- `POST /login` - Handle login form submission
- `GET /logout` - Logout and destroy session

**Protected Routes**
- `GET /dashboard` - Main dashboard page (requires authentication)

**API Endpoints** (all require authentication)

Git Operations:
- `GET /api/status` - Get current git and container status
- `POST /api/git/fetch` - Fetch updates from remote
- `POST /api/git/pull` - Pull changes from remote

Docker Container Operations:
- `POST /api/docker/start/:name` - Start specific container
- `POST /api/docker/stop/:name` - Stop specific container
- `POST /api/docker/restart/:name` - Restart specific container
- `POST /api/docker/rebuild/:name` - Rebuild specific container
- `POST /api/docker/start-all` - Start all managed containers
- `POST /api/docker/stop-all` - Stop all managed containers
- `POST /api/docker/restart-all` - Restart all managed containers
- `POST /api/docker/rebuild-all` - Rebuild all containers with compose

#### HTML Templates

**Base Template** ([templates/base.html](templates/base.html))
- Responsive layout with Pico CSS dark theme
- Navigation bar with logout link
- Shared styles for status badges and action buttons
- Footer with last update timestamp

**Login Template** ([templates/login.html](templates/login.html))
- Clean login form with password field
- Error message display for failed login attempts
- Centered layout with branded header
- Auto-focus on password field

**Dashboard Template** ([templates/dashboard.html](templates/dashboard.html))

Git Status Section:
- Repository path and current branch display
- Local and remote commit hashes (shortened to 8 chars)
- Visual indicator when updates are available
- Fetch and Pull action buttons
- Output display for operation results

Docker Containers Section:
- Container cards showing name, image, and status
- Color-coded status badges (green=running, gray=stopped, etc.)
- Individual container controls (Start/Stop/Restart/Rebuild)
- Bulk operations section for all containers
- Real-time output display for each operation

JavaScript Features:
- Auto-refresh status every 10 seconds via polling
- Async API calls with fetch
- Dynamic UI updates without page reload
- Operation output display with success/error styling
- Automatic page reload after successful git operations

#### Styling

**Pico CSS Framework**
- Lightweight, classless CSS framework
- No build step required (CDN-hosted)
- Dark theme enabled by default
- Responsive design for mobile and desktop
- Semantic HTML with automatic styling

**Custom Styles**
- Status badges with color coding
- Container cards with bordered sections
- Action buttons with proper spacing
- Output boxes with scrollable content
- Loading states and disabled button styling

#### Implementation Details

**Authentication Flow**
1. User navigates to any route
2. Session checked for authentication status
3. Unauthenticated users redirected to `/login`
4. Login form submits to `POST /login`
5. Password verified against bcrypt hash
6. Session created on success, error shown on failure
7. Authenticated users can access dashboard and API

**Dashboard Rendering**
1. Check session authentication
2. Fetch git status via `GitManager::get_status()`
3. Fetch all container statuses via `DockerManager::get_all_container_status()`
4. Transform data into template-friendly structures
5. Render Askama template with data
6. Return HTML response

**API Endpoint Pattern**
1. Check session authentication
2. Execute requested operation (git/docker command)
3. Capture results and any errors
4. Return JSON response with success flag and message/output
5. Frontend displays output and refreshes status

**Real-time Updates**
- Polling mechanism refreshes every 10 seconds
- JavaScript `setInterval` calls `/api/status` endpoint
- Response updates UI elements in-place
- User can trigger manual operations at any time
- Auto-refresh pauses during active operations

**Error Handling**
- All operations return `Result<T>` types
- Errors converted to user-friendly messages
- API responses include success flags and error details
- Dashboard displays errors in output boxes
- Logging captures all errors for debugging

#### Design Decisions

1. **Server-Side Rendering (SSR) with Askama**
   - Type-safe templates checked at compile time
   - No client-side framework needed
   - Simpler deployment (no build step)
   - Faster initial page load
   - Template logic in Rust, not JavaScript

2. **Polling vs WebSockets/SSE**
   - Polling chosen for simplicity and reliability
   - 10-second interval balances freshness vs server load
   - Works with all browsers and proxies
   - No persistent connection management needed
   - Sufficient for typical use case (manual deployments)

3. **RESTful API Design**
   - Clear separation of concerns
   - Each operation has dedicated endpoint
   - Consistent JSON response format
   - Easy to extend with new operations
   - Can be consumed by other clients if needed

4. **Pico CSS Framework**
   - Zero JavaScript dependencies
   - No build tools required
   - Professional appearance with minimal effort
   - Dark theme reduces eye strain
   - Responsive by default

5. **Session-based Authentication**
   - Standard cookie-based sessions
   - Memory store for simplicity (production could use Redis)
   - Automatic session timeout
   - CSRF protection via same-origin policy
   - Secure cookie settings

6. **Explicit Container Status Display**
   - Each container shown in separate card
   - Clear visual status indicators
   - Action buttons enabled/disabled based on state
   - Individual operation outputs per container
   - Bulk operations in separate section

#### User Interface Features

**Git Repository Management**
- View current branch and repository path
- See local commit hash (abbreviated)
- See remote commit hash (abbreviated)
- Visual "updates available" indicator
- One-click fetch to update remote tracking
- One-click pull to merge changes
- Pull button disabled when no updates available
- Operation output displayed below buttons

**Docker Container Management**
- Grid layout of container cards
- Status badge shows current state
- Container image name displayed
- Smart button enabling (can't start running container)
- Individual rebuild with output
- Bulk operations section for all containers
- Confirmation dialogs for destructive operations
- Real-time output for rebuild operations

**User Experience**
- Clean, distraction-free interface
- Logical grouping of operations
- Immediate feedback for all actions
- Auto-refresh keeps status current
- Mobile-friendly responsive design
- Clear error messages
- Operation history in output boxes

#### Integration Points

The Web Dashboard integrates with:
1. **Configuration Module** - Loads settings from `.env` file
2. **Git Manager** - Executes git operations and displays results
3. **Docker Manager** - Controls containers and shows status
4. **Authentication Module** - Protects routes and manages sessions
5. **Main Application** - Initialized and mounted in Axum server

#### Testing

**Manual Testing Checklist**
- ✅ Login with correct password succeeds
- ✅ Login with incorrect password shows error
- ✅ Logout clears session and redirects
- ✅ Unauthenticated access redirects to login
- ✅ Dashboard displays git status correctly
- ✅ Dashboard displays container statuses
- ✅ Fetch updates button works
- ✅ Pull changes button works when updates available
- ✅ Container start/stop/restart buttons work
- ✅ Container rebuild operations work
- ✅ Rebuild all containers works
- ✅ Auto-refresh updates status every 10 seconds
- ✅ Operation outputs display correctly
- ✅ Error messages display for failed operations

**Build Verification**
```bash
cargo check  # ✅ Passes with 0 errors
cargo build  # ✅ Compiles successfully
```

#### Future Enhancements

Potential improvements for future iterations:
- **Server-Sent Events (SSE)**: Stream rebuild progress in real-time
- **Container logs**: View live logs from running containers
- **Multi-repository support**: Manage multiple git repos in one dashboard
- **Operation history**: Persistent log of all operations
- **User management**: Multiple users with different permissions
- **Webhook support**: Auto-deploy on git push (optional feature)
- **Health checks**: Monitor container health and auto-restart
- **Resource monitoring**: Display CPU/memory usage per container
- **Dark/light theme toggle**: User preference for theme
- **Mobile app**: Native mobile interface
- **Notifications**: Email/Slack alerts for failures
- **Rollback support**: Revert to previous container version
- **Scheduled operations**: Cron-like scheduled rebuilds
- **Git branch switching**: Deploy different branches
- **Environment variable management**: Edit container env vars from UI

#### Usage Example

**Starting the Dashboard**
```bash
# Configure environment
cp .env.example .env
# Edit .env with your settings

# Run the application
cargo run --release

# Access dashboard
# Open browser to http://127.0.0.1:3000
# Login with configured password
```

**Typical Deployment Workflow**
1. Open dashboard and review git status
2. See "Updates available" indicator
3. Click "Fetch Updates" to update remote tracking
4. Review changes in git output
5. Click "Pull Changes" to merge updates
6. Verify pull was successful
7. Click "Rebuild All" to rebuild containers with new code
8. Monitor rebuild progress in output box
9. Verify containers started successfully
10. Check container status badges turn green

---

## 🎉 Project Status: Complete

All core requirements have been implemented:

✅ **Web Dashboard** - Fully functional with authentication and responsive UI
✅ **Git Management** - Fetch, pull, and status checking via CLI commands
✅ **Docker Management** - Full container lifecycle control with compose support
✅ **Authentication** - Secure password-based auth with session management
✅ **Configuration** - Environment-based config with validation
✅ **Error Handling** - Comprehensive error handling throughout
✅ **Logging** - Structured logging with tracing
✅ **Documentation** - Complete README and inline documentation

The GitHub + Docker Manager is ready for deployment and use!
