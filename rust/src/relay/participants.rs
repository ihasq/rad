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

    // S3 永続化 (spawn task to avoid blocking)
    if let Some(store) = state.store.clone() {
        // Clone data before spawning
        let types_participants: Vec<crate::types::Participant> = participants.iter().map(|p| {
            crate::types::Participant {
                id: p.participant_id.clone(),
                public_key: p.public_key.clone(),
                display_name: p.display_name.clone(),
                joined_at: p.joined_at,
            }
        }).collect();

        let founder_tree_json = {
            let ft = state.founder_tree.lock().unwrap();
            ft.to_json()
        };

        tokio::spawn(async move {
            println!("Saving {} participants to S3...", types_participants.len());
            match store.save_participants(&types_participants).await {
                Ok(_) => println!("Participants saved to S3 successfully"),
                Err(e) => eprintln!("Failed to save participants to S3: {}", e),
            }

            // Parse JSON back to FounderTree to save
            let temp_tree = crate::founder::FounderTree::from_json(&founder_tree_json, "");
            match store.save_founders(&temp_tree).await {
                Ok(_) => println!("Founders saved to S3 successfully"),
                Err(e) => eprintln!("Failed to save founders to S3: {}", e),
            }
        });
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
