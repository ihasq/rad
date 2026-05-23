use std::cell::RefCell;
use rad_core::{oplog::OpLog, region::RegionMap, founder::FounderTree, types::*};

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
    set_result(r#"{"status":"initialized"}"#);
    0
}

/// 参加者登録
/// 入力: {"publicKey": "...", "displayName": "..."}
/// 出力: {"participantId": "...", "joinedAt": 123}
#[no_mangle]
pub extern "C" fn rad_join(input_ptr: *const u8, input_len: usize) -> i32 {
    let input = unsafe { read_string(input_ptr, input_len) };

    let result: Result<String, String> = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().ok_or("State not initialized")?;

        // JSON パース
        #[derive(serde::Deserialize)]
        struct JoinRequest {
            #[serde(rename = "publicKey")]
            public_key: Option<String>,
            #[serde(rename = "displayName")]
            display_name: Option<String>,
        }

        let req: JoinRequest = serde_json::from_str(&input)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        // publicKey必須チェック
        let public_key = req.public_key.ok_or("publicKey is required")?;

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

        // レスポンス作成
        let response = serde_json::json!({
            "participantId": participant.id,
            "publicKey": public_key,
            "joinedAt": participant.joined_at
        });

        Ok(serde_json::to_string(&response).unwrap())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// 操作送信
/// 入力: Operation JSON
/// 出力: {"status": "visible", "id": "..."}
#[no_mangle]
pub extern "C" fn rad_submit_op(input_ptr: *const u8, input_len: usize) -> i32 {
    let input = unsafe { read_string(input_ptr, input_len) };

    let result = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().unwrap();

        // JSON パース
        let mut op: Operation = match serde_json::from_str(&input) {
            Ok(v) => v,
            Err(e) => return Err(format!("Invalid JSON: {}", e)),
        };

        // デフォルト値設定
        if op.id.is_empty() {
            op.id = format!("op-{}-{}", op.timestamp, state.oplog.len());
        }
        op.status = OpStatus::Visible;

        // OpLog に追加
        state.oplog.append(op.clone());

        // ストレージに保存
        let key = format!("operations/{}", op.id);
        let data = serde_json::to_string(&op).unwrap();
        let _ = state.backend.put(&key, &data);

        Ok(serde_json::json!({
            "status": "visible",
            "id": op.id
        }).to_string())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// Accept 操作
/// 入力: {"operationId": "...", "participantId": "..."}
/// 出力: {"status": "accepted"}
#[no_mangle]
pub extern "C" fn rad_accept(input_ptr: *const u8, input_len: usize) -> i32 {
    let input = unsafe { read_string(input_ptr, input_len) };

    let result = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let state = state.as_mut().unwrap();

        let json: serde_json::Value = match serde_json::from_str(&input) {
            Ok(v) => v,
            Err(_) => return Err("Invalid JSON".to_string()),
        };

        let op_id = json["operationId"].as_str().unwrap_or("");

        // ステータス更新
        state.oplog.set_status(op_id, OpStatus::Accepted);

        // ストレージに保存
        if let Some(op) = state.oplog.get_by_id(op_id) {
            let key = format!("operations/{}", op_id);
            let data = serde_json::to_string(&op).unwrap();
            let _ = state.backend.put(&key, &data);
        }

        Ok(r#"{"status":"accepted"}"#.to_string())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// 操作ログ取得
/// 入力: {} (empty or with filters)
/// 出力: [Operation, ...]
#[no_mangle]
pub extern "C" fn rad_get_log(_input_ptr: *const u8, _input_len: usize) -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();

        let ops = state.oplog.get_all_operations();
        Ok(serde_json::to_string(&ops).unwrap())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// コンパクション
#[no_mangle]
pub extern "C" fn rad_compact() -> i32 {
    set_result(r#"{"status":"compacted"}"#);
    0
}

/// 参加者一覧取得
#[no_mangle]
pub extern "C" fn rad_get_participants() -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().unwrap();

        Ok(serde_json::to_string(&state.participants).unwrap())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// ファイル取得
#[no_mangle]
pub extern "C" fn rad_get_file(path_ptr: *const u8, path_len: usize) -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or("State not initialized")?;
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

        let result_json = serde_json::json!({ "content": content });
        Ok(serde_json::to_string(&result_json).unwrap())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// コード領域取得
#[no_mangle]
pub extern "C" fn rad_get_regions(path_ptr: *const u8, path_len: usize) -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or("State not initialized")?;
        let path = unsafe { read_string(path_ptr, path_len) };

        let regions = state.region_map.list(&path);
        Ok(serde_json::to_string(&regions).unwrap())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// 操作ステータス取得
#[no_mangle]
pub extern "C" fn rad_get_op_status(id_ptr: *const u8, id_len: usize) -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or("State not initialized")?;
        let id = unsafe { read_string(id_ptr, id_len) };

        match state.oplog.get_by_id(&id) {
            Some(op) => {
                let status_json = serde_json::json!({
                    "status": format!("{:?}", op.status).to_lowercase()
                });
                Ok(serde_json::to_string(&status_json).unwrap())
            }
            None => Err("Operation not found".to_string())
        }
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// 操作取得
#[no_mangle]
pub extern "C" fn rad_get_op(id_ptr: *const u8, id_len: usize) -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or("State not initialized")?;
        let id = unsafe { read_string(id_ptr, id_len) };

        match state.oplog.get_by_id(&id) {
            Some(op) => Ok(serde_json::to_string(&op).unwrap()),
            None => Err("Operation not found".to_string())
        }
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// visible 操作取得
#[no_mangle]
pub extern "C" fn rad_get_visible(path_ptr: *const u8, path_len: usize) -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or("State not initialized")?;
        let path = unsafe { read_string(path_ptr, path_len) };

        let ops: Vec<_> = state.oplog.get_all_operations()
            .into_iter()
            .filter(|op| {
                op.region_id.starts_with(&path) && matches!(op.status, OpStatus::Visible)
            })
            .collect();

        Ok(serde_json::to_string(&ops).unwrap())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}

/// ファイル一覧取得
#[no_mangle]
pub extern "C" fn rad_get_file_list() -> i32 {
    let result: Result<String, String> = STATE.with(|s| {
        let state = s.borrow();
        let state = state.as_ref().ok_or("State not initialized")?;

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
        Ok(serde_json::to_string(&file_list).unwrap())
    });

    match result {
        Ok(json) => {
            set_result(&json);
            0
        }
        Err(e) => {
            set_result(&format!(r#"{{"error":"{}"}}"#, e));
            -1
        }
    }
}
