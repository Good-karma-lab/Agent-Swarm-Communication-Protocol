/// Reputation tier for an agent based on their score.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReputationTier {
    Suspended,
    Newcomer,
    Member,
    Trusted,
    Established,
    Veteran,
}

impl std::fmt::Display for ReputationTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Suspended => "suspended",
            Self::Newcomer => "newcomer",
            Self::Member => "member",
            Self::Trusted => "trusted",
            Self::Established => "established",
            Self::Veteran => "veteran",
        };
        write!(f, "{s}")
    }
}

pub fn tier_for_score(score: i64) -> ReputationTier {
    if score < 0 { return ReputationTier::Suspended; }
    match score {
        0..=99        => ReputationTier::Newcomer,
        100..=499     => ReputationTier::Member,
        500..=999     => ReputationTier::Trusted,
        1000..=4999   => ReputationTier::Established,
        _             => ReputationTier::Veteran,
    }
}

/// Compute effective score with time-based decay.
/// - `raw`: current raw score
/// - `days_inactive`: days since last activity
/// - `peak`: lifetime peak score (floor = peak/2)
/// - Decay rate: 0.5% per day after 2-day grace period
pub fn effective_score(raw: i64, days_inactive: u32, peak: i64) -> i64 {
    if days_inactive <= 2 { return raw; }
    let decay_days = (days_inactive - 2) as i32;
    let decayed = (raw as f64 * (1.0_f64 - 0.005_f64).powi(decay_days)) as i64;
    decayed.max(peak / 2)
}

/// Observer-weighted contribution.
/// - Objective events (completed tasks, verified results): full weight
/// - Subjective events (ratings from peers): scaled by observer's own reputation
/// - Observer score 0..=1000 → weight 0.0..=1.0
pub fn observer_contribution(base_points: i64, observer_score: i64) -> i64 {
    let weight = (observer_score as f64 / 1000.0).min(1.0).max(0.0);
    (base_points as f64 * weight) as i64
}

/// Check whether an agent with `caller_score` may inject a task of given `complexity`.
pub fn check_injection_permission(caller_score: i64, complexity: u32) -> Result<(), String> {
    let min_score: i64 = match complexity {
        c if c <= 1 => 100,
        c if c <= 5 => 500,
        _           => 1000,
    };
    if caller_score < min_score {
        Err(format!("insufficient reputation: need {min_score}, have {caller_score}"))
    } else {
        Ok(())
    }
}
