# GitHub + Docker Manager

A Rust-based web dashboard for managing Docker containers and Git repositories. This service provides a simple web interface to monitor repository status, pull updates, and manage Docker containers built from your code.

## Features

- **Web Dashboard**: Clean, password-protected web interface
- **Git Management**:
  - View current local and remote commit status
  - Fetch updates from remote repository
  - Pull changes with one click
  - Visual indication when updates are available
- **Docker Management**:
  - View status of all managed containers
  - Start, stop, and restart individual containers
  - Rebuild containers with latest code
  - Rebuild all containers via docker-compose
- **Authentication**: Simple password-based authentication with session management
- **Real-time Updates**: Dashboard auto-refreshes every 10 seconds

## Prerequisites

- Rust 1.70 or later
- Docker and Docker Compose installed
- Git repository with a remote configured
- Docker containers defined in a docker-compose.yml file

## Installation

1. Clone this repository:
```bash
git clone <repository-url>
cd ghd-manager
```

2. Build the application:
```bash
cargo build --release
```

3. Copy the example environment file and configure it:
```bash
cp .env.example .env
```

4. Edit `.env` with your settings:
```env
# Server Configuration
SERVER_HOST=127.0.0.1
SERVER_PORT=3000

# Authentication
DASHBOARD_PASSWORD=your_secure_password_here
SESSION_TIMEOUT=3600

# Git Configuration
GIT_REPO_PATH=/path/to/your/repository
GIT_REMOTE=origin
GIT_BRANCH=main

# Docker Configuration
DOCKER_COMPOSE_FILE=/path/to/docker-compose.yml
DOCKER_CONTAINERS=web,db,redis
```

## Usage

### Running the Service

```bash
cargo run --release
```

Or run the compiled binary:
```bash
./target/release/github-monitor
```

The service will start and listen on the configured host and port (default: http://127.0.0.1:3000).

### Accessing the Dashboard

1. Open your browser and navigate to `http://127.0.0.1:3000`
2. Enter the password you configured in `.env`
3. You'll see the dashboard with:
   - Git repository status showing local and remote commits
   - List of all managed Docker containers with their status
   - Action buttons for each operation

### Typical Workflow

1. **Check for Updates**: The dashboard shows if remote commits are ahead of local
2. **Fetch Updates**: Click "Fetch Updates" to update the remote tracking branch
3. **Pull Changes**: Click "Pull Changes" to merge remote changes into local repository
4. **Rebuild Containers**: Click "Rebuild All" to rebuild containers with the new code
5. **Monitor Status**: Watch container status to ensure successful startup

## Configuration

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SERVER_HOST` | No | `127.0.0.1` | Host to bind the web server |
| `SERVER_PORT` | No | `3000` | Port for the web dashboard |
| `DASHBOARD_PASSWORD` | Yes | - | Password for dashboard access |
| `SESSION_TIMEOUT` | No | `3600` | Session timeout in seconds |
| `GIT_REPO_PATH` | Yes | - | Path to git repository |
| `GIT_REMOTE` | No | `origin` | Git remote name |
| `GIT_BRANCH` | No | `main` | Git branch to track |
| `DOCKER_COMPOSE_FILE` | Yes | - | Path to docker-compose.yml |
| `DOCKER_CONTAINERS` | Yes | - | Comma-separated list of container names |

### Docker Containers

The `DOCKER_CONTAINERS` variable should list the service names from your `docker-compose.yml` file:

```yaml
# docker-compose.yml
services:
  web:
    build: .
    ports:
      - "8080:8080"
  db:
    image: postgres:15
  redis:
    image: redis:7
```

For the above compose file, set:
```env
DOCKER_CONTAINERS=web,db,redis
```

## Architecture

### Modules

- **`main.rs`**: Application entry point, initializes all components
- **`config.rs`**: Configuration loading and validation
- **`auth.rs`**: Authentication with bcrypt password hashing and session management
- **`git.rs`**: Git operations via CLI commands (fetch, pull, status)
- **`docker.rs`**: Docker management using Bollard SDK and docker-compose CLI
- **`routes.rs`**: Web routes and API endpoints
- **`error.rs`**: Custom error types
- **`templates/`**: HTML templates using Askama

### Technologies

- **Web Framework**: Axum with Tower middleware
- **Templating**: Askama (type-safe compile-time templates)
- **Docker SDK**: Bollard (async Docker API client)
- **Authentication**: bcrypt + tower-sessions
- **CSS Framework**: Pico CSS (lightweight, no build step)

## Security Considerations

- The dashboard password is hashed using bcrypt
- Sessions expire after configured timeout (default: 1 hour)
- The service binds to `127.0.0.1` by default (localhost only)
- For production use:
  - Use a strong password
  - Consider using a reverse proxy (nginx, Caddy) with HTTPS
  - Restrict network access appropriately
  - Review Docker socket permissions

## Development

### Running in Development Mode

```bash
cargo run
```

### Running Tests

```bash
cargo test
```

### Checking Code

```bash
cargo check
cargo clippy
```

## Troubleshooting

### "Docker error: Cannot connect to Docker daemon"

- Ensure Docker is running
- Check Docker socket path in `.env` matches your system
- Verify your user has permission to access Docker socket

### "Git error: Repository not found"

- Ensure `GIT_REPO_PATH` points to a valid git repository
- Check that the path exists and contains a `.git` directory
- Verify the configured remote exists: `git remote -v`

### "Container not found"

- Ensure container names in `DOCKER_CONTAINERS` match service names in docker-compose.yml
- Containers must be created before they can be managed (run `docker compose up` once)

### Authentication Issues

- Clear browser cookies and try logging in again
- Verify `DASHBOARD_PASSWORD` is set correctly in `.env`
- Check server logs for authentication errors

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]
