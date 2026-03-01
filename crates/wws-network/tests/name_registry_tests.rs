use wws_network::name_registry::*;

#[test]
fn test_pow_difficulty_by_name_length() {
    assert_eq!(pow_difficulty_for_name("ab"),      20); // 1-3 chars
    assert_eq!(pow_difficulty_for_name("abc"),     20);
    assert_eq!(pow_difficulty_for_name("abcd"),    16); // 4-6 chars
    assert_eq!(pow_difficulty_for_name("abcde"),   16);
    assert_eq!(pow_difficulty_for_name("abcdefg"), 12); // 7-12 chars
    assert_eq!(pow_difficulty_for_name(&"a".repeat(13)), 8); // 13+
}

#[test]
fn test_levenshtein_distance() {
    assert_eq!(levenshtein("alice", "alice_"), 1);
    assert_eq!(levenshtein("alice", "alicee"), 1);
    assert_eq!(levenshtein("alice", "bob"),    5);
    assert_eq!(levenshtein("alice", "alice"),  0);
    assert_eq!(levenshtein("",      "abc"),    3);
    assert_eq!(levenshtein("abc",   ""),       3);
}

#[test]
fn test_name_record_expired() {
    let record = NameRecord {
        name: "test".into(),
        did: "did:swarm:abc".into(),
        peer_id: "12D3".into(),
        registered_at: 0,
        expires_at: 1, // already expired (Unix epoch + 1 second)
        pow_nonce: 0,
        signature: vec![],
    };
    assert!(record.is_expired());
}

#[test]
fn test_name_record_not_expired() {
    let far_future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 86400;
    let record = NameRecord {
        name: "test".into(),
        did: "did:swarm:abc".into(),
        peer_id: "12D3".into(),
        registered_at: 0,
        expires_at: far_future,
        pow_nonce: 0,
        signature: vec![],
    };
    assert!(!record.is_expired());
}

#[test]
fn test_typosquat_detection() {
    let existing = vec!["alice", "bob"];
    // "alicee" is 1 edit away from "alice" → boost
    assert!(typosquat_difficulty_boost("alicee", &existing) > 0);
    // "charlie" is far from both → no boost
    assert_eq!(typosquat_difficulty_boost("charlie", &existing), 0);
}

#[test]
fn test_dht_key_is_deterministic() {
    let k1 = NameRecord::dht_key("alice");
    let k2 = NameRecord::dht_key("alice");
    let k3 = NameRecord::dht_key("ALICE"); // case-insensitive
    assert_eq!(k1, k2);
    assert_eq!(k1, k3);
}
