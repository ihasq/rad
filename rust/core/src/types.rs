use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub id: String,
    pub public_key: String,
    pub display_name: Option<String>,
    pub joined_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRegion {
    pub id: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub owner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OpType {
    Write,
    Approve,
    Reject,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OpStatus {
    Visible,
    Accepted,
    Rejected,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub participant_id: String,
    pub region_id: String,
    #[serde(rename = "type")]
    pub op_type: OpType,
    pub content: String,
    pub reason: Option<String>,
    pub signature: String,
    pub timestamp: u64,
    pub status: OpStatus,
}
