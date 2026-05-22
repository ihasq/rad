use axum::{Json, extract::{State, Path}, http::StatusCode};
use serde::{Deserialize, Serialize};
use super::state::SharedState;
use super::participants::ErrorResponse;
use crate::types::{Operation, OpType, OpStatus, CodeRegion};
use crate::verify::verify_operation;
use crate::reject::handle_reject;

#[derive(Debug, Deserialize, Serialize)]
pub struct OperationRequest {
    #[serde(rename = "participantId")]
    pub participant_id: String,
    #[serde(rename = "type")]
    pub op_type: String,
    #[serde(rename = "regionId")]
    pub region_id: Option<String>,
    pub content: Option<String>,
    pub reason: Option<String>,
    #[serde(rename = "targetOperationId")]
    pub target_operation_id: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OperationResponse {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp: u64,
}

// POST /rad/operations
pub async fn submit(
    State(state): State<SharedState>,
    body_str: String,
) -> Result<(StatusCode, Json<OperationResponse>), (StatusCode, Json<ErrorResponse>)> {
    // JSON をパース
    let body: OperationRequest = match serde_json::from_str(&body_str) {
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
    let signature = match body.signature {
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

    if body.op_type == "write" {
        // write 操作
        let region_id = match body.region_id {
            Some(ref rid) => rid,
            None => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "regionId is required for write".to_string(),
                    }),
                ));
            }
        };

        let content = match body.content {
            Some(ref c) => c,
            None => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "content is required for write".to_string(),
                    }),
                ));
            }
        };

        // regionId をパース
        let parts: Vec<&str> = region_id.split(':').collect();
        if parts.len() != 2 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid regionId format".to_string(),
                }),
            ));
        }

        let file_path = parts[0];
        let range_parts: Vec<&str> = parts[1].split('-').collect();
        if range_parts.len() != 2 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid regionId format".to_string(),
                }),
            ));
        }

        let start_line = range_parts[0].parse::<u32>().unwrap();
        let end_line = range_parts[1].parse::<u32>().unwrap();

        // Founder 登録
        let mut founder_tree = state.founder_tree.lock().unwrap();
        founder_tree.register_from_write(file_path, &body.participant_id);
        drop(founder_tree);

        // 領域登録
        let mut region_map = state.region_map.lock().unwrap();
        let region = CodeRegion {
            id: region_id.clone(),
            file_path: file_path.to_string(),
            start_line,
            end_line,
            owner_id: body.participant_id.clone(),
        };
        region_map.register(region);
        drop(region_map);

        // Operation 作成
        let mut op_counter = state.op_counter.lock().unwrap();
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let op_id = format!("op-{}-{}", timestamp, *op_counter);
        *op_counter += 1;
        drop(op_counter);

        let operation = Operation {
            id: op_id.clone(),
            participant_id: body.participant_id.clone(),
            region_id: region_id.clone(),
            op_type: OpType::Write,
            content: content.clone(),
            reason: None,
            signature: signature.clone(),
            timestamp,
            status: OpStatus::Visible,
        };

        let mut oplog = state.oplog.lock().unwrap();
        oplog.append(operation.clone());
        drop(oplog);

        // S3 永続化 (spawn task to avoid blocking)
        if let Some(store) = state.store.clone() {
            let op_clone = operation.clone();

            // Clone region_map and founder_tree for async task
            let region_map_clone = {
                let rm = state.region_map.lock().unwrap();
                rm.clone()
            };

            let founders_json = {
                let ft = state.founder_tree.lock().unwrap();
                ft.to_json()
            };

            tokio::spawn(async move {
                match store.append_op(&op_clone).await {
                    Ok(_) => println!("Operation {} saved to S3 successfully", op_clone.id),
                    Err(e) => eprintln!("Failed to save operation to S3: {}", e),
                }
                match store.save_regions(&region_map_clone).await {
                    Ok(_) => println!("Regions saved to S3 successfully"),
                    Err(e) => eprintln!("Failed to save regions to S3: {}", e),
                }
                // Parse JSON back to FounderTree to save
                let temp_tree = crate::founder::FounderTree::from_json(&founders_json, "");
                match store.save_founders(&temp_tree).await {
                    Ok(_) => println!("Founders saved to S3 successfully"),
                    Err(e) => eprintln!("Failed to save founders to S3: {}", e),
                }
            });
        }

        Ok((
            StatusCode::CREATED,
            Json(OperationResponse {
                operation_id: op_id,
                status: "visible".to_string(),
                timestamp: Some(timestamp),
            }),
        ))
    } else if body.op_type == "reject" {
        // reject 操作
        let target_op_id = match body.target_operation_id {
            Some(ref id) => id,
            None => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "targetOperationId is required for reject".to_string(),
                    }),
                ));
            }
        };

        let reason = match body.reason {
            Some(ref r) => r.clone(),
            None => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "reason is required for reject".to_string(),
                    }),
                ));
            }
        };

        // handleReject を呼び出し
        let mut oplog = state.oplog.lock().unwrap();
        let region_map = state.region_map.lock().unwrap();
        let founder_tree = state.founder_tree.lock().unwrap();

        match handle_reject(
            target_op_id,
            &body.participant_id,
            Some(&reason),
            &region_map,
            &founder_tree,
            &mut oplog,
        ) {
            Ok(result) => {
                drop(oplog);
                drop(region_map);
                drop(founder_tree);

                Ok((
                    StatusCode::CREATED,
                    Json(OperationResponse {
                        operation_id: result.operation_id,
                        status: result.status,
                        timestamp: None,
                    }),
                ))
            }
            Err(e) => {
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: e,
                    }),
                ))
            }
        }
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid operation type".to_string(),
            }),
        ))
    }
}

// GET /rad/operations/:id/status
pub async fn status(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let oplog = state.oplog.lock().unwrap();
    let op = oplog.get_by_id(&id);

    match op {
        Some(operation) => {
            Ok(Json(StatusResponse {
                operation_id: operation.id.clone(),
                status: format!("{:?}", operation.status).to_lowercase(),
                reason: operation.reason.clone(),
                timestamp: operation.timestamp,
            }))
        }
        None => {
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "operation not found".to_string(),
                }),
            ))
        }
    }
}

// GET /rad/operations/:id
pub async fn detail(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<Operation>, (StatusCode, Json<ErrorResponse>)> {
    let oplog = state.oplog.lock().unwrap();
    let op = oplog.get_by_id(&id);

    match op {
        Some(operation) => Ok(Json(operation.clone())),
        None => {
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "operation not found".to_string(),
                }),
            ))
        }
    }
}
