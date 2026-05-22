use serde_json::Value;
use ed25519_dalek::{SigningKey, Signer};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// signature フィールドを除外し、キー昇順ソートのコンパクト JSON を生成
pub fn canonicalize(op_json: &str) -> String {
    let mut val: Value = serde_json::from_str(op_json).unwrap();
    if let Value::Object(ref mut map) = val {
        map.remove("signature");
        map.remove("status");
        // reason が存在しなければ null を挿入
        if !map.contains_key("reason") {
            map.insert("reason".to_string(), Value::Null);
        }
    }
    // serde_json は BTreeMap でキー昇順を保証
    serde_json::to_string(&val).unwrap()
}

pub fn sign_operation(op_json: &str, secret_key_b64: &str) -> String {
    let canonical = canonicalize(op_json);
    let sk_bytes = STANDARD.decode(secret_key_b64).unwrap();
    // tweetnacl フォーマット（64バイト）に対応：最初の32バイトが秘密鍵
    let sk_32 = if sk_bytes.len() == 64 {
        &sk_bytes[0..32]
    } else {
        &sk_bytes[..]
    };
    let signing_key = SigningKey::from_bytes(&sk_32.try_into().unwrap());
    let sig = signing_key.sign(canonical.as_bytes());
    STANDARD.encode(sig.to_bytes())
}

pub fn inject_signature(op_json: &str, signature: &str) -> String {
    let mut val: Value = serde_json::from_str(op_json).unwrap();
    val["signature"] = Value::String(signature.to_string());
    serde_json::to_string(&val).unwrap()
}
