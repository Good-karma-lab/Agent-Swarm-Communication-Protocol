//! wws:// name registry — decentralized, first-claim, TTL 24h.
//!
//! DHT key: /wws/names/<sha256(lowercase(name))>

use serde::{Deserialize, Serialize};

pub const NAME_TTL_SECS: u64 = 86_400;           // 24 hours
pub const NAME_GRACE_SECS: u64 = 21_600;         // 6 hours grace after expiry
pub const NAME_RENEWAL_WINDOW_SECS: u64 = 3_600; // renew 1h before expiry
pub const MIN_REPUTATION_SHORT_NAME: i64 = 1_000; // 1-3 char names
pub const MIN_REPUTATION_MEDIUM_NAME: i64 = 100;  // 4-6 char names

/// A signed name registration record stored in the Kademlia DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NameRecord {
    pub name: String,
    pub did: String,
    pub peer_id: String,
    pub registered_at: u64,
    pub expires_at: u64,
    pub pow_nonce: u64,
    /// Ed25519 signature over canonical JSON of all other fields
    pub signature: Vec<u8>,
}

impl NameRecord {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.expires_at
    }

    pub fn in_grace_period(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.expires_at && now <= self.expires_at + NAME_GRACE_SECS
    }

    /// DHT storage key for this name record.
    pub fn dht_key(name: &str) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(name.to_lowercase().as_bytes());
        format!("/wws/names/{}", hex::encode(hash)).into_bytes()
    }
}

/// PoW difficulty required for a name registration based on name length.
pub fn pow_difficulty_for_name(name: &str) -> u32 {
    match name.len() {
        1..=3  => 20,
        4..=6  => 16,
        7..=12 => 12,
        _      => 8,
    }
}

/// Compute Levenshtein edit distance between two strings.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m { dp[i][0] = i; }
    for j in 0..=n { dp[0][j] = j; }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i-1] == b[j-1] {
                dp[i-1][j-1]
            } else {
                1 + dp[i-1][j-1].min(dp[i-1][j]).min(dp[i][j-1])
            };
        }
    }
    dp[m][n]
}

/// Extra PoW difficulty added when name is within 2 edits of an existing registered name.
pub fn typosquat_difficulty_boost(new_name: &str, existing_names: &[&str]) -> u32 {
    for existing in existing_names {
        if levenshtein(new_name, existing) <= 2 {
            return 4;
        }
    }
    0
}
