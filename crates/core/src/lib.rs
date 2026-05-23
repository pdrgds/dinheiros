pub mod api;
pub mod calendar;
pub mod db;
pub mod export;
pub mod maintenance;
pub mod parsers;
pub mod portfolio;
pub mod reconcile;
pub mod seed;
pub mod types;

pub use types::*;

/// SHA256 hash a string and return hex-encoded result.
pub fn hash_string(input: &str) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
