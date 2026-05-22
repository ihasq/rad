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

pub fn keypair_from_secret(secret_b64: &str) -> KeyPair {
    let secret_bytes = STANDARD.decode(secret_b64).expect("Invalid base64 secret key");
    // 64バイトフォーマット: 先頭32バイトが秘密鍵、後半32バイトが公開鍵
    let public_key = STANDARD.encode(&secret_bytes[32..64]);
    KeyPair {
        public_key,
        secret_key: secret_bytes,
    }
}

pub fn format_public_key(kp: &KeyPair) -> String {
    kp.public_key.clone()
}
