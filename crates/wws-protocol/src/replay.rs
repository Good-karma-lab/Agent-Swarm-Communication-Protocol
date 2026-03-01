use std::collections::HashMap;
use crate::ProtocolError;

pub const REPLAY_WINDOW_SECS: u64 = 600;      // 10-minute nonce window
pub const TIMESTAMP_TOLERANCE_SECS: u64 = 300; // 5 minutes tolerance

/// Rolling time-bucketed nonce replay prevention window.
/// Tracks seen (nonce, timestamp) pairs within the window.
/// Evicts expired entries on each check.
pub struct ReplayWindow {
    /// nonce → insertion_timestamp
    seen: HashMap<String, u64>,
}

impl ReplayWindow {
    pub fn new() -> Self { Self { seen: HashMap::new() } }

    /// Check that timestamp is within tolerance and nonce has not been seen before.
    /// On success, records the nonce. On failure, returns ProtocolError::Crypto.
    pub fn check_and_insert(&mut self, nonce: &str, timestamp: u64) -> Result<(), ProtocolError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Evict expired entries (older than REPLAY_WINDOW_SECS)
        self.seen.retain(|_, ts| now.saturating_sub(*ts) < REPLAY_WINDOW_SECS);

        // Check timestamp tolerance (both past and future)
        let diff = now.abs_diff(timestamp);
        if diff > TIMESTAMP_TOLERANCE_SECS {
            return Err(ProtocolError::Crypto(
                format!("timestamp {diff}s outside {TIMESTAMP_TOLERANCE_SECS}s tolerance")
            ));
        }

        // Check replay
        if self.seen.contains_key(nonce) {
            return Err(ProtocolError::Crypto(
                format!("replay detected for nonce '{nonce}'")
            ));
        }

        self.seen.insert(nonce.to_string(), now);
        Ok(())
    }

    /// Returns the number of tracked nonces (for testing/monitoring).
    pub fn size(&self) -> usize { self.seen.len() }
}

impl Default for ReplayWindow {
    fn default() -> Self { Self::new() }
}
