use std::cell::RefCell;
use rad_core::{oplog::OpLog, region::RegionMap, founder::FounderTree, types::*, verify::verify_operation};

mod host;
mod wasm_backend;

use wasm_backend::WasmStorageBackend;

// ==================
// グローバル状態
// ==================

struct RadState {
    oplog: OpLog,
    region_map: RegionMap,
    founder_tree: FounderTree,
    participants: Vec<Participant>,
    backend: WasmStorageBackend,
}

thread_local! {
    static STATE: RefCell<Option<RadState>> = RefCell::new(None);
}

// ==================
// メモリ管理
// ==================

#[no_mangle]
pub extern "C" fn rad_alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn rad_dealloc(ptr: *mut u8, size: usize) {
    drop(Vec::from_raw_parts(ptr, 0, size));
}

// 結果バッファ - static mut で直接管理
static mut RESULT_BUFFER: Vec<u8> = Vec::new();

fn set_result(s: &str) {
    unsafe {
        RESULT_BUFFER = s.as_bytes().to_vec();
    }
}

#[no_mangle]
pub extern "C" fn rad_result_ptr() -> *const u8 {
    unsafe {
        if RESULT_BUFFER.is_empty() {
            std::ptr::null()
        } else {
            RESULT_BUFFER.as_ptr()
        }
    }
}

#[no_mangle]
pub extern "C" fn rad_result_len() -> usize {
    unsafe { RESULT_BUFFER.len() }
}

// ==================
// ヘルパー関数
// ==================

unsafe fn read_string(ptr: *const u8, len: usize) -> String {
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf8_lossy(slice).to_string()
}

// ==================
// レスポンス統一ヘルパー
// ==================

fn ok_response<T: serde::Serialize>(data: T) -> String {
    serde_json::json!({
        "ok": true,
        "data": data
    }).to_string()
}

fn error_response(message: &str, code: &str) -> String {
    serde_json::json!({
        "ok": false,
        "error": message,
        "code": code
    }).to_string()
}

// ==================
// ID/Timestamp 生成（ホスト関数経由）
// ==================

fn generate_op_id() -> String {
    let ts = unsafe { host::host_get_timestamp() };
    let rand = unsafe { host::host_random_u64() };
    format!("op-{}-{}", ts, rand % 10000)
}

fn current_timestamp() -> u64 {
    unsafe { host::host_get_timestamp() }
}

// ==================
// WASM Export 関数
// ==================

/// 初期化
#[no_mangle]
pub extern "C" fn rad_init() -> i32 {
    STATE.with(|s| {
        *s.borrow_mut() = Some(RadState {
            oplog: OpLog::new(),
            region_map: RegionMap::new(),
            founder_tree: FounderTree::new("system"),
            participants: vec![],
            backend: WasmStorageBackend::new(),
        });
    });
    let response = ok_response(serde_json::json!({"status": "initialized"}));
    set_result(&response);
    0
}

