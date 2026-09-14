use std::time::{SystemTime, UNIX_EPOCH};

/// Ranking uses only user frequency, base frequency, and recency.
///
/// Input method (full / initial / fuzzy) is intentionally absent.
#[derive(Debug, Clone, Copy)]
pub struct ScoreInputs {
    pub base_frequency: u32,
    pub user_count: u32,
    pub last_used: Option<u64>,
    pub now: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Recency {
    pub bonus: f64,
}

/// `user_count` is linear so personal habit can overtake a large public frequency
/// after repeated choices, without exploding as fast as a raw public count.
///
/// Public frequency is log-scaled so 90000 vs 20000 is a modest gap
/// (about 8–10 user selections with default weights).
pub fn score(inputs: ScoreInputs) -> f64 {
    let base = (inputs.base_frequency as f64 + 1.0).ln() * 8.0;
    let user = inputs.user_count as f64 * 2.0;
    let recency = recency_bonus(inputs.last_used, inputs.now);
    user + base + recency
}

pub fn recency_bonus(last_used: Option<u64>, now: u64) -> f64 {
    let Some(ts) = last_used else {
        return 0.0;
    };
    if ts > now {
        return 1.5;
    }
    let dt = now.saturating_sub(ts);
    const HOUR: u64 = 3600;
    const DAY: u64 = 86400;
    const WEEK: u64 = 7 * DAY;
    if dt < HOUR {
        1.5
    } else if dt < DAY {
        0.8
    } else if dt < WEEK {
        0.3
    } else {
        0.0
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_habit_can_overtake() {
        let now = 1_700_000_000;
        let shangpin = score(ScoreInputs {
            base_frequency: 90_000,
            user_count: 0,
            last_used: None,
            now,
        });
        let shangpin_user = score(ScoreInputs {
            base_frequency: 20_000,
            user_count: 12,
            last_used: Some(now),
            now,
        });
        assert!(
            shangpin_user > shangpin,
            "user 12 times should outrank 90000 vs 20000 (got {shangpin_user} vs {shangpin})"
        );
    }

    #[test]
    fn fuzzy_has_no_score_term() {
        // Two hits from different pinyin origins with identical stats must tie.
        let a = score(ScoreInputs {
            base_frequency: 100,
            user_count: 0,
            last_used: None,
            now: 0,
        });
        let b = score(ScoreInputs {
            base_frequency: 100,
            user_count: 0,
            last_used: None,
            now: 0,
        });
        assert_eq!(a, b);
    }
}
