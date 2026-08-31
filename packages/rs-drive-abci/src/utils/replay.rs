//! Telling a node that is replaying history from one that is following the tip.
//!
//! Some per-block work only earns its cost at the tip. Creating a GroveDB
//! checkpoint every ten minutes of chain time is useful on a running node and
//! pure waste while catching up, where ten minutes of chain time is a handful of
//! blocks and every checkpoint but the last few is deleted within the second.

/// A block older than this is not one the network just produced. Mainnet aims at
/// about 2.5 minutes a block, so this leaves several blocks of slack for a node
/// that is merely a little behind.
const HISTORICAL_BLOCK_AGE_MS: u64 = 10 * 60 * 1000;

/// True when a block with this timestamp is old enough that the node producing
/// it is clearly replaying history rather than following the tip.
pub fn is_historical_block(block_time_ms: u64) -> bool {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since_epoch| since_epoch.as_millis() as u64)
        .unwrap_or(0);
    now_ms.saturating_sub(block_time_ms) > HISTORICAL_BLOCK_AGE_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before the unix epoch")
            .as_millis() as u64
    }

    #[test]
    fn a_block_from_a_year_ago_is_historical() {
        assert!(is_historical_block(
            now_ms() - 365 * 24 * 60 * 60 * 1000
        ));
    }

    #[test]
    fn a_block_from_a_minute_ago_is_not_historical() {
        assert!(!is_historical_block(now_ms() - 60 * 1000));
    }

    #[test]
    fn a_block_at_the_threshold_is_not_yet_historical() {
        assert!(!is_historical_block(now_ms() - HISTORICAL_BLOCK_AGE_MS));
    }

    #[test]
    fn a_block_timestamped_in_the_future_is_not_historical() {
        assert!(!is_historical_block(now_ms() + 60 * 1000));
    }
}
