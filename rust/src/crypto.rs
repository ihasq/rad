use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub struct KeyPair {
    pub public_key: String,
    pub secret_key: Vec<u8>,
}

pub fn generate_keypair() -> KeyPair {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    // tweetnacl と互換性を保つため、64バイトフォーマット（秘密鍵32バイト + 公開鍵32バイト）を使用
    let mut secret_key = Vec::new();
    secret_key.extend_from_slice(&signing_key.to_bytes());
    secret_key.extend_from_slice(verifying_key.as_bytes());
    KeyPair {
        public_key: STANDARD.encode(verifying_key.as_bytes()),
        secret_key,
    }
}

pub fn format_keypair(kp: &KeyPair) -> String {
    let secret_b64 = STANDARD.encode(&kp.secret_key);
    format!("public:  {}\nsecret:  {}", kp.public_key, secret_b64)
}
