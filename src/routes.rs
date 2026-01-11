use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_sessions::Session;
use askama::Template;

use crate::{
    auth::{self, SESSION_USER_KEY},
    config::Config,
    docker::DockerManager,
    git::GitManager,
};

// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub git: Arc<GitManager>,
    pub docker: Arc<DockerManager>,
    pub password_hash: Arc<String>,
}

// Template structs
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    repo_path: String,
    current_branch: String,
    local_commit: String,
    remote_commit: String,
    updates_available: bool,
    containers: Vec<ContainerDisplay>,
}

#[derive(Serialize)]
struct ContainerDisplay {
    name: String,
    image: String,
    status: String,
    status_class: String,
}

// Form structs
#[derive(Deserialize)]
pub struct LoginForm {
    password: String,
}

// API response structs
#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
}

#[derive(Serialize)]
struct StatusResponse {
    git: GitStatusDisplay,
    containers: Vec<ContainerDisplay>,
}

#[derive(Serialize)]
struct GitStatusDisplay {
    local_commit: String,
    remote_commit: String,
    updates_available: bool,
    current_branch: String,
}

// Create the router with all routes
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Public routes
        .route("/", get(index))
        .route("/login", get(show_login).post(handle_login))
        .route("/logout", get(handle_logout))
        // Protected routes
        .route("/dashboard", get(show_dashboard))
        // API routes
        .route("/api/status", get(api_status))
        .route("/api/git/fetch", post(api_git_fetch))
        .route("/api/git/pull", post(api_git_pull))
        .route("/api/docker/start/:name", post(api_docker_start))
        .route("/api/docker/stop/:name", post(api_docker_stop))
        .route("/api/docker/restart/:name", post(api_docker_restart))
        .route("/api/docker/update/:name", post(api_docker_update))
        .route("/api/docker/start-all", post(api_docker_start_all))
        .route("/api/docker/stop-all", post(api_docker_stop_all))
        .route("/api/docker/restart-all", post(api_docker_restart_all))
        .route("/api/docker/update-all", post(api_docker_update_all))
        .with_state(state)
}

// Route handlers

async fn index() -> Redirect {
    Redirect::to("/dashboard")
}

async fn show_login() -> impl IntoResponse {
    let template = LoginTemplate { error: None };
    Html(template.render().unwrap())
}

async fn handle_login(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Response {
    match auth::verify_password(&form.password, &state.password_hash) {
        Ok(true) => {
            // Password correct, create session
            if let Err(e) = session.insert(SESSION_USER_KEY, true).await {
                tracing::error!("Failed to create session: {}", e);
                let template = LoginTemplate {
                    error: Some("Session error. Please try again.".to_string()),
                };
                return Html(template.render().unwrap()).into_response();
            }

            Redirect::to("/dashboard").into_response()
        }
        Ok(false) => {
            // Password incorrect
            let template = LoginTemplate {
                error: Some("Invalid password".to_string()),
            };
            Html(template.render().unwrap()).into_response()
        }
        Err(e) => {
            // Error verifying password
            tracing::error!("Password verification error: {}", e);
            let template = LoginTemplate {
                error: Some("Authentication error. Please try again.".to_string()),
            };
            Html(template.render().unwrap()).into_response()
        }
    }
}

async fn handle_logout(session: Session) -> Redirect {
    session.delete().await.ok();
    Redirect::to("/login")
}

async fn show_dashboard(State(state): State<AppState>, session: Session) -> Response {
    // Check authentication
    if !auth::is_authenticated(&session).await {
        return Redirect::to("/login").into_response();
    }

    // Get git status
    let git_status = match state.git.get_status() {
        Ok(status) => status,
        Err(e) => {
            tracing::error!("Failed to get git status: {}", e);
            return Html(format!("Error getting git status: {}", e)).into_response();
        }
    };

    // Get container statuses
    let container_infos = match state.docker.get_all_container_status().await {
        Ok(infos) => infos,
        Err(e) => {
            tracing::error!("Failed to get container status: {}", e);
            return Html(format!("Error getting container status: {}", e)).into_response();
        }
    };

    let containers: Vec<ContainerDisplay> = container_infos
        .into_iter()
        .map(|info| {
            let status_str = info.status.to_string();
            let status_class = match status_str.as_str() {
                "running" => "running",
                "stopped" | "exited" => "stopped",
                "paused" => "paused",
                _ => "error",
            };
            ContainerDisplay {
                name: info.name,
                image: info.image,
                status: status_str,
                status_class: status_class.to_string(),
            }
        })
        .collect();

    let template = DashboardTemplate {
        repo_path: state.config.git.repo_path.clone(),
        current_branch: git_status.current_branch,
        local_commit: git_status.local_commit[..8].to_string(),
        remote_commit: git_status.remote_commit[..8].to_string(),
        updates_available: git_status.updates_available,
        containers,
    };

    Html(template.render().unwrap()).into_response()
}

// API handlers

async fn api_status(State(state): State<AppState>, session: Session) -> Response {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        })
        .into_response();
    }

    let git_status = match state.git.get_status() {
        Ok(status) => status,
        Err(e) => {
            return Json(ApiResponse {
                success: false,
                error: Some(format!("Git error: {}", e)),
                message: None,
                output: None,
            })
            .into_response();
        }
    };

    let container_infos = match state.docker.get_all_container_status().await {
        Ok(infos) => infos,
        Err(e) => {
            return Json(ApiResponse {
                success: false,
                error: Some(format!("Docker error: {}", e)),
                message: None,
                output: None,
            })
            .into_response();
        }
    };

    let containers: Vec<ContainerDisplay> = container_infos
        .into_iter()
        .map(|info| {
            let status_str = info.status.to_string();
            let status_class = match status_str.as_str() {
                "Running" => "running",
                "Stopped" | "Exited" => "stopped",
                "Paused" => "paused",
                _ => "error",
            };
            ContainerDisplay {
                name: info.name,
                image: info.image,
                status: status_str,
                status_class: status_class.to_string(),
            }
        })
        .collect();

    Json(StatusResponse {
        git: GitStatusDisplay {
            local_commit: git_status.local_commit[..8].to_string(),
            remote_commit: git_status.remote_commit[..8].to_string(),
            updates_available: git_status.updates_available,
            current_branch: git_status.current_branch,
        },
        containers,
    })
    .into_response()
}

