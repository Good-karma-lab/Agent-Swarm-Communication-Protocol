use wws_protocol::replay::ReplayWindow;

fn current_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

#[test]
fn test_fresh_nonce_accepted() {
    let mut w = ReplayWindow::new();
    let ts = current_ts();
    assert!(w.check_and_insert("nonce-abc", ts).is_ok());
}

#[test]
fn test_replay_rejected() {
    let mut w = ReplayWindow::new();
    let ts = current_ts();
    w.check_and_insert("nonce-abc", ts).unwrap();
    assert!(w.check_and_insert("nonce-abc", ts).is_err(), "replay should be rejected");
}

#[test]
fn test_stale_timestamp() {
    let mut w = ReplayWindow::new();
    let stale = current_ts().saturating_sub(400); // 6+ minutes ago
    assert!(w.check_and_insert("nonce-abc", stale).is_err(), "stale timestamp should be rejected");
}

#[test]
fn test_future_timestamp_rejected() {
    let mut w = ReplayWindow::new();
    let future = current_ts() + 400; // 6+ minutes in future
    assert!(w.check_and_insert("nonce-abc", future).is_err(), "future timestamp should be rejected");
}

#[test]
fn test_different_nonces_accepted() {
    let mut w = ReplayWindow::new();
    let ts = current_ts();
    assert!(w.check_and_insert("nonce-1", ts).is_ok());
    assert!(w.check_and_insert("nonce-2", ts).is_ok());
    assert!(w.check_and_insert("nonce-3", ts).is_ok());
}
