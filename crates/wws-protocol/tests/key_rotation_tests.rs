use wws_protocol::key_rotation::*;
use wws_protocol::crypto::generate_keypair;

#[test]
fn test_rotation_announcement_valid() {
    let old_key = generate_keypair();
    let new_key = generate_keypair();
    let ts = current_timestamp_secs();
    let announcement = build_rotation_announcement(&old_key, &new_key, ts);
    assert!(verify_rotation_announcement(&announcement, ts).is_ok());
}

#[test]
fn test_rotation_announcement_stale_timestamp() {
    let old_key = generate_keypair();
    let new_key = generate_keypair();
    let stale_ts = current_timestamp_secs().saturating_sub(400); // > 5 min ago
    let announcement = build_rotation_announcement(&old_key, &new_key, stale_ts);
    let now = current_timestamp_secs();
    assert!(verify_rotation_announcement(&announcement, now).is_err(),
        "stale timestamp should be rejected");
}

#[test]
fn test_rotation_announcement_different_keys() {
    let old_key = generate_keypair();
    let new_key = generate_keypair();
    let ts = current_timestamp_secs();
    let ann = build_rotation_announcement(&old_key, &new_key, ts);
    assert_ne!(ann.old_pubkey_hex, ann.new_pubkey_hex);
}

#[test]
fn test_guardian_recovery_threshold_not_met() {
    // With threshold=2 and only 1 guardian, recovery should fail
    let votes: Vec<String> = vec!["guardian1".to_string()];
    let guardians = vec!["guardian1", "guardian2", "guardian3"];
    assert!(verify_guardian_threshold(&votes, 2, &guardians).is_err());
}

#[test]
fn test_guardian_recovery_threshold_met() {
    let votes: Vec<String> = vec!["guardian1".to_string(), "guardian2".to_string()];
    let guardians = vec!["guardian1", "guardian2", "guardian3"];
    assert!(verify_guardian_threshold(&votes, 2, &guardians).is_ok());
}

#[test]
fn test_guardian_recovery_invalid_guardian() {
    // A vote from someone not in the guardian list should not count
    let votes: Vec<String> = vec!["guardian1".to_string(), "impostor".to_string()];
    let guardians = vec!["guardian1", "guardian2", "guardian3"];
    assert!(verify_guardian_threshold(&votes, 2, &guardians).is_err());
}
