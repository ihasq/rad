use crate::remote::RemoteClient;
use crate::store::RadStore;
use crate::crypto;
use std::fs;
use std::path::Path;

pub fn run_clone(url: &str, participant: &str, secret_key: &str) -> Result<String, String> {
    let client = RemoteClient::new(url);

    // Generate public key from secret key
    let kp = crypto::keypair_from_secret(secret_key);
    let public_key = crypto::format_public_key(&kp);

    // 1. Join the relay
    client.join(participant, &public_key, false)?;

    // 2. Get all participants
    let participants = client.get_participants()?;

    // 3. Get all operations
    let operations = client.get_log(None)?;

    // 4. Create .rad directory
    let rad_dir = Path::new(".rad");
    fs::create_dir_all(rad_dir)
        .map_err(|e| format!("Failed to create .rad directory: {}", e))?;

    // 5. Initialize RadStore and save state
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    // Create config.json with founder (if we can determine it)
    let founder = if let Some(p) = participants.iter().find(|p| p.id == "alice") {
        p.id.clone()
    } else if let Some(p) = participants.first() {
        p.id.clone()
    } else {
        participant.to_string()
    };

    let config = serde_json::json!({
        "founder": founder,
    });
    fs::write(
        rad_dir.join("config.json"),
        serde_json::to_string(&config).unwrap()
    ).map_err(|e| format!("Failed to write config.json: {}", e))?;

    // Open store
    let store = RadStore::open(&cwd)?;

    // Save participants
    store.save_participants(&participants)?;

    // Save operations
    let mut oplog = crate::oplog::OpLog::new();
    for op in &operations {
        oplog.add_operation(op.clone());
    }
    store.save_oplog(&oplog)?;

    // Save remote.json
    let remote_config = serde_json::json!({
        "url": url,
        "lastPullTimestamp": if operations.is_empty() {
            0
        } else {
            operations.iter().map(|o| o.timestamp).max().unwrap_or(0)
        },
    });
    fs::write(
        rad_dir.join("remote.json"),
        serde_json::to_string(&remote_config).unwrap()
    ).map_err(|e| format!("Failed to write remote.json: {}", e))?;

    // Build response
    let mut output = String::new();
    output.push_str(&format!("cloned: rad project from {}\n", url));
    output.push_str(&format!("participants: {}\n", participants.len()));
    output.push_str(&format!("operations: {}\n", operations.len()));

    // Count unique files from operations
    let files: std::collections::HashSet<String> = operations.iter()
        .filter_map(|o| {
            if let Some(colon_pos) = o.region_id.find(':') {
                Some(o.region_id[..colon_pos].to_string())
            } else {
                None
            }
        })
        .collect();
    output.push_str(&format!("files: {}\n", files.len()));

    Ok(output)
}
