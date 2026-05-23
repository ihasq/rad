use crate::types::{Operation, OpType, OpStatus};
use crate::oplog::OpLog;
use crate::founder::FounderTree;
use crate::sign;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DeleteResult {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub status: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
}

/// delete 操作を処理する
/// file_path: 削除対象のファイルパス
/// participant: 削除を提案する参加者ID
/// secret_key: 署名用の秘密鍵
/// founder_tree: Founder 階層
/// oplog: 操作ログ
pub fn handle_delete(
    file_path: &str,
    participant: &str,
    secret_key: &str,
    founder_tree: &FounderTree,
    oplog: &mut OpLog,
) -> Result<DeleteResult, String> {
    // region_id はファイル全体を表す特別なID（例: "file:src/main.ts"）
    let region_id = format!("file:{}", file_path);

    // Operation 生成
    let op_id = format!("op-{}-{}", participant, oplog.len());
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let mut operation = Operation {
        id: op_id.clone(),
        participant_id: participant.to_string(),
        region_id: region_id.clone(),
        op_type: OpType::Delete,
        content: String::new(), // delete は内容なし
        reason: None,
        signature: String::new(),
        timestamp,
        status: OpStatus::Visible, // delete は visible で提出される
    };

    // JSON 正規化 → 署名
    let op_json = serde_json::to_string(&operation).map_err(|e| e.to_string())?;
    let signature = sign::sign_operation(&op_json, secret_key);
    operation.signature = signature;

    // oplog に追加
    oplog.add_operation(operation);

    Ok(DeleteResult {
        operation_id: op_id,
        status: "visible".to_string(),
        file_path: file_path.to_string(),
    })
}

/// delete 操作を accept できるかチェック
pub fn can_accept_delete(
    file_path: &str,
    accepter: &str,
    founder_tree: &FounderTree,
) -> Result<(), String> {
    match founder_tree.get_file_founder(file_path) {
        Some(file_founder) => {
            if file_founder == accepter {
                // accepter がファイルの Founder → accept 可能
                Ok(())
            } else {
                Err(format!(
                    "Only file founder '{}' can accept delete for '{}'",
                    file_founder, file_path
                ))
            }
        }
        None => Err(format!("No file founder found for '{}'", file_path)),
    }
}
