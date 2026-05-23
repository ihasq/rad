use crate::store::RadStore;
use crate::types::{OpStatus, OpType};
use std::collections::HashMap;

pub fn run_diff(store: &RadStore) -> Result<String, String> {
    let oplog = store.load_oplog()?;
    let ops = oplog.get_all_operations();

    // Get visible writes
    let visible_writes: Vec<_> = ops.iter()
        .filter(|o| o.status == OpStatus::Visible && o.op_type == OpType::Write)
        .collect();

    if visible_writes.is_empty() {
        return Ok("no pending changes\n".to_string());
    }

    // Build a map of accepted state (file -> latest accepted content)
    let mut accepted_state: HashMap<String, String> = HashMap::new();
    let mut ops_by_time = ops.clone();
    ops_by_time.sort_by_key(|o| o.timestamp);

    for op in &ops_by_time {
        if op.status == OpStatus::Accepted && op.op_type == OpType::Write {
            // Extract file path from region_id (format: "file:start-end")
            if let Some(colon_pos) = op.region_id.find(':') {
                let file_path = op.region_id[..colon_pos].to_string();
                accepted_state.insert(file_path, op.content.clone());
            }
        }
    }

    // Build output
    let mut output = String::new();
    for op in visible_writes {
        // Extract file path from region_id
        let file_path = if let Some(colon_pos) = op.region_id.find(':') {
            &op.region_id[..colon_pos]
        } else {
            &op.region_id
        };

        // Check if this is a new file
        let is_new_file = !accepted_state.contains_key(file_path);

        if is_new_file {
            output.push_str("--- (new file)\n");
            output.push_str(&format!("+++ visible by {} ({})\n", op.participant_id, op.id));
            output.push_str(&format!("+ {}  \"{}\"\n", op.region_id, op.content));
        } else {
            let accepted_content = accepted_state.get(file_path).unwrap();
            output.push_str(&format!("--- accepted ({})\n", file_path));
            output.push_str(&format!("+++ visible by {} ({})\n", op.participant_id, op.id));
            output.push_str(&format!("-{}\n", accepted_content));
            output.push_str(&format!("+{}\n", op.content));
        }
        output.push_str("\n");
    }

    Ok(output)
}
