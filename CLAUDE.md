# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**GitHub + Docker Manager** is a Rust-based web dashboard for managing Docker containers and Git repositories. It provides a simple web interface to monitor repository status, pull updates, and manage Docker containers built from your code.

The application enables remote triggering of:
1. Pulling updates from GitHub
2. Pulling and restarting Docker containers from remote registries
3. Viewing the status of containers

## Core Architecture

### Module Structure

The codebase follows a clean modular architecture with clear separation of concerns:

- **[main.rs](src/main.rs)**: Application entry point - initializes all managers, sets up authentication, configures session management, and starts the Axum web server
- **[config.rs](src/config.rs)**: Loads configuration from environment variables with validation
- **[error.rs](src/error.rs)**: Custom error types (`MonitorError`) used throughout the application
- **[auth.rs](src/auth.rs)**: Password-based authentication with bcrypt hashing, session management via tower-sessions, and rate limiting
- **[git.rs](src/git.rs)**: Git operations via CLI commands (fetch, pull, status checking)
- **[docker.rs](src/docker.rs)**: Docker management using hybrid approach - Bollard SDK for individual container operations + docker-compose CLI for orchestration
- **[routes.rs](src/routes.rs)**: Web routes, API endpoints, and request handlers
- **[templates/](templates/)**: Askama HTML templates for the web UI

### Key Architectural Decisions

**Git Management**: Uses git CLI commands (`git fetch`, `git pull`, `git rev-parse`) via `std::process::Command` instead of libgit2. This is simpler to implement, easier to debug, and relies on the well-tested git binary.

**Docker Management**: Hybrid approach using Bollard SDK for individual container operations (start/stop/restart/status) and docker-compose CLI for orchestration (pull and restart). This avoids parsing docker-compose.yml while providing clean Rust APIs. Containers are pulled from remote registries rather than built locally.

**Authentication**: Session-based authentication with bcrypt password hashing. Sessions stored in memory (MemoryStore) with configurable timeout. Rate limiting prevents brute-force attacks.

**Templating**: Server-side rendering with Askama (compile-time type-safe templates) instead of client-side frameworks. No build step required.

**UI Framework**: Pico CSS for styling - lightweight, no JavaScript dependencies, no build tools, professional dark theme out of the box.

**State Management**: All managers (`GitManager`, `DockerManager`) are wrapped in `Arc` and shared via Axum state. Thread-safe and efficient for concurrent requests.

## Development Commands

### Building and Running

```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run in development mode
cargo run

# Run release version
cargo run --release
```

### Testing and Validation

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test --bin github-monitor auth::
cargo test --bin github-monitor git::
cargo test --bin github-monitor docker::

# Check code without building
cargo check

# Run clippy for linting
cargo clippy

# Check formatting
cargo fmt --check

# Format code
cargo fmt
```

### Configuration Setup

```bash
# Copy example environment file
cp .env.example .env

# Edit .env with your actual configuration
# Required: DASHBOARD_PASSWORD, GIT_REPO_PATH, DOCKER_COMPOSE_FILE, DOCKER_CONTAINERS
```

## Configuration

All configuration is loaded from environment variables (via `.env` file):

**Required Variables:**
- `DASHBOARD_PASSWORD` - Password for web dashboard access (hashed with bcrypt)
- `GIT_REPO_PATH` - Absolute path to git repository to manage
- `DOCKER_COMPOSE_FILE` - Absolute path to docker-compose.yml
- `DOCKER_CONTAINERS` - Comma-separated list of container names (must match service names in docker-compose.yml)

**Optional Variables:**
- `SERVER_HOST` (default: `127.0.0.1`)
- `SERVER_PORT` (default: `3000`)
- `SESSION_TIMEOUT` (default: `3600` seconds)
- `GIT_REMOTE` (default: `origin`)
- `GIT_BRANCH` (default: `main`)
- `DOCKER_SOCKET` (default: `unix:///var/run/docker.sock`)

