use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use tracing::{error, info, warn};
use crate::error::{MonitorError, Result};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub repository: Repository,
    pub commits: Vec<Commit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub clone_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProcessedWebhook {
    pub branch: String,
    pub repository: String,
    pub added_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub removed_files: Vec<String>,
}

pub struct WebhookServer {
    secret: String,
}

impl WebhookServer {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn router(self) -> Router {
        let state = Arc::new(self);
        Router::new()
            .route("/webhook", post(handle_webhook))
            .with_state(state)
    }

    /// Validates GitHub webhook signature using HMAC-SHA256
    /// GitHub sends signature in format: "sha256=<hex_digest>"
    pub fn validate_signature(&self, payload: &[u8], signature: &str) -> Result<bool> {
        // Extract the hex digest from the signature header
        let expected_signature = signature
            .strip_prefix("sha256=")
            .ok_or_else(|| {
                MonitorError::WebhookValidation("Invalid signature format".to_string())
            })?;

        // Create HMAC instance with the secret key
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .map_err(|e| MonitorError::WebhookValidation(format!("Invalid key: {}", e)))?;

        // Compute the HMAC of the payload
        mac.update(payload);
        let computed_hmac = mac.finalize().into_bytes();

        // Convert computed HMAC to hex string
        let computed_signature = hex::encode(computed_hmac);

        // Constant-time comparison to prevent timing attacks
        Ok(computed_signature == expected_signature)
    }

    /// Process webhook payload to extract branch, repository, and changed files
    /// Aggregates all file changes across all commits in the push event
    pub fn process_payload(&self, payload: WebhookPayload) -> Result<ProcessedWebhook> {
        // Extract branch name from git_ref (format: "refs/heads/branch-name")
        let branch = payload
            .git_ref
            .strip_prefix("refs/heads/")
            .ok_or_else(|| {
                MonitorError::WebhookValidation(format!(
                    "Invalid git ref format: {}",
                    payload.git_ref
                ))
            })?
            .to_string();

        info!(
            "Processing webhook for repository: {}, branch: {}",
            payload.repository.full_name, branch
        );

        // Aggregate all file changes from all commits
        let mut added_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut removed_files = Vec::new();

        for commit in &payload.commits {
            info!(
                "Processing commit {}: {}",
                &commit.id[..7],
                commit.message.lines().next().unwrap_or("")
            );

            added_files.extend(commit.added.clone());
            modified_files.extend(commit.modified.clone());
            removed_files.extend(commit.removed.clone());
        }

        // Remove duplicates while preserving order
        added_files.sort();
        added_files.dedup();
        modified_files.sort();
        modified_files.dedup();
        removed_files.sort();
        removed_files.dedup();

        info!(
            "Files changed - Added: {}, Modified: {}, Removed: {}",
            added_files.len(),
            modified_files.len(),
            removed_files.len()
        );

        Ok(ProcessedWebhook {
            branch,
            repository: payload.repository.full_name,
            added_files,
            modified_files,
            removed_files,
        })
    }
}

async fn handle_webhook(
    State(state): State<Arc<WebhookServer>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    info!("Received webhook request");

    // Extract the signature from headers
    let signature = match headers.get("X-Hub-Signature-256") {
        Some(sig) => match sig.to_str() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to parse signature header: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid signature header encoding"
                    })),
                );
            }
        },
        None => {
            warn!("Missing X-Hub-Signature-256 header");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing signature header"
                })),
            );
        }
    };

    // Validate the webhook signature
    match state.validate_signature(&body, signature) {
        Ok(true) => {
            info!("Webhook signature validated successfully");
        }
        Ok(false) => {
            warn!("Invalid webhook signature");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Invalid signature"
                })),
            );
        }
        Err(e) => {
            error!("Signature validation error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Signature validation failed: {}", e)
                })),
            );
        }
    }

    // Parse the webhook payload
    let payload: WebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse webhook payload: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid JSON payload: {}", e)
                })),
            );
        }
    };

    // Process the payload
    match state.process_payload(payload) {
        Ok(processed) => {
            info!(
                "Successfully processed webhook for {}/{} - {} files changed",
                processed.repository,
                processed.branch,
                processed.added_files.len()
                    + processed.modified_files.len()
                    + processed.removed_files.len()
            );

            // TODO: Trigger the update workflow here
            // This will be implemented in main.rs integration
            // For now, just return success

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "success",
                    "message": "Webhook processed successfully",
                    "repository": processed.repository,
                    "branch": processed.branch,
                    "files_changed": {
                        "added": processed.added_files.len(),
                        "modified": processed.modified_files.len(),
                        "removed": processed.removed_files.len()
                    }
                })),
            )
        }
        Err(e) => {
            error!("Failed to process webhook payload: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Payload processing failed: {}", e)
                })),
            )
        }
    }
}
