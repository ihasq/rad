use crate::remote::RemoteClient;
use crate::store::RadStore;
use std::fs;

#[derive(serde::Deserialize)]
struct RemoteConfig {
    url: String,
}

pub fn run_push(store: &RadStore) -> Result<String, String> {
    // Load remote.json
    let remote_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .join(".rad/remote.json");

    let remote_json = fs::read_to_string(&remote_path)
        .map_err(|e| format!("Failed to read remote.json: {}", e))?;

    let remote_config: RemoteConfig = serde_json::from_str(&remote_json)
        .map_err(|e| format!("Failed to parse remote.json: {}", e))?;

    let client = RemoteClient::new(&remote_config.url);

    // Load operations
    let oplog = store.load_oplog()?;
    let operations = oplog.get_all_operations();

    // Track pushed and skipped operations
    let mut pushed_ops = Vec::new();
    let mut skipped = 0;

    // Submit each operation
    for op in operations {
        // Skip if already accepted (these were likely from clone/pull)
        // We only push operations that are local and not yet on the server
        // For simplicity, we'll try to push all operations and let the server handle duplicates

        match client.submit_operation(&op) {
            Ok(_) => {
                pushed_ops.push((op.id.clone(), op.status.clone(), op.region_id.clone()));
            }
            Err(e) => {
                // If it's a duplicate, that's okay (idempotency)
                if e.contains("409") || e.contains("already exists") {
                    skipped += 1;
                } else {
                    // Other errors we should report but not fail
                    eprintln!("Warning: Failed to push {}: {}", op.id, e);
                }
            }
        }
    }

    // Build output
    let mut output = String::new();
    output.push_str(&format!("pushed: {} operations to {}\n", pushed_ops.len(), remote_config.url));

    for (id, status, region_id) in &pushed_ops {
        // Extract file from region_id
        let file = if let Some(colon_pos) = region_id.find(':') {
            &region_id[..colon_pos]
        } else {
            region_id.as_str()
        };

        output.push_str(&format!("  {} [{:?}] {}\n", id, status, file));
    }

    if skipped > 0 {
        output.push_str(&format!("  ({} operations already on server)\n", skipped));
    }

    Ok(output)
}