Configuration is validated on startup - the application will fail fast if paths don't exist or required variables are missing. See [config.rs:114-153](src/config.rs#L114-L153) for validation logic.

## Git Management Implementation

The `GitManager` wraps git CLI commands and provides these key operations:

- **`fetch()`** - Updates remote tracking branch without modifying working directory
- **`pull()`** - Merges changes using `--ff-only` flag (prevents unexpected merge commits)
- **`get_status()`** - Compares local HEAD with remote branch to detect available updates
- **`validate_repository()`** - Validates git repository exists and remote is configured

**Important Safety Features:**
- Uses `--ff-only` for pulls to prevent accidental merges
- Always specifies explicit remote and branch
- Validates repository on startup to fail fast

See [requirements.md:241-461](requirements.md#L241-L461) for complete Git Manager documentation.

## Docker Management Implementation

The `DockerManager` uses a hybrid architecture:

**Bollard SDK** (async Rust Docker API client):
- Individual container operations: `start_container()`, `stop_container()`, `restart_container()`
- Status queries: `get_container_status()`, `get_all_container_status()`
- Graceful shutdown with 10-second timeout before force kill

**docker-compose CLI**:
- `update_container(name)` - Pull and restart single container: `docker compose pull <name> && docker compose up -d <name>`
- `update_all_containers()` - Pull and restart all: `docker compose down && docker compose pull && docker compose up -d`
- Images are pulled from remote registries rather than built locally

**Why Hybrid?** This avoids needing to parse docker-compose.yml files while leveraging compose's orchestration logic for dependencies, networks, and volumes.

See [requirements.md:465-797](requirements.md#L465-L797) for complete Docker Manager documentation.

## Authentication Flow

1. User navigates to dashboard → redirected to `/login` if not authenticated
2. Login form submits password to `POST /login`
3. Password verified against bcrypt hash
4. Session created on success (cookie-based, auto-expires after timeout)
5. Rate limiting enforces 5 login attempts per IP within 5-minute window
6. Protected routes use `require_auth` middleware to check session validity

Sessions are stored in memory (tower-sessions with MemoryStore). For production with multiple instances, consider using Redis-backed session store.

## Web Dashboard Structure

### Routes

**Public Routes:**
- `GET /` - Redirect to dashboard
- `GET /login` - Login page
- `POST /login` - Login handler
- `GET /logout` - Logout and destroy session

**Protected API Endpoints:**
- `GET /api/status` - Get combined git + container status (used for polling)
- `POST /api/git/fetch` - Fetch updates from remote
- `POST /api/git/pull` - Pull changes
- `POST /api/docker/start/:name` - Start container
- `POST /api/docker/stop/:name` - Stop container
- `POST /api/docker/restart/:name` - Restart container
- `POST /api/docker/update/:name` - Update single container (pull and restart)
- `POST /api/docker/start-all` - Start all containers
- `POST /api/docker/stop-all` - Stop all containers
- `POST /api/docker/restart-all` - Restart all containers
- `POST /api/docker/update-all` - Update all containers (pull and restart)

### Templates

- **[base.html](templates/base.html)** - Base layout with Pico CSS, navigation, shared styles
- **[login.html](templates/login.html)** - Login form
- **[dashboard.html](templates/dashboard.html)** - Main dashboard with git status, container cards, and JavaScript for auto-refresh (10-second polling)

### Real-time Updates

The dashboard uses polling (not WebSockets) for simplicity:
- JavaScript `setInterval` calls `/api/status` every 10 seconds
- Updates UI in-place without page reload
- Auto-refresh pauses during active operations

## Error Handling

All operations return `Result<T, MonitorError>` where `MonitorError` is defined in [error.rs](src/error.rs):

```rust
pub enum MonitorError {
    Docker(String),
    Git(String),
    Authentication(String),
    Config(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}
```

Errors are converted to user-friendly messages in API responses and displayed in the dashboard's output boxes.

## Logging

Uses `tracing` crate with environment-based filtering:

```bash
# Default log level
RUST_LOG=info cargo run

# Debug logging for this application
RUST_LOG=github_monitor=debug cargo run

# Full debug logging
RUST_LOG=debug cargo run
```

**Logging Conventions:**
- `info!` - Successful operations, startup events
- `debug!` - Detailed status, command outputs
- `warn!` - Missing containers, non-fatal issues
- `error!` - Failed operations with full error messages

## Common Development Tasks

### Adding a New Git Operation

1. Add method to `GitManager` struct in [git.rs](src/git.rs)
2. Execute git command via `std::process::Command` with `.current_dir(repo_path)`
3. Parse output and return structured result
4. Add corresponding API endpoint in [routes.rs](src/routes.rs)
5. Update dashboard template to call new endpoint

### Adding a New Docker Operation

1. Add method to `DockerManager` struct in [docker.rs](src/docker.rs)
2. Use Bollard SDK for container-level ops or `std::process::Command` for compose ops
3. Add corresponding API endpoint in [routes.rs](src/routes.rs)
4. Update dashboard template with new button/action

### Modifying Configuration

1. Update structs in [config.rs](src/config.rs)
2. Add new environment variable parsing in `Config::load()`
3. Add validation in `Config::validate()` if needed
4. Update [.env.example](.env.example) with new variable
5. Update README.md configuration table

### Adding a New Template

1. Create new `.html` file in `templates/` directory
2. Define corresponding struct in [routes.rs](src/routes.rs) with `#[derive(Template)]`
3. Set template path with `#[template(path = "filename.html")]`
4. Implement route handler that renders the template
5. Templates can extend `base.html` using `{% extends "base.html" %}`

## Deployment Considerations

**Security:**
- Application binds to `127.0.0.1` by default (local-only)
- For remote access, use reverse proxy (nginx, Caddy) with HTTPS
- Use strong `DASHBOARD_PASSWORD` in production
- Consider rate limiting at reverse proxy level
- Review Docker socket permissions (requires access to `/var/run/docker.sock`)

**Process Management:**
- Use systemd, supervisor, or Docker to manage the application process
- Application logs to stdout (capture with systemd or container logs)
- Graceful shutdown on SIGTERM/SIGINT

**Session Store:**
- Current implementation uses in-memory sessions (lost on restart)
- For production, consider tower-sessions with Redis backend
- Memory store is fine for single-instance deployments

## Dependencies Overview

**Web Server:**
- `axum` - Web framework
- `tower`, `tower-http` - Middleware
- `tower-sessions` - Session management
- `tokio` - Async runtime

**Docker:**
- `bollard` - Docker API client

**Authentication:**
- `bcrypt` - Password hashing
- `chrono` - Timestamp handling

**Templating:**
- `askama`, `askama_axum` - HTML templates

**Configuration:**
- `dotenvy` - .env file loading
- `serde`, `serde_json` - Serialization

**Error Handling:**
- `anyhow` - Error handling
- `thiserror` - Custom error types

**Logging:**
- `tracing`, `tracing-subscriber` - Structured logging

## Typical Deployment Workflow

1. User opens dashboard and reviews git status
2. Sees "Updates available" indicator (if git updates are available)
3. Clicks "Fetch Updates" to update remote tracking
4. Clicks "Pull Changes" to merge git updates
5. Verifies pull was successful in output
6. Clicks "Update" button to pull latest container images from remote registry
7. Monitors update progress in output box
8. Verifies containers started successfully (status badges turn green)

Note: Containers are pulled from remote registries (e.g., Docker Hub, GitHub Container Registry) rather than built locally. This assumes pre-built images are pushed to a registry as part of your CI/CD pipeline.

## Project Status

All core requirements are complete and functional:
- ✅ Web dashboard with authentication
- ✅ Git management (fetch, pull, status)
- ✅ Docker management (container lifecycle + compose operations)
- ✅ Session management with timeout
- ✅ Real-time status updates via polling
- ✅ Comprehensive error handling
- ✅ Structured logging

See [requirements.md:1170-1183](requirements.md#L1170-L1183) for detailed implementation status.
