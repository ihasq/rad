use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use super::state::SharedState;
use super::participants::ErrorResponse;
use crate::verify::verify_operation;
use crate::accept::handle_accept;

#[derive(Debug, Deserialize, Serialize)]
pub struct AcceptRequest {
    #[serde(rename = "participantId")]
    pub participant_id: String,
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AcceptResponse {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub status: String,
}

// POST /rad/accept
pub async fn accept(
    State(state): State<SharedState>,
    body_str: String,
) -> Result<Json<AcceptResponse>, (StatusCode, Json<ErrorResponse>)> {
    // JSON をパース
    let body: AcceptRequest = match serde_json::from_str(&body_str) {
        Ok(b) => b,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid JSON".to_string(),
                }),
            ));
        }
    };

    // signature 検証
    let _signature = match body.signature {
        Some(ref sig) => sig,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "signature is required".to_string(),
                }),
            ));
        }
    };

    // 参加者確認
    let participants = state.participants.lock().unwrap();
    let participant = participants
        .iter()
        .find(|p| p.participant_id == body.participant_id);

    let public_key = match participant {
        Some(p) => p.public_key.clone(),
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "participant not found".to_string(),
                }),
            ));
        }
    };
    drop(participants);

    // 署名検証 (元の JSON 文字列を使用)
    if !verify_operation(&body_str, &public_key) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "invalid signature".to_string(),
            }),
        ));
    }

    // handleAccept を呼び出し
    let mut oplog = state.oplog.lock().unwrap();
    let region_map = state.region_map.lock().unwrap();

    match handle_accept(&body.operation_id, &body.participant_id, &region_map, &mut oplog) {
        Ok(result) => {
            drop(oplog);
            drop(region_map);

            Ok(Json(AcceptResponse {
                operation_id: result.operation_id,
                status: result.status,
            }))
        }
        Err(e) => {
            // Leader でない場合は 403
            if e.contains("leader") || e.contains("Leader") {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: e,
                    }),
                ));
            }
            // その他のエラーは 409 または 400
            if e.contains("cannot accept") || e.contains("status") {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: e,
                    }),
                ));
            }
            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e,
                }),
            ))
        }
    }
}