/// 参加者登録
/// 入力: {"publicKey": "...", "displayName": "..."}
/// 出力: {ok: true, data: {"id": "...", "publicKey": "...", "joinedAt": 123}}
#[no_mangle]
pub extern "C" fn rad_join(input_ptr: *const u8, input_len: usize) -> i32 {
    let input = unsafe { read_string(input_ptr, input_len) };

    let result: Result<serde_json::Value, (String, String)> = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().ok_or(("State not initialized".to_string(), "INTERNAL".to_string()))?;

        // JSON パース
        #[derive(serde::Deserialize)]
        struct JoinRequest {
            #[serde(rename = "publicKey")]
            public_key: Option<String>,
            #[serde(rename = "displayName")]
            display_name: Option<String>,
        }

        let req: JoinRequest = serde_json::from_str(&input)
            .map_err(|e| (format!("Invalid JSON: {}", e), "INVALID_JSON".to_string()))?;

        // publicKey必須チェック
        let public_key = req.public_key.ok_or(("publicKey is required".to_string(), "MISSING_FIELD".to_string()))?;

        // 参加者作成
        let participant = Participant {
            id: format!("p-{}", state.participants.len()),
            public_key: public_key.clone(),
            display_name: req.display_name,
            joined_at: 1234567890, // TODO: 実際のタイムスタンプ
        };

        state.participants.push(participant.clone());

        // ストレージに保存
        let key = format!("participants/{}", participant.id);
        let data = serde_json::to_string(&participant).unwrap();
        let _ = state.backend.put(&key, &data);

        // レスポンス作成（内部スキーマ）
        Ok(serde_json::json!({
            "id": participant.id,
            "publicKey": public_key,
            "joinedAt": participant.joined_at,
            "isFounder": state.participants.len() == 1,
            "isMessenger": false
        }))
    });

    match result {
        Ok(data) => {
            set_result(&ok_response(data));
            0
        }
        Err((msg, code)) => {
            set_result(&error_response(&msg, &code));
            -1
        }
    }
}

/// 操作送信
/// 入力: SubmitInput JSON (id/timestamp なし、signature 付き)
/// 出力: {ok: true, data: {"id": "...", "status": "visible"}}
#[no_mangle]
pub extern "C" fn rad_submit_op(input_ptr: *const u8, input_len: usize) -> i32 {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SubmitInput {
        participant_id: String,
        #[serde(rename = "type")]
        op_type: String,
        region_id: Option<String>,  // write/delete で使用
        target_operation_id: Option<String>,  // reject/approve で使用
        #[serde(default)]
        content: String,
        reason: Option<String>,
        signature: String,
    }

    let input_str = unsafe { read_string(input_ptr, input_len) };

    let result = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().unwrap();

        // JSON パース（SubmitInput: id/timestamp なし）
        let input: SubmitInput = serde_json::from_str(&input_str)
            .map_err(|e| (format!("Invalid JSON: {}", e), "INVALID_JSON".to_string()))?;

        // 署名検証: 入力 JSON をそのまま canonicalize
        let participant = state.participants.iter()
            .find(|p| p.id == input.participant_id)
            .ok_or(("Participant not found".to_string(), "NOT_FOUND".to_string()))?;

        if !verify_operation(&input_str, &participant.public_key) {
            return Err(("Invalid signature".to_string(), "INVALID_SIGNATURE".to_string()));
        }

        // WASM がホスト関数経由で id/timestamp を生成
        let op_type = match input.op_type.as_str() {
            "write" => OpType::Write,
            "delete" => OpType::Delete,
            "reject" => OpType::Reject,
            "approve" => OpType::Approve,
            _ => return Err(("Invalid operation type".to_string(), "INVALID_JSON".to_string())),
        };

        // region_id の決定: write/delete は regionId、reject/approve は targetOperationId から取得
        let region_id = match op_type {
            OpType::Write | OpType::Delete => {
                input.region_id.ok_or(("regionId is required for write/delete".to_string(), "MISSING_FIELD".to_string()))?
            }
            OpType::Reject | OpType::Approve => {
                // reject/approve の場合、targetOperationId から対象操作を取得してregion_idを使う
                let target_id = input.target_operation_id.ok_or(("targetOperationId is required for reject/approve".to_string(), "MISSING_FIELD".to_string()))?;
                // target_id をそのまま region_id として使用（仕様により異なるが、ここでは簡易的に）
                target_id
            }
        };

        let op = Operation {
            id: generate_op_id(),
            participant_id: input.participant_id,
            region_id,
            op_type,
            content: input.content,
            reason: input.reason,
            signature: input.signature,
            timestamp: current_timestamp(),
            status: OpStatus::Visible,
        };

        // OpLog に追加
        state.oplog.append(op.clone());

        // ストレージに保存
        let key = format!("operations/{}", op.id);
        let data = serde_json::to_string(&op).unwrap();
        let _ = state.backend.put(&key, &data);

        Ok(serde_json::json!({
            "id": op.id,
            "status": "visible"
        }))
    });

    match result {
        Ok(data) => {
            set_result(&ok_response(data));
            0
        }
        Err((msg, code)) => {
            set_result(&error_response(&msg, &code));
            -1
        }
    }
}

