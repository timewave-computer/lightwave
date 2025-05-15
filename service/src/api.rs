use crate::state::StateManager;
use axum::{http::StatusCode, response::IntoResponse};
use hex;
use serde_json;
use tracing::{error, info};

pub async fn get_proof() -> impl IntoResponse {
    info!("Received request for latest proof");
    let state_manager = match StateManager::from_env() {
        Ok(manager) => manager,
        Err(e) => {
            error!("Failed to initialize state manager: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let service_state = match state_manager.load_state() {
        Ok(Some(state)) => state,
        Ok(None) => {
            info!("No state found in database");
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(e) => {
            error!("Failed to load state: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    match service_state.most_recent_wrapper_proof {
        Some(proof) => {
            info!("Returning latest proof");
            // Serialize the entire SP1ProofWithPublicValues using serde_json
            let serialized = serde_json::to_vec(&proof).unwrap();
            // Convert to hex for human readability
            let hex_proof = hex::encode(&serialized);
            (StatusCode::OK, hex_proof).into_response()
        }
        None => {
            info!("No proof available");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}
