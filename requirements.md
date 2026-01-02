# GitHub + Docker Manager & Updater

## Overview

A Rust-based service that manages projects that are deployed as Docker containers built from Github repositories. The service enables remote triggering of (1) pulling updates from Github, (2) triggering container rebuilds and (3) viewing the status of containers.

## Core Requirements

### 1. Web dashboard 
A simple web dashboard with
- **Auth**: password-based authentication. the password can be set in the .env
- **Github**: 
  - user can see latest commiy of remote (origin) and the current state of the local (i.e. is an update available)
  - user can trigger fetch from Github 
 events
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

### Configuration

- Repository URL and branch to monitor
- Docker container names
- password
- GitHub API authentication credentials?
- orking directory
- Port for web server

### Logging & Monitoring

- Log all events received from the FE
- Track file synchronization operations
- Record Docker management activities

### Error Handling

- Retry logic for transient failures
- Notification on critical errors (webhook validation failures, restart failures)
- Maintain service availability even if monitored app fails

## Workflow

1. Service starts and begins listening for webhooks
2. GitHub sends push event to webhook endpoint
3. Service validates signature and parses payload
4. Changed files are fetched from GitHub
5. `uv` scans for dependency changes and upgrades packages
6. Application is gracefully stopped
7. Application restarts with new code and dependencies
8. Service continues monitoring for next webhook event

---

## Implementation Status

### ✅ Completed Components

- **Error Handling** (`error.rs`) - Complete error types for all modules
- **GitHub Client** (`github.rs`) - File fetching from GitHub API with authentication
- **Package Manager** (`package_manager.rs`) - Full uv integration with dependency detection and package sync
- **App Manager** (`app_manager.rs`) - Complete process lifecycle management with graceful shutdown

### 🚧 Remaining Tasks

1. **Configuration Loading** (`config.rs:34-35`)
   - Implement loading from config file (TOML/JSON/YAML)
   - Support environment variable overrides
   - Add configuration validation

2. **Webhook Signature Validation** (`webhook.rs:53-54`)
   - Implement HMAC-SHA256 signature verification
   - Validate GitHub webhook signatures using secret

3. **Webhook Payload Processing** (`webhook.rs:58-59`)
   - Parse webhook payload for changed files
   - Filter events by branch/repository
   - Extract file paths (added/modified/removed)

4. **Webhook Handler Implementation** (`webhook.rs:67`)
   - Complete async webhook handler function
   - Integrate signature validation
   - Process payload and trigger update workflow

5. **File Writing to Disk**
   - After fetching changed files from GitHub, write them to local working directory
   - Preserve directory structure
   - Handle file permissions appropriately

6. **Main Integration** (`main.rs:18-20`)
   - Load configuration on startup
   - Initialize webhook server with configuration
   - Wire together all components (GitHub client, package manager, app manager)
   - Start HTTP server and begin monitoring
   - Implement complete update workflow when webhook received

7. **Testing & Validation**
   - End-to-end testing of webhook → fetch → update → restart workflow
   - Error handling and retry logic verification
   - Configuration validation testing
