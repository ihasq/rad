use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use crate::oplog::OpLog;
use crate::region::RegionMap;
use crate::founder::FounderTree;
use crate::types::{Operation, OpStatus, CodeRegion};

pub struct RadStore {
    rad_dir: PathBuf,
}

impl RadStore {
    pub fn open(project_dir: &Path) -> Result<Self, String> {
        let rad_dir = project_dir.join(".rad");
        if !rad_dir.exists() {
            return Err("Not a rad project (run rad init first)".to_string());
        }
        Ok(Self { rad_dir })
    }

    pub fn load_oplog(&self) -> Result<OpLog, String> {
        let oplog_path = self.rad_dir.join("oplog.json");
        if !oplog_path.exists() {
            return Ok(OpLog::new());
        }

        let content = fs::read_to_string(&oplog_path)
            .map_err(|e| format!("error: failed to read oplog.json: {}", e))?;

        let ops = serde_json::from_str::<Vec<Operation>>(&content)
            .map_err(|e| format!("error: corrupt or invalid oplog.json: {}", e))?;

        let mut oplog = OpLog::new();
        for op in ops {
            oplog.add_operation(op);
        }
        Ok(oplog)
    }

    pub fn save_oplog(&self, oplog: &OpLog) -> Result<(), String> {
        let oplog_path = self.rad_dir.join("oplog.json");
        let ops = oplog.get_all_operations();
        let json = serde_json::to_string(&ops).map_err(|e| e.to_string())?;
        fs::write(&oplog_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_regions(&self) -> RegionMap {
        let regions_path = self.rad_dir.join("regions.json");
        if !regions_path.exists() {
            return RegionMap::new();
        }

        match fs::read_to_string(&regions_path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<CodeRegion>>(&content) {
                    Ok(regions) => {
                        let mut map = RegionMap::new();
                        for region in regions {
                            map.register(region);
                        }
                        map
                    }
                    Err(_) => RegionMap::new()
                }
            }
            Err(_) => RegionMap::new()
        }
    }

    pub fn save_regions(&self, region_map: &RegionMap) -> Result<(), String> {
        let regions_path = self.rad_dir.join("regions.json");
        let regions = region_map.get_all_regions();
        let json = serde_json::to_string(&regions).map_err(|e| e.to_string())?;
        fs::write(&regions_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_founders(&self) -> FounderTree {
        let config_path = self.rad_dir.join("config.json");
        let root_founder = if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                    config.get("founder")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let founders_path = self.rad_dir.join("founders.json");
        if founders_path.exists() {
            if let Ok(content) = fs::read_to_string(&founders_path) {
                return FounderTree::from_json(&content, &root_founder);
            }
        }
        FounderTree::new(&root_founder)
    }

    pub fn save_founders(&self, tree: &FounderTree) -> Result<(), String> {
        let founders_path = self.rad_dir.join("founders.json");
        let json = tree.to_json();
        fs::write(&founders_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn put_snapshot(&self, file_path: &str, content: &str) -> Result<(), String> {
        let snap_path = self.rad_dir.join("snapshots").join(file_path);
        if let Some(parent) = snap_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&snap_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_snapshot(&self, file_path: &str) -> Option<String> {
        let snap_path = self.rad_dir.join("snapshots").join(file_path);
        fs::read_to_string(&snap_path).ok()
    }

    pub fn compact(&self) -> Result<(), String> {
        let mut oplog = self.load_oplog()?;
        let ops = oplog.get_all_operations();

        // Group operations by file path and collect accepted writes
        let mut file_contents: HashMap<String, String> = HashMap::new();
        let mut accepted_ids: Vec<String> = Vec::new();

        for op in ops {
            if op.status == OpStatus::Accepted && matches!(op.op_type, crate::types::OpType::Write) {
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
            self.put_snapshot(&file_path, &content)?;
        }

        // Remove accepted operations from oplog
        for id in accepted_ids {
            oplog.remove_operation(&id);
        }

        self.save_oplog(&oplog)?;
        Ok(())
    }
}
