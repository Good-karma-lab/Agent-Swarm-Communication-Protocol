use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use crate::ProtocolError;

pub const ROTATION_TIMESTAMP_TOLERANCE_SECS: u64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationAnnouncement {
    pub agent_did: String,
    pub old_pubkey_hex: String,
    pub new_pubkey_hex: String,
    pub rotation_timestamp: u64,
    pub sig_old: Vec<u8>, // old key signs (new_pubkey_bytes || timestamp_le_bytes)
    pub sig_new: Vec<u8>, // new key signs (old_pubkey_bytes || timestamp_le_bytes)
}

pub fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn build_rotation_announcement(old_key: &SigningKey, new_key: &SigningKey, ts: u64) -> RotationAnnouncement {
    let old_pub_bytes = old_key.verifying_key().to_bytes();
    let new_pub_bytes = new_key.verifying_key().to_bytes();
    let ts_bytes = ts.to_le_bytes();

    // old key signs: new_pubkey || timestamp
    let payload_old = [new_pub_bytes.as_slice(), &ts_bytes].concat();
    // new key signs: old_pubkey || timestamp
    let payload_new = [old_pub_bytes.as_slice(), &ts_bytes].concat();

    let sig_old = crate::crypto::sign_message(old_key, &payload_old).to_bytes().to_vec();
    let sig_new = crate::crypto::sign_message(new_key, &payload_new).to_bytes().to_vec();

    RotationAnnouncement {
        agent_did: crate::crypto::derive_agent_id(&old_key.verifying_key()),
        old_pubkey_hex: hex::encode(old_pub_bytes),
        new_pubkey_hex: hex::encode(new_pub_bytes),
        rotation_timestamp: ts,
        sig_old,
        sig_new,
    }
}

pub fn verify_rotation_announcement(ann: &RotationAnnouncement, now: u64) -> Result<(), ProtocolError> {
    // Check timestamp within tolerance
    let diff = now.abs_diff(ann.rotation_timestamp);
    if diff > ROTATION_TIMESTAMP_TOLERANCE_SECS {
        return Err(ProtocolError::Crypto(
            format!("stale rotation timestamp: {diff}s off (tolerance: {ROTATION_TIMESTAMP_TOLERANCE_SECS}s)")
        ));
    }

    let ts_bytes = ann.rotation_timestamp.to_le_bytes();

    // Decode pubkeys
    let old_pub_bytes = hex::decode(&ann.old_pubkey_hex)
        .map_err(|e| ProtocolError::Crypto(format!("invalid old pubkey hex: {e}")))?;
    let new_pub_bytes = hex::decode(&ann.new_pubkey_hex)
        .map_err(|e| ProtocolError::Crypto(format!("invalid new pubkey hex: {e}")))?;

    let old_pub_arr: [u8; 32] = old_pub_bytes.try_into()
        .map_err(|_| ProtocolError::Crypto("old pubkey wrong length".into()))?;
    let new_pub_arr: [u8; 32] = new_pub_bytes.try_into()
        .map_err(|_| ProtocolError::Crypto("new pubkey wrong length".into()))?;

    let old_vk = VerifyingKey::from_bytes(&old_pub_arr)
        .map_err(|e| ProtocolError::Crypto(format!("invalid old pubkey: {e}")))?;
    let new_vk = VerifyingKey::from_bytes(&new_pub_arr)
        .map_err(|e| ProtocolError::Crypto(format!("invalid new pubkey: {e}")))?;

    // Verify: old key signed (new_pubkey || timestamp)
    let payload_old = [new_pub_arr.as_slice(), &ts_bytes].concat();
    let sig_old_arr: [u8; 64] = ann.sig_old.as_slice().try_into()
        .map_err(|_| ProtocolError::Crypto("sig_old wrong length".into()))?;
    let sig_old = Signature::from_bytes(&sig_old_arr);
    crate::crypto::verify_signature(&old_vk, &payload_old, &sig_old)?;

    // Verify: new key signed (old_pubkey || timestamp)
    let payload_new = [old_pub_arr.as_slice(), &ts_bytes].concat();
    let sig_new_arr: [u8; 64] = ann.sig_new.as_slice().try_into()
        .map_err(|_| ProtocolError::Crypto("sig_new wrong length".into()))?;
    let sig_new = Signature::from_bytes(&sig_new_arr);
    crate::crypto::verify_signature(&new_vk, &payload_new, &sig_new)?;

    Ok(())
}

/// Emergency revocation using the pre-committed recovery key.
/// The recovery key was derived during identity creation and its hash is known to peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyRevocation {
    pub agent_did: String,
    pub recovery_pubkey_hex: String, // reveals recovery pubkey for first time
    pub new_primary_pubkey_hex: String,
    pub revocation_timestamp: u64,
    pub sig_recovery: Vec<u8>, // recovery key signs (new_primary_pubkey || timestamp)
}

/// Guardian designation: agent designates up to 5 trusted guardians for M-of-N recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianDesignation {
    pub agent_did: String,
    pub guardian_dids: Vec<String>, // up to 5 guardian DIDs
    pub threshold: u32,
    pub timestamp: u64,
    pub sig: Vec<u8>, // signed by agent primary key
}

/// A guardian's vote to recover an agent's identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianRecoveryVote {
    pub target_did: String,
    pub new_pubkey_hex: String,
    pub timestamp: u64,
    pub guardian_did: String,
    pub sig_guardian: Vec<u8>,
}

/// Verify that M-of-N guardian threshold is met.
/// `votes`: DIDs of guardians who have submitted valid votes
/// `threshold`: required number of valid votes
/// `authorized_guardians`: the registered guardian DIDs for this agent
pub fn verify_guardian_threshold(
    votes: &[String],
    threshold: u32,
    authorized_guardians: &[&str],
) -> Result<(), ProtocolError> {
    let valid_count = votes.iter()
        .filter(|v| authorized_guardians.contains(&v.as_str()))
        .count();

    if valid_count >= threshold as usize {
        Ok(())
    } else {
        Err(ProtocolError::Crypto(format!(
            "insufficient guardian votes: {valid_count}/{threshold}"
        )))
    }
}
