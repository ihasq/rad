use crate::remote::RemoteClient;
use crate::store::RadStore;
use std::fs;

#[derive(serde::Deserialize, serde::Serialize)]
struct RemoteConfig {
    url: String,
    #[serde(rename = "lastPullTimestamp")]
    last_pull_timestamp: u64,
}

pub fn run_pull(store: &RadStore) -> Result<String, String> {
    // Load remote.json
    let remote_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .join(".rad/remote.json");

    let remote_json = fs::read_to_string(&remote_path)
        .map_err(|e| format!("Failed to read remote.json: {}", e))?;

    let mut remote_config: RemoteConfig = serde_json::from_str(&remote_json)
        .map_err(|e| format!("Failed to parse remote.json: {}", e))?;

    let client = RemoteClient::new(&remote_config.url);

    // Get new operations since last pull
    let new_ops = client.get_log(Some(remote_config.last_pull_timestamp))?;

    if new_ops.is_empty() {
        return Ok("pulled: 0 operations (already up to date)\n".to_string());
    }

    // Load existing oplog
    let mut oplog = store.load_oplog()?;
    let existing_ids: std::collections::HashSet<String> = oplog.get_all_operations()
        .iter()
        .map(|o| o.id.clone())
        .collect();

    // Add new operations (deduplication by ID)
    let mut added = Vec::new();
    for op in new_ops {
        if !existing_ids.contains(&op.id) {
            added.push((op.id.clone(), op.status.clone(), op.participant_id.clone(), op.region_id.clone()));
            oplog.add_operation(op.clone());
        }
    }

    if added.is_empty() {
        return Ok("pulled: 0 new operations (duplicates filtered)\n".to_string());
    }

    // Save updated oplog
    store.save_oplog(&oplog)?;

    // Update lastPullTimestamp
    let max_timestamp = oplog.get_all_operations()
        .iter()
        .map(|o| o.timestamp)
        .max()
        .unwrap_or(remote_config.last_pull_timestamp);

    remote_config.last_pull_timestamp = max_timestamp;

    fs::write(
        &remote_path,
        serde_json::to_string(&remote_config).unwrap()
    ).map_err(|e| format!("Failed to update remote.json: {}", e))?;

    // Build output
    let mut output = String::new();
    output.push_str(&format!("pulled: {} operations from {}\n", added.len(), remote_config.url));

    for (id, status, participant, region_id) in &added {
        // Extract file from region_id
        let file = if let Some(colon_pos) = region_id.find(':') {
            &region_id[..colon_pos]
        } else {
            region_id.as_str()
        };

        output.push_str(&format!("  {} [{:?}] {} by {}\n", id, status, file, participant));
    }

    Ok(output)
}