/// Accept 操作
/// 入力: AcceptInput JSON (operationId, participantId, signature)
/// 出力: {ok: true, data: {"id": "...", "status": "accepted", "acceptedBy": "...", "acceptedAt": 123}}
#[no_mangle]
pub extern "C" fn rad_accept(input_ptr: *const u8, input_len: usize) -> i32 {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AcceptInput {
        participant_id: String,
        operation_id: String,
        signature: String,
    }

    let input_str = unsafe { read_string(input_ptr, input_len) };

    let result = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().unwrap();

        // JSON パース
        let input: AcceptInput = serde_json::from_str(&input_str)
            .map_err(|e| (format!("Invalid JSON: {}", e), "INVALID_JSON".to_string()))?;

        // 署名検証
        let participant = state.participants.iter()
            .find(|p| p.id == input.participant_id)
            .ok_or(("Participant not found".to_string(), "NOT_FOUND".to_string()))?;

        if !verify_operation(&input_str, &participant.public_key) {
            return Err(("Invalid signature".to_string(), "INVALID_SIGNATURE".to_string()));
        }

        // ステータス更新
        state.oplog.set_status(&input.operation_id, OpStatus::Accepted);

        // ストレージに保存
        if let Some(op) = state.oplog.get_by_id(&input.operation_id) {
            let key = format!("operations/{}", input.operation_id);
            let data = serde_json::to_string(&op).unwrap();
            let _ = state.backend.put(&key, &data);
        }

        Ok(serde_json::json!({
            "id": input.operation_id,
            "status": "accepted",
            "acceptedBy": input.participant_id,
            "acceptedAt": current_timestamp()
        }))
    });

    match result {
        Ok(data) => {
            set_result(&ok_response(data));
            0
        }
        Err((msg, code)) => {
            set_result(&error_response(&msg, &code));
            -1
        }
    }
}

/// 操作ログ取得
/// 入力: {} (empty or with filters)
/// 出力: {ok: true, data: [Operation, ...]}
#[no_mangle]
pub extern "C" fn rad_get_log(_input_ptr: *const u8, _input_len: usize) -> i32 {
    STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();

        let ops = state.oplog.get_all_operations();
        set_result(&ok_response(ops));
    });
    0
}

/// コンパクション
#[no_mangle]
pub extern "C" fn rad_compact() -> i32 {
    let data = serde_json::json!({"status": "compacted"});
    set_result(&ok_response(data));
    0
}

/// 参加者一覧取得
/// 出力: {ok: true, data: [Participant, ...]}
#[no_mangle]
pub extern "C" fn rad_get_participants() -> i32 {
    STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();

        set_result(&ok_response(&state.participants));
    });
    0
}

/// ファイル取得
/// 出力: {ok: true, data: {"content": "..."}}
#[no_mangle]
pub extern "C" fn rad_get_file(path_ptr: *const u8, path_len: usize) -> i32 {
    let result: Result<serde_json::Value, (String, String)> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or(("State not initialized".to_string(), "INTERNAL".to_string()))?;
        let path = unsafe { read_string(path_ptr, path_len) };

        // Get accepted operations for this file
        let mut content_parts: Vec<(usize, String)> = vec![];
        for op in state.oplog.get_all_operations() {
            if op.region_id.starts_with(&path) && matches!(op.status, OpStatus::Accepted) {
                // Parse region ID to get line number
                if let Some(range) = op.region_id.split(':').nth(1) {
                    if let Some(start_str) = range.split('-').next() {
                        if let Ok(start) = start_str.parse::<usize>() {
                            content_parts.push((start, op.content.clone()));
                        }
                    }
                }
            }
        }

        // Sort by line number and concatenate
        content_parts.sort_by_key(|(line, _)| *line);
        let content: String = content_parts
            .into_iter()
            .map(|(_, c)| c)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(serde_json::json!({ "content": content }))
    });

    match result {
        Ok(data) => {
            set_result(&ok_response(data));
            0
        }
        Err((msg, code)) => {
            set_result(&error_response(&msg, &code));
            -1
        }
    }
}

