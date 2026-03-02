pub mod platform;
pub mod publisher;

use rand::RngExt;
use sha2::{Digest, Sha256};

/// Hash an API key with SHA-256 for storage/lookup.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a random API key with the given prefix (e.g. "oat_pk", "oat_pub").
pub fn generate_raw_key(prefix: &str) -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 24] = rng.random();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{prefix}_{hex}")
}
