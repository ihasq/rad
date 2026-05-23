use ed25519_dalek::{VerifyingKey, Verifier, Signature};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crate::sign::canonicalize;

pub fn verify_operation(op_json: &str, public_key_b64: &str) -> bool {
    let canonical = canonicalize(op_json);
    let val: serde_json::Value = serde_json::from_str(op_json).unwrap();
    let sig_b64 = val["signature"].as_str().unwrap_or("");

    let sig_bytes = match STANDARD.decode(sig_b64) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let pk_bytes = match STANDARD.decode(public_key_b64) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let signature = match Signature::from_bytes(&sig_bytes.try_into().unwrap_or([0u8; 64])) {
        s => s,
    };
    let verifying_key = match VerifyingKey::from_bytes(&pk_bytes.try_into().unwrap_or([0u8; 32])) {
        Ok(k) => k,
        Err(_) => return false,
    };

    verifying_key.verify(canonical.as_bytes(), &signature).is_ok()
}