/// コード領域取得
/// 出力: {ok: true, data: [Region, ...]}
#[no_mangle]
pub extern "C" fn rad_get_regions(path_ptr: *const u8, path_len: usize) -> i32 {
    STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();
        let path = unsafe { read_string(path_ptr, path_len) };

        let regions = state.region_map.list(&path);
        set_result(&ok_response(regions));
    });
    0
}

/// 操作ステータス取得
/// 出力: {ok: true, data: {"id": "...", "status": "...", ...}}
#[no_mangle]
pub extern "C" fn rad_get_op_status(id_ptr: *const u8, id_len: usize) -> i32 {
    let result: Result<serde_json::Value, (String, String)> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or(("State not initialized".to_string(), "INTERNAL".to_string()))?;
        let id = unsafe { read_string(id_ptr, id_len) };

        match state.oplog.get_by_id(&id) {
            Some(op) => {
                Ok(serde_json::json!({
                    "id": op.id,
                    "status": format!("{:?}", op.status).to_lowercase(),
                    "reason": op.reason,
                    "decidedBy": None::<String>,
                    "decidedAt": None::<u64>
                }))
            }
            None => Err(("Operation not found".to_string(), "NOT_FOUND".to_string()))
        }
    });

    match result {
        Ok(data) => {
            set_result(&ok_response(data));
            0
        }
        Err((msg, code)) => {
            set_result(&error_response(&msg, &code));
            -1
        }
    }
}

/// 操作取得
/// 出力: {ok: true, data: Operation}
#[no_mangle]
pub extern "C" fn rad_get_op(id_ptr: *const u8, id_len: usize) -> i32 {
    let result: Result<Operation, (String, String)> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or(("State not initialized".to_string(), "INTERNAL".to_string()))?;
        let id = unsafe { read_string(id_ptr, id_len) };

        match state.oplog.get_by_id(&id) {
            Some(op) => Ok(op.clone()),
            None => Err(("Operation not found".to_string(), "NOT_FOUND".to_string()))
        }
    });

    match result {
        Ok(data) => {
            set_result(&ok_response(data));
            0
        }
        Err((msg, code)) => {
            set_result(&error_response(&msg, &code));
            -1
        }
    }
}

/// visible 操作取得
/// 出力: {ok: true, data: [Operation, ...]}
#[no_mangle]
pub extern "C" fn rad_get_visible(path_ptr: *const u8, path_len: usize) -> i32 {
    STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();
        let path = unsafe { read_string(path_ptr, path_len) };

        let ops: Vec<_> = state.oplog.get_all_operations()
            .into_iter()
            .filter(|op| {
                op.region_id.starts_with(&path) && matches!(op.status, OpStatus::Visible)
            })
            .collect();

        set_result(&ok_response(ops));
    });
    0
}

/// ファイル一覧取得
/// 出力: {ok: true, data: [String, ...]}
#[no_mangle]
pub extern "C" fn rad_get_file_list() -> i32 {
    STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();

        // Get unique file paths from accepted operations
        let mut files = std::collections::HashSet::new();
        for op in state.oplog.get_all_operations() {
            if matches!(op.status, OpStatus::Accepted) {
                if let Some(file_path) = op.region_id.split(':').next() {
                    files.insert(file_path.to_string());
                }
            }
        }

        let file_list: Vec<String> = files.into_iter().collect();
        set_result(&ok_response(file_list));
    });
    0
}
