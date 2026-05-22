use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use super::state::{SharedState, Participant};

#[derive(Debug, Deserialize)]
pub struct JoinRequest {
    #[serde(rename = "publicKey")]
    pub public_key: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

// POST /rad/participants
pub async fn join(
    State(state): State<SharedState>,
    Json(body): Json<JoinRequest>,
) -> Result<(StatusCode, Json<Participant>), (StatusCode, Json<ErrorResponse>)> {
    // publicKey 検証
    let public_key = match body.public_key {
        Some(pk) => pk,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "publicKey is required".to_string(),
                }),
            ));
        }
    };

    // participantId 生成
    let participant_id = body.display_name.clone().unwrap_or_else(|| format!("participant-{}", chrono::Utc::now().timestamp_millis()));

    // 最初の参加者は Founder
    let mut participants = state.participants.lock().unwrap();
    let is_founder = participants.is_empty();

    let participant = Participant {
        participant_id: participant_id.clone(),
        public_key,
        display_name: body.display_name,
        is_founder,
        joined_at: chrono::Utc::now().timestamp_millis() as u64,
    };

    participants.push(participant.clone());

    // Founder として登録
    if is_founder {
        let mut founder_tree = state.founder_tree.lock().unwrap();
        founder_tree.register_from_write(".", &participant_id);
    }

    Ok((StatusCode::CREATED, Json(participant)))
}

// GET /rad/participants
pub async fn list(
    State(state): State<SharedState>,
) -> Json<Vec<Participant>> {
    let participants = state.participants.lock().unwrap();
    Json(participants.clone())
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
