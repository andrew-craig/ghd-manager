# GitHub Repository Monitor & Auto-Updater

## Overview

A Rust-based service that automatically monitors a GitHub repository for changes, fetches updated files, manages Python package dependencies using `uv`, and restarts the application to apply updates.

## Core Requirements

### 1. GitHub Webhook Integration

- **Webhook Listener**: HTTP server that receives GitHub push webhook events
- **Event Filtering**: Process only push events to monitored repository/branch
- **Security**: Validate webhook signatures using shared secret
- **Endpoint**: Expose POST endpoint (e.g., `/webhook`) for GitHub to send events

### 2. Repository Synchronization

- **Change Detection**: Parse webhook payload to identify modified files
- **File Fetching**: Pull latest versions of changed files from GitHub repository
- **Authentication**: Support GitHub personal access tokens or app authentication
- **Error Handling**: Graceful handling of network failures and invalid responses

### 3. Package Management with uv

- **Dependency Detection**: Identify Python package requirements (e.g., from `requirements.txt`, `pyproject.toml`)
- **Package Upgrade**: Execute `uv` to upgrade existing packages to latest compatible versions
- **New Package Installation**: Install any newly added dependencies
- **Virtual Environment**: Manage Python virtual environment for isolated dependencies
- **Validation**: Verify successful package installation before proceeding

### 4. Application Restart

- **Process Management**: Track and control the monitored application's process
- **Graceful Shutdown**: Allow current operations to complete before restart
- **Startup**: Launch application with updated code and dependencies
- **Health Check**: Verify application starts successfully after restart
- **Rollback**: (Optional) Revert to previous version if restart fails

## Technical Specifications

### Configuration

- Repository URL and branch to monitor
- GitHub webhook secret for signature validation
- GitHub API authentication credentials
- Application start command and working directory
- Port for webhook server

### Logging & Monitoring

- Log all webhook events received
- Track file synchronization operations
- Record package upgrade activities
- Monitor application restart status

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
