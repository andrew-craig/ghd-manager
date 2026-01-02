mod auth;
mod config;
mod docker;
mod error;
mod git;
mod routes;

use anyhow::Result;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_memory_store::MemoryStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use docker::DockerManager;
use git::GitManager;
use routes::{create_router, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with environment filter support
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,github_monitor=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("GitHub + Docker Manager starting...");

    // Load configuration
    let config = Config::load()?;
    tracing::info!("Configuration loaded successfully");
    tracing::info!("Server will listen on {}:{}", config.server.host, config.server.port);
    tracing::info!("Git repository: {}", config.git.repo_path);
    tracing::info!("Docker compose file: {}", config.docker.compose_file);
    tracing::info!("Managing {} container(s)", config.docker.containers.len());

    // Hash the password for authentication
    let password_hash = auth::hash_password(&config.auth.password)?;
    tracing::info!("Password hash generated");

    // Initialize Git Manager
    let git_manager = GitManager::new(
        config.git.repo_path.clone(),
        config.git.remote.clone(),
        config.git.branch.clone(),
    );
    git_manager.validate_repository()?;
    tracing::info!("Git manager initialized and validated");

    // Initialize Docker Manager
    let docker_manager = DockerManager::new(
        config.docker.compose_file.clone(),
        config.docker.containers.clone(),
    )?;
    docker_manager.validate().await?;
    tracing::info!("Docker manager initialized and validated");

    // Create session store
    let session_store = MemoryStore::default();
    let session_expiry = Expiry::OnInactivity(
        tower_sessions::cookie::time::Duration::seconds(config.auth.session_timeout),
    );
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(session_expiry)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_secure(false); // Set to true if using HTTPS

    // Create application state
    let state = AppState {
        config: Arc::new(config.clone()),
        git: Arc::new(git_manager),
        docker: Arc::new(docker_manager),
        password_hash: Arc::new(password_hash),
    };

    // Create router
    let app = create_router(state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(session_layer),
        );

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);
    tracing::info!("GitHub + Docker Manager is ready!");
    tracing::info!("Visit http://{} to access the dashboard", addr);

    axum::serve(listener, app)
        .await?;

    Ok(())
}
