use axum::{Json, extract::{State, Path, Query}, http::StatusCode};
use serde::{Deserialize, Serialize};
use super::state::SharedState;
use super::participants::ErrorResponse;
use crate::types::{Operation, CodeRegion, OpType};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    #[serde(rename = "regionId")]
    region_id: Option<String>,
    #[serde(rename = "participantId")]
    participant_id: Option<String>,
    since: Option<u64>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct CompactResponse {
    compacted: usize,
    message: String,
}

// GET /rad/visible/:path
pub async fn visible(
    State(state): State<SharedState>,
    Path(file_path): Path<String>,
) -> Json<Vec<Operation>> {
    let oplog = state.oplog.lock().unwrap();
    let ops = oplog.all();

    let visible_writes: Vec<Operation> = ops
        .into_iter()
        .filter(|op| {
            format!("{:?}", op.status).to_lowercase() == "visible"
                && op.op_type == OpType::Write
                && op.region_id.starts_with(&format!("{}:", file_path))
        })
        .cloned()
        .collect();

    Json(visible_writes)
}

// GET /rad/files/:path
pub async fn file(
    State(state): State<SharedState>,
    Path(file_path): Path<String>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let oplog = state.oplog.lock().unwrap();
    let ops = oplog.all();

    let accepted_writes: Vec<Operation> = ops
        .into_iter()
        .filter(|op| {
            format!("{:?}", op.status).to_lowercase() == "accepted"
                && op.op_type == OpType::Write
                && op.region_id.starts_with(&format!("{}:", file_path))
        })
        .cloned()
        .collect();

    if let Some(latest) = accepted_writes.last() {
        Ok(latest.content.clone())
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "file not found or no accepted writes".to_string(),
            }),
        ))
    }
}

// GET /rad/files
pub async fn file_list(State(state): State<SharedState>) -> Json<Vec<String>> {
    let oplog = state.oplog.lock().unwrap();
    let ops = oplog.all();

    let mut files = HashSet::new();
    for op in ops {
        if op.op_type == OpType::Write {
            if let Some(colon_pos) = op.region_id.find(':') {
                files.insert(op.region_id[..colon_pos].to_string());
            }
        }
    }

    Json(files.into_iter().collect())
}

// GET /rad/regions/:path
pub async fn regions(
    State(state): State<SharedState>,
    Path(file_path): Path<String>,
) -> Json<Vec<CodeRegion>> {
    let region_map = state.region_map.lock().unwrap();
    let all_regions = region_map.get_all_regions();

    let file_regions: Vec<CodeRegion> = all_regions
        .into_iter()
        .filter(|r| r.file_path == file_path)
        .collect();

    Json(file_regions)
}

// GET /rad/log
pub async fn log(
    State(state): State<SharedState>,
    Query(query): Query<LogQuery>,
) -> Json<Vec<Operation>> {
    let oplog = state.oplog.lock().unwrap();
    let mut ops: Vec<Operation> = oplog.all().to_vec();

    // フィルタリング
    if let Some(ref region_id) = query.region_id {
        ops.retain(|op| &op.region_id == region_id);
    }
    if let Some(ref participant_id) = query.participant_id {
        ops.retain(|op| &op.participant_id == participant_id);
    }
    if let Some(since) = query.since {
        ops.retain(|op| op.timestamp >= since);
    }
    if let Some(limit) = query.limit {
        ops.truncate(limit);
    }

    Json(ops)
}

// POST /rad/compact
pub async fn compact(State(state): State<SharedState>) -> Json<CompactResponse> {
    let mut oplog = state.oplog.lock().unwrap();
    let ops = oplog.all().to_vec();

    let accepted_ops: Vec<Operation> = ops
        .iter()
        .filter(|op| format!("{:?}", op.status).to_lowercase() == "accepted")
        .cloned()
        .collect();

    let non_accepted_ops: Vec<Operation> = ops
        .into_iter()
        .filter(|op| format!("{:?}", op.status).to_lowercase() != "accepted")
        .collect();

    let compacted_count = accepted_ops.len();

    // accepted でない操作のみを残す
    // OpLog を新しいものに置き換える（load_operationsの代わり）
    *oplog = crate::oplog::OpLog::new();
    for op in non_accepted_ops {
        oplog.append(op);
    }

    Json(CompactResponse {
        compacted: compacted_count,
        message: "compaction completed".to_string(),
    })
}

// POST /rad/sync/git
pub async fn sync_git() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "not implemented yet (RP11)".to_string(),
        }),
    )
}
