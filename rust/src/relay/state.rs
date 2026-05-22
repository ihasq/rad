use std::sync::{Arc, Mutex};
use crate::oplog::OpLog;
use crate::region::RegionMap;
use crate::founder::FounderTree;
use crate::storage::S3RadStore;
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
    pub store: Option<Arc<S3RadStore>>,
}

impl RelayState {
    pub fn new() -> Self {
        Self {
            oplog: Mutex::new(OpLog::new()),
            region_map: Mutex::new(RegionMap::new()),
            founder_tree: Mutex::new(FounderTree::new("")),
            participants: Mutex::new(Vec::new()),
            op_counter: Mutex::new(0),
            store: None,
        }
    }

    pub async fn from_s3_store(store: Arc<S3RadStore>) -> Result<Self, String> {
        // Load data from S3
        let oplog = store.load_oplog().await?;
        let region_map = store.load_regions().await?;
        let founder_tree = store.load_founders().await?;
        let types_participants = store.load_participants().await?;

        // Convert types::Participant to state::Participant
        // For now, assume all participants loaded from S3 are not founders
        // (founder status is determined by founder tree)
        let participants: Vec<Participant> = types_participants
            .into_iter()
            .map(|p| Participant {
                participant_id: p.id,
                public_key: p.public_key,
                display_name: p.display_name,
                is_founder: false,  // This will be determined by founder tree
                joined_at: p.joined_at,
            })
            .collect();

        Ok(Self {
            oplog: Mutex::new(oplog),
            region_map: Mutex::new(region_map),
            founder_tree: Mutex::new(founder_tree),
            participants: Mutex::new(participants),
            op_counter: Mutex::new(0),
            store: Some(store),
        })
    }
}

pub type SharedState = Arc<RelayState>;
