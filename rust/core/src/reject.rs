use crate::types::OpStatus;
use crate::oplog::OpLog;
use crate::region::RegionMap;
use crate::founder::FounderTree;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RejectResult {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub status: String,
    #[serde(rename = "rejectedBy")]
    pub rejected_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub fn handle_reject(
    op_id: &str,
    rejecter_id: &str,
    reason: Option<&str>,
    region_map: &RegionMap,
    founder_tree: &FounderTree,
    oplog: &mut OpLog,
) -> Result<RejectResult, String> {
    // まずデータを取得してクローン
    let (region_id, participant_id, file_path) = {
        let op = oplog.get_by_id(op_id)
            .ok_or_else(|| "Operation not found".to_string())?;

        // ステータス検証
        if op.status != OpStatus::Visible {
            return Err("Cannot reject: not visible".to_string());
        }

        let region = region_map.get_by_id(&op.region_id)
            .ok_or_else(|| "Region not found".to_string())?;

        (op.region_id.clone(), op.participant_id.clone(), region.file_path.clone())
    };

    // Founder 階層チェック: 下位 Founder が上位 Founder を reject する場合は reason 必須
    // Get the directory where the operation's file is located
    let op_dir = match file_path.rfind('/') {
        Some(pos) => &file_path[..pos],
        None => ".",
    };

    // Find which directory the rejecter is founder of
    let rejecter_founder_dir = {
        let all_founders = founder_tree.list_all();
        all_founders.iter()
            .find(|(_, f)| *f == rejecter_id)
            .map(|(d, _)| d.to_string())
    };

    if let Some(rej_dir) = rejecter_founder_dir {
        // If rejecter is lower in hierarchy (rej_dir is descendant of op_dir), reason is required
        if founder_tree.is_ancestor_founder(op_dir, &rej_dir) {
            // Rejecter is lower-level founder, reason required
            if reason.is_none() || reason.unwrap().is_empty() {
                return Err("Lower Founder must provide reason to reject upper Founder".to_string());
            }
        }
    }

    // Leader → Follower (region-based): reason 必須
    let owner = region_map.get_owner_by_region_id(&region_id).unwrap_or("");
    if owner == rejecter_id && participant_id != rejecter_id {
        // rejecter は Leader、対象は Follower
        if reason.is_none() || reason.unwrap().is_empty() {
            return Err("Leader must provide reason to reject Follower".to_string());
        }
    }

    // reject 実行
    oplog.set_status(op_id, OpStatus::Rejected);

    Ok(RejectResult {
        operation_id: op_id.to_string(),
        status: "rejected".to_string(),
        rejected_by: rejecter_id.to_string(),
        reason: reason.map(|s| s.to_string()),
    })
}