async fn api_git_fetch(State(state): State<AppState>, session: Session) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.git.fetch() {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: Some("Successfully fetched updates from remote".to_string()),
            error: None,
            output: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Fetch failed: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_git_pull(State(state): State<AppState>, session: Session) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.git.pull() {
        Ok(result) => {
            let message = if result.already_up_to_date {
                "Already up to date".to_string()
            } else {
                format!("Successfully pulled {} file(s)", result.files_changed)
            };
            Json(ApiResponse {
                success: true,
                message: Some(message),
                error: None,
                output: Some(result.output),
            })
        }
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Pull failed: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_start(
    State(state): State<AppState>,
    session: Session,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.start_container(&name).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: Some(format!("Successfully started container '{}'", name)),
            error: None,
            output: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to start container: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_stop(
    State(state): State<AppState>,
    session: Session,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.stop_container(&name).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: Some(format!("Successfully stopped container '{}'", name)),
            error: None,
            output: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to stop container: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_restart(
    State(state): State<AppState>,
    session: Session,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.restart_container(&name).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: Some(format!("Successfully restarted container '{}'", name)),
            error: None,
            output: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to restart container: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_update(
    State(state): State<AppState>,
    session: Session,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.update_container(&name).await {
        Ok(result) => Json(ApiResponse {
            success: result.success,
            message: if result.success {
                Some(format!("Successfully updated container '{}'", name))
            } else {
                None
            },
            error: result.error,
            output: Some(result.output),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to update container: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_start_all(
    State(state): State<AppState>,
    session: Session,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.start_all_containers().await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: Some("Successfully started all containers".to_string()),
            error: None,
            output: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to start all containers: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_stop_all(
    State(state): State<AppState>,
    session: Session,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.stop_all_containers().await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: Some("Successfully stopped all containers".to_string()),
            error: None,
            output: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to stop all containers: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_restart_all(
    State(state): State<AppState>,
    session: Session,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.restart_all_containers().await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: Some("Successfully restarted all containers".to_string()),
            error: None,
            output: None,
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to restart all containers: {}", e)),
            message: None,
            output: None,
        }),
    }
}

async fn api_docker_update_all(
    State(state): State<AppState>,
    session: Session,
) -> Json<ApiResponse> {
    if !auth::is_authenticated(&session).await {
        return Json(ApiResponse {
            success: false,
            error: Some("Unauthorized".to_string()),
            message: None,
            output: None,
        });
    }

    match state.docker.update_all_containers().await {
        Ok(result) => Json(ApiResponse {
            success: result.success,
            message: if result.success {
                Some("Successfully updated all containers".to_string())
            } else {
                None
            },
            error: result.error,
            output: Some(result.output),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            error: Some(format!("Failed to update all containers: {}", e)),
            message: None,
            output: None,
        }),
    }
}
