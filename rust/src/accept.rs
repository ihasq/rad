use crate::types::{OpStatus, OpType};
use crate::oplog::OpLog;
use crate::region::RegionMap;
use crate::founder::FounderTree;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AcceptResult {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub status: String,
    #[serde(rename = "acceptedBy")]
    pub accepted_by: String,
}

pub fn handle_accept(
    op_id: &str,
    leader_id: &str,
    region_map: &RegionMap,
    founder_tree: &FounderTree,
    oplog: &mut OpLog,
) -> Result<AcceptResult, String> {
    // まずデータを取得してクローン
    let (region_id, op_type) = {
        let op = oplog.get_by_id(op_id)
            .ok_or_else(|| "Operation not found".to_string())?;

        // ステータス検証
        if op.status != OpStatus::Visible {
            return Err(format!("Cannot accept: status is {:?}", op.status));
        }

        (op.region_id.clone(), op.op_type.clone())
    };

    // Delete の場合は file founder 検証
    if op_type == OpType::Delete {
        // region_id から file_path を抽出（"file:src/main.ts" → "src/main.ts"）
        let file_path = region_id.strip_prefix("file:").unwrap_or(&region_id);

        // file founder のみが delete を accept できる
        if !founder_tree.can_accept_delete(file_path, leader_id) {
            return Err(format!("Only file founder can accept delete for '{}'", file_path));
        }
    } else {
        // 通常の write/reject の Leader 検証
        let owner = region_map.get_owner_by_region_id(&region_id)
            .ok_or_else(|| "Region not found".to_string())?;
        if owner != leader_id {
            return Err("Only the Leader can accept".to_string());
        }
    }

    // accept 実行
    oplog.set_status(op_id, OpStatus::Accepted);

    // 階段飛ばし: チェーン内で op より前の visible を discard
    let to_discard: Vec<String> = {
        let chain = oplog.get_chain_by_region_id(&region_id);
        chain.iter()
            .take_while(|c| c.id != op_id)
            .filter(|c| c.status == OpStatus::Visible && c.participant_id != leader_id)
            .map(|c| c.id.clone())
            .collect()
    };

    for id in to_discard {
        oplog.set_status(&id, OpStatus::Discarded);
    }

    Ok(AcceptResult {
        operation_id: op_id.to_string(),
        status: "accepted".to_string(),
        accepted_by: leader_id.to_string(),
    })
}
