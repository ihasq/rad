use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    pub public_key: String,
    pub display_name: Option<String>,
    pub joined_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRegion {
    pub id: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub owner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpType {
    Write,
    Approve,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub participant_id: String,
    pub region_id: String,
    pub op_type: OpType,
    pub content: String,
    pub reason: Option<String>,
    pub signature: String,
    pub timestamp: u64,
}
