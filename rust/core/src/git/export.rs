use std::path::Path;
use std::process::Command;
use std::fs;
use std::collections::HashMap;
use crate::store::RadStore;
use crate::types::OpStatus;

#[derive(Debug)]
pub struct ExportResult {
    pub commit_count: usize,
    pub operation_count: usize,
}

pub fn export_to_git(project_dir: &Path) -> Result<ExportResult, String> {
    // Open RadStore
    let store = RadStore::open(project_dir)
        .map_err(|e| format!("failed to open RadStore: {}", e))?;

    // Load oplog
    let oplog = store.load_oplog()
        .map_err(|e| format!("failed to load oplog: {}", e))?;

    // Get all accepted operations
    let accepted_ops: Vec<_> = oplog.all()
        .iter()
        .filter(|op| matches!(op.status, OpStatus::Accepted))
        .cloned()
        .collect();

    if accepted_ops.is_empty() {
        return Ok(ExportResult {
            commit_count: 0,
            operation_count: 0,
        });
    }

    // Group operations by timestamp (within 1 second window)
    let mut groups: Vec<Vec<_>> = vec![];
    let mut current_group = vec![];
    let mut last_timestamp = 0u64;

    for op in accepted_ops.iter() {
        if current_group.is_empty() || (op.timestamp - last_timestamp) <= 1000 {
            current_group.push(op);
            last_timestamp = op.timestamp;
        } else {
            groups.push(current_group);
            current_group = vec![op];
            last_timestamp = op.timestamp;
        }
    }
    if !current_group.is_empty() {
        groups.push(current_group);
    }

    let mut commit_count = 0;
    let mut operation_count = 0;

    // Create commits for each group
    for group in groups {
        // Write files to working tree
        for op in group.iter() {
            // Extract file path from region_id (format: "file:start-end")
            let region_parts: Vec<&str> = op.region_id.split(':').collect();
            if region_parts.len() >= 1 {
                let file_path = region_parts[0];
                let full_path = project_dir.join(file_path);

                // Create parent directories
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).ok();
                }

                // Write content
                fs::write(&full_path, &op.content)
                    .map_err(|e| format!("failed to write file {}: {}", file_path, e))?;

                operation_count += 1;
            }
        }

        // Stage changes
        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(project_dir)
            .output()
            .map_err(|e| format!("git add failed: {}", e))?;

        if !add_output.status.success() {
            return Err("git add failed".to_string());
        }

        // Check if there are changes to commit
        let status_output = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(project_dir)
            .status()
            .map_err(|e| format!("git diff failed: {}", e))?;

        // If exit code is 0, there are no changes
        if status_output.success() {
            continue;
        }

        // Collect participants
        let mut participants: Vec<String> = vec![];
        let mut participant_set = std::collections::HashSet::new();
        for op in group.iter() {
            if participant_set.insert(&op.participant_id) {
                participants.push(op.participant_id.clone());
            }
        }

        // Create commit message
        let participant_list = participants.join(", ");
        let message = format!("rad: {} accepted writes by {}", group.len(), participant_list);

        // Add Co-authored-by if multiple participants
        let mut full_message = message.clone();
        if participants.len() > 1 {
            for participant in participants.iter().skip(1) {
                full_message.push_str(&format!("\n\nCo-authored-by: {} <{}@rad.local>",
                    participant, participant));
            }
        }

        // Create commit
        let commit_output = Command::new("git")
            .args(["commit", "-m", &full_message])
            .current_dir(project_dir)
            .output()
            .map_err(|e| format!("git commit failed: {}", e))?;

        if !commit_output.status.success() {
            return Err(format!("git commit failed: {}",
                String::from_utf8_lossy(&commit_output.stderr)));
        }

        commit_count += 1;
    }

    Ok(ExportResult {
        commit_count,
        operation_count,
    })
}
