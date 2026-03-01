//! WWS integration tests for new features.
//! These tests verify the new WWS modules work together end-to-end.

// ─── Identity persistence ────────────────────────────────────────────────────

#[test]
fn test_identity_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");

    // Create keypair on first call
    let k1 = wws_protocol::crypto::load_or_create_keypair(&key_path).unwrap();
    assert!(key_path.exists(), "key file must be created on first call");

    // Load same keypair on second call
    let k2 = wws_protocol::crypto::load_or_create_keypair(&key_path).unwrap();
    assert_eq!(
        k1.verifying_key().as_bytes(),
        k2.verifying_key().as_bytes(),
        "keypair must be stable across restarts"
    );
}

// ─── Mnemonic round-trip ─────────────────────────────────────────────────────

#[test]
fn test_identity_to_mnemonic_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("agent.key");
    let key = wws_protocol::crypto::load_or_create_keypair(&key_path).unwrap();

    let mnemonic = wws_protocol::crypto::keypair_to_mnemonic(&key).unwrap();
    let restored = wws_protocol::crypto::keypair_from_mnemonic(&mnemonic).unwrap();

    assert_eq!(
        key.verifying_key().as_bytes(),
        restored.verifying_key().as_bytes(),
        "mnemonic roundtrip must restore the exact same identity"
    );
}

// ─── Name registration lifecycle ─────────────────────────────────────────────

#[test]
fn test_name_registration_lifecycle() {
    use wws_network::name_registry::*;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let record = NameRecord {
        name: "testuser".into(),
        did: "did:swarm:test123".into(),
        peer_id: "12D3KooWTest".into(),
        registered_at: now,
        expires_at: now + NAME_TTL_SECS,
        pow_nonce: 42,
        signature: vec![],
    };

    assert!(!record.is_expired(), "freshly created record must not be expired");

    let dht_key = NameRecord::dht_key("testuser");
    let dht_key_upper = NameRecord::dht_key("TESTUSER"); // case-insensitive
    assert_eq!(dht_key, dht_key_upper, "DHT key must be case-insensitive");

    assert_eq!(pow_difficulty_for_name("testuser"), 12, "7-12 char names need difficulty 12");
}

// ─── Reputation CRDT merge ───────────────────────────────────────────────────

#[test]
fn test_reputation_crdt_merge() {
    use wws_state::pn_counter::PnCounter;
    use wws_state::reputation::{tier_for_score, ReputationTier};

    let mut counter_a = PnCounter::new("agent_a");
    counter_a.increment(200);

    let mut counter_b = PnCounter::new("agent_b");
    counter_b.increment(100);

    counter_a.merge(&counter_b);

    assert_eq!(counter_a.value(), 300, "merged counter must sum both contributions");
    assert_eq!(
        tier_for_score(counter_a.value()),
        ReputationTier::Member,
        "score 300 maps to Member tier"
    );
}

// ─── Key rotation and guardian recovery ──────────────────────────────────────

#[test]
fn test_key_rotation_and_guardian_recovery() {
    use wws_protocol::key_rotation::*;
    use wws_protocol::crypto::generate_keypair;

    // Test rotation announcement
    let old_key = generate_keypair();
    let new_key = generate_keypair();
    let ts = current_timestamp_secs();
    let ann = build_rotation_announcement(&old_key, &new_key, ts);
    assert!(
        verify_rotation_announcement(&ann, ts).is_ok(),
        "freshly built rotation announcement must verify successfully"
    );

    // Test guardian threshold
    let votes = vec!["guardian1".to_string(), "guardian2".to_string()];
    let guardians = vec!["guardian1", "guardian2", "guardian3"];
    assert!(
        verify_guardian_threshold(&votes, 2, &guardians).is_ok(),
        "2-of-3 guardian threshold must be satisfied by 2 valid votes"
    );
}

// ─── Replay window integration ───────────────────────────────────────────────

#[test]
fn test_replay_window_integration() {
    use wws_protocol::replay::ReplayWindow;

    let mut window = ReplayWindow::new();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // First use: accepted
    assert!(
        window.check_and_insert("unique-nonce-001", ts).is_ok(),
        "fresh nonce must be accepted"
    );

    // Second use: rejected (replay attack)
    assert!(
        window.check_and_insert("unique-nonce-001", ts).is_err(),
        "replayed nonce must be rejected"
    );

    // Different nonce: accepted
    assert!(
        window.check_and_insert("unique-nonce-002", ts).is_ok(),
        "different nonce must be accepted"
    );
}
