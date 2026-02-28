use wws_state::reputation::*;

#[test]
fn test_tier_boundaries() {
    assert_eq!(tier_for_score(-1),    ReputationTier::Suspended);
    assert_eq!(tier_for_score(0),     ReputationTier::Newcomer);
    assert_eq!(tier_for_score(99),    ReputationTier::Newcomer);
    assert_eq!(tier_for_score(100),   ReputationTier::Member);
    assert_eq!(tier_for_score(499),   ReputationTier::Member);
    assert_eq!(tier_for_score(500),   ReputationTier::Trusted);
    assert_eq!(tier_for_score(999),   ReputationTier::Trusted);
    assert_eq!(tier_for_score(1000),  ReputationTier::Established);
    assert_eq!(tier_for_score(4999),  ReputationTier::Established);
    assert_eq!(tier_for_score(5000),  ReputationTier::Veteran);
}

#[test]
fn test_score_decay_no_activity() {
    let score = effective_score(1000, 15, 1000); // 15 days inactive
    assert!(score < 1000, "score should decay after inactivity");
    assert!(score >= 500, "score should not fall below 50% of peak");
}

#[test]
fn test_score_decay_grace_period() {
    let score = effective_score(1000, 1, 1000); // 1 day inactive (< 2 day grace)
    assert_eq!(score, 1000, "no decay within grace period");
}

#[test]
fn test_score_decay_at_boundary() {
    let score = effective_score(1000, 2, 1000); // exactly at grace period boundary
    assert_eq!(score, 1000, "no decay at exactly 2 days");
}

#[test]
fn test_observer_weight_zero_score() {
    assert_eq!(observer_contribution(10, 0), 0);
}

#[test]
fn test_observer_weight_full_score() {
    assert_eq!(observer_contribution(10, 1000), 10);
}

#[test]
fn test_observer_weight_partial() {
    let contrib = observer_contribution(100, 500);
    assert!(contrib > 0 && contrib < 100);
}

#[test]
fn test_injection_blocked_newcomer() {
    assert!(check_injection_permission(50, 1).is_err());
}

#[test]
fn test_injection_allowed_member() {
    assert!(check_injection_permission(100, 1).is_ok());
}

#[test]
fn test_injection_blocked_insufficient_for_complex() {
    assert!(check_injection_permission(100, 6).is_err()); // complexity 6 needs 1000
}
