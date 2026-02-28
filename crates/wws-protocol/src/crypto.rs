use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::ProtocolError;

/// Generate a new Ed25519 keypair.
pub fn generate_keypair() -> SigningKey {
    let mut rng = rand::thread_rng();
    SigningKey::generate(&mut rng)
}

/// Derive the agent ID (DID) from a public key.
/// Format: did:swarm:<hex(sha256(pub_key))>
pub fn derive_agent_id(verifying_key: &VerifyingKey) -> String {
    let hash = Sha256::digest(verifying_key.as_bytes());
    format!("did:swarm:{}", hex_encode(&hash))
}

/// Sign a message payload with the signing key.
pub fn sign_message(signing_key: &SigningKey, payload: &[u8]) -> Signature {
    signing_key.sign(payload)
}

/// Verify a message signature against the verifying key.
pub fn verify_signature(
    verifying_key: &VerifyingKey,
    payload: &[u8],
    signature: &Signature,
) -> Result<(), ProtocolError> {
    verifying_key
        .verify(payload, signature)
        .map_err(|e| ProtocolError::InvalidSignature(e.to_string()))
}

/// Compute SHA-256 hash of data.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let hash = Sha256::digest(data);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    result
}

/// Compute a Content ID (CID) for data — SHA-256 hex string.
pub fn compute_cid(data: &[u8]) -> String {
    hex_encode(&sha256(data))
}

/// Simple Proof of Work: find a nonce such that SHA-256(data || nonce)
/// has at least `difficulty` leading zero bits.
pub fn proof_of_work(data: &[u8], difficulty: u32) -> (u64, [u8; 32]) {
    let mut nonce: u64 = 0;
    loop {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(nonce.to_le_bytes());
        let hash = hasher.finalize();
        let hash_arr: [u8; 32] = hash.into();
        if leading_zeros(&hash_arr) >= difficulty {
            return (nonce, hash_arr);
        }
        nonce += 1;
    }
}

/// Verify a Proof of Work.
pub fn verify_pow(data: &[u8], nonce: u64, difficulty: u32) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.update(nonce.to_le_bytes());
    let hash = hasher.finalize();
    let hash_arr: [u8; 32] = hash.into();
    leading_zeros(&hash_arr) >= difficulty
}

/// Count leading zero bits in a byte array.
fn leading_zeros(hash: &[u8; 32]) -> u32 {
    let mut count = 0u32;
    for byte in hash.iter() {
        if *byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros();
            break;
        }
    }
    count
}

/// Hex-encode bytes.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

use std::path::Path;

/// Load an Ed25519 keypair from a file, or create a new one if the file doesn't exist.
/// The file stores the raw 32-byte Ed25519 seed with mode 0600.
pub fn load_or_create_keypair(path: &Path) -> Result<SigningKey, crate::ProtocolError> {
    if path.exists() {
        let seed_bytes = std::fs::read(path)
            .map_err(|e| crate::ProtocolError::Crypto(format!("read key file: {e}")))?;
        if seed_bytes.len() != 32 {
            return Err(crate::ProtocolError::Crypto(
                format!("key file is {} bytes, expected 32", seed_bytes.len())
            ));
        }
        let seed: [u8; 32] = seed_bytes.try_into().unwrap();
        Ok(SigningKey::from_bytes(&seed))
    } else {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::ProtocolError::Crypto(format!("create dir: {e}")))?;
        }
        let mut rng = rand::thread_rng();
        let key = SigningKey::generate(&mut rng);
        let seed = key.to_bytes();
        std::fs::write(path, seed)
            .map_err(|e| crate::ProtocolError::Crypto(format!("write key file: {e}")))?;
        // Set file permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| crate::ProtocolError::Crypto(format!("set permissions: {e}")))?;
        }
        Ok(key)
    }
}

/// Derive a 24-word BIP-39 mnemonic from an Ed25519 signing key.
/// The mnemonic encodes the 32-byte seed as 256 bits → 24 words.
pub fn keypair_to_mnemonic(key: &SigningKey) -> Result<String, crate::ProtocolError> {
    use bip39::Mnemonic;
    let seed = key.to_bytes();
    let mnemonic = Mnemonic::from_entropy(&seed)
        .map_err(|e| crate::ProtocolError::Crypto(format!("mnemonic generation: {e}")))?;
    Ok(mnemonic.to_string())
}

/// Restore an Ed25519 signing key from a 24-word BIP-39 mnemonic.
pub fn keypair_from_mnemonic(phrase: &str) -> Result<SigningKey, crate::ProtocolError> {
    use bip39::Mnemonic;
    use zeroize::Zeroize;
    let mnemonic = Mnemonic::parse(phrase)
        .map_err(|e| crate::ProtocolError::Crypto(format!("invalid mnemonic: {e}")))?;
    let mut entropy = mnemonic.to_entropy();
    if entropy.len() < 32 {
        return Err(crate::ProtocolError::Crypto("entropy too short".into()));
    }
    let seed: [u8; 32] = entropy[..32].try_into().unwrap();
    entropy.zeroize();
    Ok(SigningKey::from_bytes(&seed))
}

/// Derive a recovery keypair from the primary signing key.
/// Uses SHA-256 hash of (primary_seed || "wws-recovery") as the recovery seed.
pub fn derive_recovery_key(primary: &SigningKey) -> SigningKey {
    let seed = primary.to_bytes();
    let mut hasher = Sha256::new();
    hasher.update(&seed);
    hasher.update(b"wws-recovery");
    let recovery_seed: [u8; 32] = hasher.finalize().into();
    SigningKey::from_bytes(&recovery_seed)
}

/// Hex-decode a string into bytes.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, ProtocolError> {
    if s.len() % 2 != 0 {
        return Err(ProtocolError::Crypto("odd-length hex string".into()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| ProtocolError::Crypto(format!("invalid hex: {}", e)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_keypair_and_agent_id() {
        let signing_key = generate_keypair();
        let verifying_key = signing_key.verifying_key();
        let agent_id = derive_agent_id(&verifying_key);
        assert!(agent_id.starts_with("did:swarm:"));
        assert_eq!(agent_id.len(), "did:swarm:".len() + 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_sign_and_verify() {
        let signing_key = generate_keypair();
        let verifying_key = signing_key.verifying_key();
        let message = b"hello swarm";
        let sig = sign_message(&signing_key, message);
        assert!(verify_signature(&verifying_key, message, &sig).is_ok());
    }

    #[test]
    fn test_sign_verify_wrong_message() {
        let signing_key = generate_keypair();
        let verifying_key = signing_key.verifying_key();
        let sig = sign_message(&signing_key, b"correct");
        assert!(verify_signature(&verifying_key, b"wrong", &sig).is_err());
    }

    #[test]
    fn test_cid() {
        let cid1 = compute_cid(b"hello");
        let cid2 = compute_cid(b"hello");
        let cid3 = compute_cid(b"world");
        assert_eq!(cid1, cid2);
        assert_ne!(cid1, cid3);
    }

    #[test]
    fn test_pow() {
        let data = b"test data";
        let difficulty = 8; // low difficulty for test speed
        let (nonce, _hash) = proof_of_work(data, difficulty);
        assert!(verify_pow(data, nonce, difficulty));
        assert!(!verify_pow(data, nonce + 1, difficulty)); // wrong nonce
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = vec![0xde, 0xad, 0xbe, 0xef];
        let encoded = hex_encode(&original);
        assert_eq!(encoded, "deadbeef");
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }
}
