use crate::types::{CodeRegion, Operation, OpType, OpStatus};
use crate::region::RegionMap;
use crate::oplog::OpLog;
use crate::sign;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn handle_write(
    parts: &[&str],
    region_map: &mut RegionMap,
    oplog: &mut OpLog,
) -> String {
    // parts: write <file> <start> <end> <participant> <secret-key> "<content>"
    let file = parts[1];
    let start: u32 = parts[2].parse().unwrap();
    let end: u32 = parts[3].parse().unwrap();
    let participant = parts[4];
    let secret_key = parts[5];
    let content = parts[6..].join(" ").trim_matches('"').to_string();
    let region_id = format!("{}:{}-{}", file, start, end);

    // 未登録領域なら自動登録（書き込み者が Leader）
    let region = CodeRegion {
        id: region_id.clone(),
        file_path: file.to_string(),
        start_line: start,
        end_line: end,
        owner_id: participant.to_string(),
    };
    region_map.register(region); // 既存なら無視

    // Operation 生成 + 署名
    let timestamp = current_timestamp_ms();
    let seq = oplog.len();
    let op_id = format!("op-{}-{}", timestamp, seq);
    let mut op = Operation {
        id: op_id.clone(),
        participant_id: participant.to_string(),
        region_id: region_id.clone(),
        op_type: OpType::Write,
        content: content.clone(),
        reason: None,
        signature: String::new(),
        timestamp,
        status: OpStatus::Visible,
    };

    // JSON 正規化 → 署名
    let op_json = serde_json::to_string(&op).unwrap();
    let sig = sign::sign_operation(&op_json, secret_key);
    op.signature = sig;

    oplog.append(op.clone());

    // 出力: JSON with status + chain
    let chain: Vec<String> = oplog.get_chain(file, start, end)
        .iter().map(|o| o.id.clone()).collect();
    format!(r#"{{"id":"{}","status":"visible","chain":{}}}"#,
        op.id, serde_json::to_string(&chain).unwrap())
}

pub fn handle_chain(parts: &[&str], oplog: &OpLog) -> String {
    let file = parts[1];
    let start: u32 = parts[2].parse().unwrap();
    let end: u32 = parts[3].parse().unwrap();

    // 通常の領域チェーンに加え、ファイル全体の操作（delete等）も含める
    let region_id = format!("{}:{}-{}", file, start, end);
    let file_prefix = format!("file:{}", file);

    let mut chain: Vec<&Operation> = oplog.all()
        .iter()
        .filter(|op| op.region_id == region_id || op.region_id == file_prefix)
        .collect();
    chain.sort_by_key(|op| op.timestamp);

    let chain = chain; // 既存コードとの互換性維持

    // ステータスカウント
    let visible_count = chain.iter().filter(|op| op.status == OpStatus::Visible).count();
    let all_visible = visible_count == chain.len();

    // ヘッダ
    let mut result = if all_visible {
        format!("{}:{}-{} ({} writes, all visible)\n", file, start, end, chain.len())
    } else {
        format!("{}:{}-{} ({} writes)\n", file, start, end, chain.len())
    };

    // 各操作の1行表示
    for op in chain {
        let status_str = match op.status {
            OpStatus::Visible => "visible",
            OpStatus::Accepted => "accepted",
            OpStatus::Rejected => "rejected",
            OpStatus::Discarded => "discarded",
        };
        let type_str = match op.op_type {
            OpType::Write => "write",
            OpType::Approve => "approve",
            OpType::Reject => "reject",
            OpType::Delete => "delete",
        };
        result.push_str(&format!("  {} [{}] [{}] {}  t={}  \"{}\"\n",
            op.id, status_str, type_str, op.participant_id, op.timestamp, op.content));
    }

    result.trim_end().to_string()
}
