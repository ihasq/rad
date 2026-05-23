use crate::types::{Participant, Operation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JoinResponse {
    #[serde(rename = "participantId")]
    pub participant_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct SubmitResponse {
    pub id: String,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct AcceptResponse {
    pub id: String,
}

#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
}

pub struct RemoteClient {
    pub url: String,
}

impl RemoteClient {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
        }
    }

    pub fn join(&self, participant_id: &str, public_key: &str, is_founder: bool) -> Result<JoinResponse, String> {
        let body = serde_json::json!({
            "participantId": participant_id,
            "publicKey": public_key,
            "isFounder": is_founder,
        });

        let url = format!("{}/rad/participants", self.url);
        let resp = ureq::post(&url)
            .send_json(&body)
            .map_err(|e| format!("Failed to join: {}", e))?;

        let result: JoinResponse = resp.into_json()
            .map_err(|e| format!("Failed to parse join response: {}", e))?;

        Ok(result)
    }

    pub fn get_participants(&self) -> Result<Vec<Participant>, String> {
        let url = format!("{}/rad/participants", self.url);
        let resp = ureq::get(&url)
            .call()
            .map_err(|e| format!("Failed to get participants: {}", e))?;

        // The response is an array of participant objects
        #[derive(Deserialize)]
        struct ApiParticipant {
            #[serde(rename = "participantId")]
            participant_id: String,
            #[serde(rename = "publicKey")]
            public_key: String,
            #[serde(rename = "displayName")]
            display_name: Option<String>,
            #[serde(rename = "joinedAt")]
            joined_at: u64,
        }

        let api_participants: Vec<ApiParticipant> = resp.into_json()
            .map_err(|e| format!("Failed to parse participants: {}", e))?;

        let participants = api_participants.into_iter().map(|p| Participant {
            id: p.participant_id,
            public_key: p.public_key,
            display_name: p.display_name,
            joined_at: p.joined_at,
        }).collect();

        Ok(participants)
    }

    pub fn submit_operation(&self, operation: &Operation) -> Result<SubmitResponse, String> {
        // Convert Operation to JSON matching the API format
        let body = serde_json::json!({
            "id": operation.id,
            "participantId": operation.participant_id,
            "regionId": operation.region_id,
            "type": format!("{:?}", operation.op_type).to_lowercase(),
            "content": operation.content,
            "reason": operation.reason,
            "signature": operation.signature,
            "timestamp": operation.timestamp,
            "status": format!("{:?}", operation.status).to_lowercase(),
        });

        let url = format!("{}/rad/operations", self.url);
        let resp = ureq::post(&url)
            .send_json(&body)
            .map_err(|e| format!("Failed to submit operation: {}", e))?;

        let result: SubmitResponse = resp.into_json()
            .map_err(|e| format!("Failed to parse submit response: {}", e))?;

        Ok(result)
    }

    pub fn accept(&self, accept_json: &str) -> Result<AcceptResponse, String> {
        let url = format!("{}/rad/accept", self.url);
        let resp = ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_string(accept_json)
            .map_err(|e| format!("Failed to accept: {}", e))?;

        let result: AcceptResponse = resp.into_json()
            .map_err(|e| format!("Failed to parse accept response: {}", e))?;

        Ok(result)
    }

    pub fn get_log(&self, since: Option<u64>) -> Result<Vec<Operation>, String> {
        let mut url = format!("{}/rad/log", self.url);
        if let Some(ts) = since {
            url = format!("{}?since={}", url, ts);
        }

        let resp = ureq::get(&url)
            .call()
            .map_err(|e| format!("Failed to get log: {}", e))?;

        // Parse the JSON response
        #[derive(Deserialize)]
        struct ApiOperation {
            id: String,
            #[serde(rename = "participantId")]
            participant_id: String,
            #[serde(rename = "regionId")]
            region_id: String,
            #[serde(rename = "type")]
            op_type: String,
            content: String,
            reason: Option<String>,
            signature: String,
            timestamp: u64,
            status: String,
        }

        let api_ops: Vec<ApiOperation> = resp.into_json()
            .map_err(|e| format!("Failed to parse log: {}", e))?;

        let operations = api_ops.into_iter().map(|o| {
            use crate::types::{OpType, OpStatus};

            let op_type = match o.op_type.as_str() {
                "write" => OpType::Write,
                "approve" => OpType::Approve,
                "reject" => OpType::Reject,
                _ => OpType::Write,
            };

            let status = match o.status.as_str() {
                "visible" => OpStatus::Visible,
                "accepted" => OpStatus::Accepted,
                "rejected" => OpStatus::Rejected,
                "discarded" => OpStatus::Discarded,
                _ => OpStatus::Visible,
            };

            Operation {
                id: o.id,
                participant_id: o.participant_id,
                region_id: o.region_id,
                op_type,
                content: o.content,
                reason: o.reason,
                signature: o.signature,
                timestamp: o.timestamp,
                status,
            }
        }).collect();

        Ok(operations)
    }

    pub fn get_files(&self) -> Result<Vec<String>, String> {
        let url = format!("{}/rad/files", self.url);
        let resp = ureq::get(&url)
            .call()
            .map_err(|e| format!("Failed to get files: {}", e))?;

        let files: Vec<FileInfo> = resp.into_json()
            .map_err(|e| format!("Failed to parse files: {}", e))?;

        Ok(files.into_iter().map(|f| f.path).collect())
    }

    pub fn get_file(&self, path: &str) -> Result<FileContent, String> {
        let url = format!("{}/rad/files/{}", self.url, path);
        let resp = ureq::get(&url)
            .call()
            .map_err(|e| format!("Failed to get file: {}", e))?;

        let result: FileContent = resp.into_json()
            .map_err(|e| format!("Failed to parse file content: {}", e))?;

        Ok(result)
    }
}
