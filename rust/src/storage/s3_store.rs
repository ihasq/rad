use std::sync::Arc;
use std::collections::HashMap;
use crate::oplog::OpLog;
use crate::region::RegionMap;
use crate::founder::FounderTree;
use crate::types::{Operation, OpStatus, OpType, CodeRegion, Participant};
use super::backend::RadStorageBackend;

/// S3-backed RadStore implementation.
/// Uses RadStorageBackend abstraction for storage operations.
///
/// S3 bucket structure:
///   rad/
///     config.json
///     participants.json
///     oplog/
///       {timestamp}-{id}.json    ← individual operations
///       _index.json              ← operation ID → key mapping
///     regions.json
///     founders.json
///     snapshots/
///       src/main.rs
///       ...
pub struct S3RadStore {
    backend: Arc<dyn RadStorageBackend>,
}

impl S3RadStore {
    pub fn new(backend: Arc<dyn RadStorageBackend>) -> Self {
        Self { backend }
    }

    /// Load all operations from S3
    pub async fn load_oplog(&self) -> Result<OpLog, String> {
        let keys = self.backend.list("rad/oplog/").await?;
        let mut operations = Vec::new();

        for key in keys {
            // Skip the index file
            if key.ends_with("_index.json") {
                continue;
            }

            if let Some(data) = self.backend.get(&key).await? {
                if let Ok(op) = serde_json::from_str::<Operation>(&data) {
                    operations.push(op);
                }
            }
        }

        // Sort by timestamp
        operations.sort_by_key(|op| op.timestamp);

        let mut oplog = OpLog::new();
        for op in operations {
            oplog.add_operation(op);
        }
        Ok(oplog)
    }

    /// Save operations to S3
    /// Each operation is stored as a separate object
    pub async fn save_oplog(&self, oplog: &OpLog) -> Result<(), String> {
        let operations = oplog.get_all_operations();
        let mut index = HashMap::new();

        for op in operations {
            let key = format!("rad/oplog/{}-{}.json", op.timestamp, op.id);
            let json = serde_json::to_string(&op).map_err(|e| e.to_string())?;
            self.backend.put(&key, &json).await?;
            index.insert(op.id.clone(), key);
        }

        // Save index
        let index_json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
        self.backend.put("rad/oplog/_index.json", &index_json).await?;

        Ok(())
    }

    /// Append a single operation to S3
    pub async fn append_op(&self, op: &Operation) -> Result<(), String> {
        let key = format!("rad/oplog/{}-{}.json", op.timestamp, op.id);
        let json = serde_json::to_string(&op).map_err(|e| e.to_string())?;
        self.backend.put(&key, &json).await?;

        // Update index
        let index_data = self.backend.get("rad/oplog/_index.json").await?;
        let mut index: HashMap<String, String> = if let Some(data) = index_data {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };

        index.insert(op.id.clone(), key);
        let index_json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
        self.backend.put("rad/oplog/_index.json", &index_json).await?;

        Ok(())
    }

    pub async fn load_regions(&self) -> Result<RegionMap, String> {
        let data = self.backend.get("rad/regions.json").await?;
        if data.is_none() {
            return Ok(RegionMap::new());
        }

        let regions: Vec<CodeRegion> = serde_json::from_str(&data.unwrap())
            .unwrap_or_default();

        let mut map = RegionMap::new();
        for region in regions {
            map.register(region);
        }
        Ok(map)
    }

    pub async fn save_regions(&self, region_map: &RegionMap) -> Result<(), String> {
        let regions = region_map.get_all_regions();
        let json = serde_json::to_string(&regions).map_err(|e| e.to_string())?;
        self.backend.put("rad/regions.json", &json).await?;
        Ok(())
    }

    pub async fn load_participants(&self) -> Result<Vec<Participant>, String> {
        let data = self.backend.get("rad/participants.json").await?;
        if data.is_none() {
            return Ok(Vec::new());
        }

        let participants: Vec<Participant> = serde_json::from_str(&data.unwrap())
            .unwrap_or_default();
        Ok(participants)
    }

    pub async fn save_participants(&self, participants: &[Participant]) -> Result<(), String> {
        let json = serde_json::to_string(participants).map_err(|e| e.to_string())?;
        self.backend.put("rad/participants.json", &json).await?;
        Ok(())
    }

    pub async fn load_founders(&self) -> Result<FounderTree, String> {
        // Load root founder from config
        let root_founder = if let Some(config_data) = self.backend.get("rad/config.json").await? {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_data) {
                config.get("founder")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Load founders map
        if let Some(founders_data) = self.backend.get("rad/founders.json").await? {
            return Ok(FounderTree::from_json(&founders_data, &root_founder));
        }

        Ok(FounderTree::new(&root_founder))
    }

    pub async fn save_founders(&self, tree: &FounderTree) -> Result<(), String> {
        let json = tree.to_json();
        self.backend.put("rad/founders.json", &json).await?;
        Ok(())
    }

    pub async fn put_snapshot(&self, file_path: &str, content: &str) -> Result<(), String> {
        let key = format!("rad/snapshots/{}", file_path);
        self.backend.put(&key, content).await?;
        Ok(())
    }

    pub async fn get_snapshot(&self, file_path: &str) -> Result<Option<String>, String> {
        let key = format!("rad/snapshots/{}", file_path);
        self.backend.get(&key).await
    }

    /// Compact operation log
    /// Moves accepted writes to snapshots and removes them from oplog
    pub async fn compact(&self) -> Result<(), String> {
        let oplog = self.load_oplog().await?;
        let operations = oplog.get_all_operations();

        // Group operations by file path and collect accepted writes
        let mut file_contents: HashMap<String, String> = HashMap::new();
        let mut accepted_ids: Vec<String> = Vec::new();

        for op in &operations {
            if op.status == OpStatus::Accepted && matches!(op.op_type, OpType::Write) {
                // Extract file path from region_id (format: "file:start-end")
                if let Some(colon_pos) = op.region_id.find(':') {
                    let file_path = &op.region_id[..colon_pos];
                    file_contents.insert(file_path.to_string(), op.content.clone());
                    accepted_ids.push(op.id.clone());
                }
            }
        }

        // Write snapshots
        for (file_path, content) in file_contents {
            self.put_snapshot(&file_path, &content).await?;
        }

        // Remove accepted operations from S3
        for op in &operations {
            if accepted_ids.contains(&op.id) {
                let key = format!("rad/oplog/{}-{}.json", op.timestamp, op.id);
                self.backend.delete(&key).await?;
            }
        }

        // Update index to remove deleted operations
        if let Some(index_data) = self.backend.get("rad/oplog/_index.json").await? {
            let mut index: HashMap<String, String> = serde_json::from_str(&index_data)
                .unwrap_or_default();

            for id in accepted_ids {
                index.remove(&id);
            }

            let index_json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
            self.backend.put("rad/oplog/_index.json", &index_json).await?;
        }

        Ok(())
    }
}
