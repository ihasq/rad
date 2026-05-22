use std::sync::{Arc, Mutex};
use crate::oplog::OpLog;
use crate::region::RegionMap;
use crate::founder::FounderTree;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    #[serde(rename = "participantId")]
    pub participant_id: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "isFounder")]
    pub is_founder: bool,
    #[serde(rename = "joinedAt")]
    pub joined_at: u64,
}

pub struct RelayState {
    pub oplog: Mutex<OpLog>,
    pub region_map: Mutex<RegionMap>,
    pub founder_tree: Mutex<FounderTree>,
    pub participants: Mutex<Vec<Participant>>,
    pub op_counter: Mutex<u64>,
}

impl RelayState {
    pub fn new() -> Self {
        Self {
            oplog: Mutex::new(OpLog::new()),
            region_map: Mutex::new(RegionMap::new()),
            founder_tree: Mutex::new(FounderTree::new("")),
            participants: Mutex::new(Vec::new()),
            op_counter: Mutex::new(0),
        }
    }
}

pub type SharedState = Arc<RelayState>;
