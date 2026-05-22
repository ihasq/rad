use std::path::Path;
use std::process::Command;
use crate::store::RadStore;
use crate::types::{Operation, OpType, OpStatus, CodeRegion, Participant};
use crate::oplog::OpLog;
use crate::region::RegionMap;

#[derive(Debug)]
pub struct ImportResult {
    pub commit_count: usize,
    pub operation_count: usize,
    pub participant_count: usize,
}

pub fn import_from_git(project_dir: &Path) -> Result<ImportResult, String> {
    // Open RadStore
    let store = RadStore::open(project_dir)
        .map_err(|e| format!("failed to open RadStore: {}", e))?;

    // Load existing state
    let mut oplog = match store.load_oplog() {
        Ok(log) => log,
        Err(_) => OpLog::new(),
    };
    let mut region_map = store.load_regions();

    // Track participants
    let mut participants: Vec<Participant> = vec![];
    let mut participant_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Get git log
    let log_output = Command::new("git")
        .args(["log", "--format=%H|%ae|%an|%at|%s", "--reverse", "--name-status"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("git log failed: {}", e))?;

    if !log_output.status.success() {
        return Err("git log command failed".to_string());
    }

    let log_str = String::from_utf8_lossy(&log_output.stdout);
    let lines: Vec<&str> = log_str.lines().collect();

    let mut commit_count = 0;
    let mut operation_count = 0;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Parse commit line
        if line.contains('|') {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 5 {
                let commit_hash = parts[0];
                let author_email = parts[1];
                let author_name = parts[2];
                let timestamp = parts[3].parse::<u64>().unwrap_or(0) * 1000; // Convert to ms
                let _message = parts[4];

                // Register participant
                let participant_id = author_name.to_string();
                if !participant_map.contains_key(&participant_id) {
                    participant_map.insert(participant_id.clone(), author_email.to_string());
                    participants.push(Participant {
                        id: participant_id.clone(),
                        public_key: format!("git-import-{}", commit_hash[..7].to_string()),
                        display_name: Some(author_name.to_string()),
                        joined_at: timestamp,
                    });
                }

                // Get changed files from following lines
                i += 1;

                // Skip empty line after commit info
                if i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }

                while i < lines.len() {
                    let file_line = lines[i].trim();

                    // Stop at next commit or empty line
                    if file_line.is_empty() || file_line.contains('|') {
                        break;
                    }

                    // Parse file status line (e.g., "M	src/main.ts" or "A	src/utils.ts")
                    let file_parts: Vec<&str> = file_line.split_whitespace().collect();
                    if file_parts.len() >= 2 {
                        let _status = file_parts[0]; // M, A, D, etc.
                        let file_path = file_parts[1];

                        // Get file content at this commit
                        let content_output = Command::new("git")
                            .args(["show", &format!("{}:{}", commit_hash, file_path)])
                            .current_dir(project_dir)
                            .output()
                            .ok();

                        let content = if let Some(output) = content_output {
                            if output.status.success() {
                                String::from_utf8_lossy(&output.stdout).to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };

                        if !content.is_empty() {
                            // Calculate line count
                            let line_count = content.lines().count().max(1) as u32;
                            let region_id = format!("{}:1-{}", file_path, line_count);

                            // Register region
                            let region = CodeRegion {
                                id: region_id.clone(),
                                file_path: file_path.to_string(),
                                start_line: 1,
                                end_line: line_count,
                                owner_id: participant_id.clone(),
                            };
                            region_map.register(region);

                            // Create operation
                            let op_id = format!("op-{}-{}", timestamp, operation_count);
                            let operation = Operation {
                                id: op_id,
                                participant_id: participant_id.clone(),
                                region_id: region_id.clone(),
                                op_type: OpType::Write,
                                content: content.clone(),
                                reason: None,
                                signature: "git-imported".to_string(),
                                timestamp,
                                status: OpStatus::Accepted,
                            };

                            oplog.append(operation);
                            operation_count += 1;
                        }
                    }

                    i += 1;
                }

                commit_count += 1;
                continue;
            }
        }

        i += 1;
    }

    // Save state
    store.save_oplog(&oplog)
        .map_err(|e| format!("failed to save oplog: {}", e))?;
    store.save_regions(&region_map)
        .map_err(|e| format!("failed to save regions: {}", e))?;
    store.save_participants(&participants)
        .map_err(|e| format!("failed to save participants: {}", e))?;

    Ok(ImportResult {
        commit_count,
        operation_count,
        participant_count: participants.len(),
    })
}
