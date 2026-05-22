use std::fs;
use std::path::Path;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct InitResult {
    pub founder: String,
    pub public_key: String,
}

pub fn init_project(
    dir: &Path,
    participant_id: &str,
    public_key: &str,
) -> Result<InitResult, String> {
    let rad_dir = dir.join(".rad");
    if rad_dir.exists() {
        return Err("Already initialized".to_string());
    }

    fs::create_dir_all(&rad_dir).map_err(|e| e.to_string())?;

    // config.json
    let config = serde_json::json!({
        "founder": participant_id,
        "publicKey": public_key
    });
    fs::write(rad_dir.join("config.json"), serde_json::to_string(&config).unwrap())
        .map_err(|e| e.to_string())?;

    // participants.json
    let participants = serde_json::json!([{
        "id": participant_id,
        "publicKey": public_key
    }]);
    fs::write(rad_dir.join("participants.json"), serde_json::to_string(&participants).unwrap())
        .map_err(|e| e.to_string())?;

    // empty oplog + regions
    fs::write(rad_dir.join("oplog.json"), "[]").map_err(|e| e.to_string())?;
    fs::write(rad_dir.join("regions.json"), "[]").map_err(|e| e.to_string())?;

    // founders.json with root founder
    let founders = serde_json::json!({
        ".": participant_id
    });
    fs::write(rad_dir.join("founders.json"), serde_json::to_string(&founders).unwrap())
        .map_err(|e| e.to_string())?;

    Ok(InitResult {
        founder: participant_id.to_string(),
        public_key: public_key.to_string(),
    })
}
