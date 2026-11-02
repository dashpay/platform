mod v0;
mod v1;

use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl RewardDistributionType {
    /// Determines the maximum cycle moment allowed based on the last paid moment,
    /// the current cycle moment, and the maximum allowed token redemption cycles.
    ///
    /// This function calculates a capped distribution moment (`RewardDistributionMoment`) by limiting
    /// the range between the `start_moment` (the last paid moment, or the distribution start) and
    /// the `current_cycle_moment` to the maximum allowed number of redemption cycles: near
    /// unlimited for a fixed-amount function, `max_non_fixed_amount_cycles` otherwise. For an
    /// epoch-based distribution the cap is also held below the current cycle moment, whose
    /// cycle is still running and cannot be rewarded yet.
    ///
    /// The arithmetic is selected by
    /// `platform_version.dpp.token_versions.reward_distribution_max_cycle_moment_version`: version 0
    /// wraps for epoch-based distributions with an interval above one (see `v0`), version 1 does not.
    ///
    /// # Arguments
    /// - `start_moment`: The last moment at which tokens were claimed, or the distribution start.
    /// - `current_cycle_moment`: The current cycle moment as of the current block.
    /// - `max_non_fixed_amount_cycles`: The maximum number of redemption cycles permitted per claim
    ///   for functions other than a fixed amount.
    /// - `platform_version`: Selects the arithmetic version.
    ///
    /// # Returns
    /// - `RewardDistributionMoment`: The maximum allowed cycle moment capped by the cycle limit.
    pub fn max_cycle_moment(
        &self,
        start_moment: RewardDistributionMoment,
        current_cycle_moment: RewardDistributionMoment,
        max_non_fixed_amount_cycles: u32,
        platform_version: &PlatformVersion,
    ) -> Result<RewardDistributionMoment, ProtocolError> {
        match platform_version
            .dpp
            .token_versions
            .reward_distribution_max_cycle_moment_version
        {
            0 => self.max_cycle_moment_v0(
                start_moment,
                current_cycle_moment,
                max_non_fixed_amount_cycles,
            ),
            1 => self.max_cycle_moment_v1(
                start_moment,
                current_cycle_moment,
                max_non_fixed_amount_cycles,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "RewardDistributionType::max_cycle_moment".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use RewardDistributionMoment::{BlockBasedMoment, EpochBasedMoment, TimeBasedMoment};

    /// The redemption cycle cap for functions other than a fixed amount, as every
    /// `SystemLimits` table sets it.
    const MAX_NON_FIXED_CYCLES: u32 = 128;

    fn fixed_epoch(interval: u16) -> RewardDistributionType {
        RewardDistributionType::EpochBasedDistribution {
            interval,
            function: DistributionFunction::FixedAmount { amount: 5 },
        }
    }

    fn random_epoch(interval: u16) -> RewardDistributionType {
        RewardDistributionType::EpochBasedDistribution {
            interval,
            function: DistributionFunction::Random { min: 1, max: 10 },
        }
    }

    fn fixed_block(interval: u64) -> RewardDistributionType {
        RewardDistributionType::BlockBasedDistribution {
            interval,
            function: DistributionFunction::FixedAmount { amount: 5 },
        }
    }

    fn random_block(interval: u64) -> RewardDistributionType {
        RewardDistributionType::BlockBasedDistribution {
            interval,
            function: DistributionFunction::Random { min: 1, max: 10 },
        }
    }

    fn fixed_time(interval: u64) -> RewardDistributionType {
        RewardDistributionType::TimeBasedDistribution {
            interval,
            function: DistributionFunction::FixedAmount { amount: 5 },
        }
    }

    fn latest() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    /// The last protocol version whose cycle cap wrapped.
    fn v13() -> &'static PlatformVersion {
        PlatformVersion::get(13).expect("expected protocol version 13")
    }

    /// Each row is one claim: the distribution, the start moment, the current cycle moment,
    /// the non-fixed cycle limit, the cap expected from the latest protocol version, and the
    /// cap protocol version 13 shipped with (release builds wrap, and `v0` now pins that wrap
    /// explicitly). The latest version caps at the last completed cycle moment; version 13
    /// capped at the current cycle moment less one, which for an interval of one is the same
    /// epoch.
    #[test]
    fn should_cap_epoch_rewards_without_wrapping_from_protocol_version_14() {
        let cases: [(RewardDistributionType, u16, u16, u32, u16, u16); 15] = [
            // The reported shape: interval 2 from epoch 6, claimed at cycle moment 10. A fixed
            // amount allows 32_767 cycles, 6 + 2 * 32_767 = 65_540 overflows u16 and v0 wraps
            // to 4, below the start, so v13 pays nothing. v14 caps at the last completed cycle,
            // the one that began at epoch 8.
            (fixed_epoch(2), 6, 10, MAX_NON_FIXED_CYCLES, 8, 4),
            // Interval 3: 3 * 32_767 already saturates the u16 product, so v0 wraps any nonzero
            // start to one below itself.
            (fixed_epoch(3), 3, 9, MAX_NON_FIXED_CYCLES, 6, 2),
            // A large interval: the u64 reach is 983_040_000, capped at the completed cycle
            // that began at epoch 30_000, the start itself. v0 wrapped to 29_999.
            (
                fixed_epoch(30_000),
                30_000,
                60_000,
                MAX_NON_FIXED_CYCLES,
                30_000,
                29_999,
            ),
            // The widest interval: no cycle completes before epoch 65_535, so nothing is
            // rewardable yet, without any arithmetic fault.
            (
                fixed_epoch(u16::MAX),
                0,
                u16::MAX,
                MAX_NON_FIXED_CYCLES,
                0,
                u16::MAX - 1,
            ),
            // Interval 1 from epoch 0 never overflowed; both versions agree.
            (fixed_epoch(1), 0, 3, MAX_NON_FIXED_CYCLES, 2, 2),
            // Epoch 0 has no completed epoch before it.
            (fixed_epoch(1), 0, 0, MAX_NON_FIXED_CYCLES, 0, 0),
            (fixed_epoch(1), 0, 1, MAX_NON_FIXED_CYCLES, 0, 0),
            // Interval 1 from a start past 32_768: 40_000 + 32_767 wraps to 7_231 under v0.
            (
                fixed_epoch(1),
                40_000,
                50_000,
                MAX_NON_FIXED_CYCLES,
                49_999,
                7_231,
            ),
            // Top of the epoch range: 65_534 + 32_767 wraps to 32_765 under v0.
            (
                fixed_epoch(1),
                u16::MAX - 1,
                u16::MAX,
                MAX_NON_FIXED_CYCLES,
                u16::MAX - 1,
                32_765,
            ),
            // Everything at the maximum: the cap lands below the start and the claim later
            // evaluates to no rewards, in both versions without any arithmetic fault.
            (
                fixed_epoch(u16::MAX),
                u16::MAX,
                u16::MAX,
                MAX_NON_FIXED_CYCLES,
                0,
                u16::MAX - 1,
            ),
            // A function other than a fixed amount honours the small cycle limit instead.
            (random_epoch(2), 6, 100, 3, 12, 12),
            // The cycle limit reaches past the current cycle: v14 caps on the boundary at 98,
            // v13 on the epoch before the cycle moment, 99.
            (random_epoch(2), 6, 100, MAX_NON_FIXED_CYCLES, 98, 99),
            // The widest non-fixed reach, 65_535 * u32::MAX + 1, still fits u64; no cycle has
            // completed yet. v0 truncated the cycle limit to u16 and then wrapped to 0.
            (random_epoch(u16::MAX), 1, 100, u32::MAX, 0, 0),
            // Zero cycles allowed: the cap is the start itself.
            (random_epoch(2), 6, 10, 0, 6, 6),
            // A start left off a boundary (a last paid moment persisted by v0) is still capped
            // on the boundary from v14 on; v0 wrapped 9 + 65_534 to 7.
            (fixed_epoch(2), 9, 14, MAX_NON_FIXED_CYCLES, 12, 7),
        ];

        for (distribution, start, current, max_non_fixed_cycles, expected_v14, expected_v13) in
            cases
        {
            let scenario = format!("{distribution} from {start} at {current}");
            assert_eq!(
                distribution
                    .max_cycle_moment(
                        EpochBasedMoment(start),
                        EpochBasedMoment(current),
                        max_non_fixed_cycles,
                        latest(),
                    )
                    .unwrap_or_else(|e| panic!("latest failed for {scenario}: {e}")),
                EpochBasedMoment(expected_v14),
                "latest: {scenario}"
            );
            assert_eq!(
                distribution
                    .max_cycle_moment(
                        EpochBasedMoment(start),
                        EpochBasedMoment(current),
                        max_non_fixed_cycles,
                        v13(),
                    )
                    .unwrap_or_else(|e| panic!("v13 failed for {scenario}: {e}")),
                EpochBasedMoment(expected_v13),
                "v13: {scenario}"
            );
        }
    }

    #[test]
    fn should_cap_block_and_time_rewards_without_wrapping_from_protocol_version_14() {
        let cases: [(
            RewardDistributionType,
            RewardDistributionMoment,
            RewardDistributionMoment,
            u32,
            RewardDistributionMoment,
            RewardDistributionMoment,
        ); 5] = [
            // Ordinary claims are unchanged.
            (
                fixed_block(100),
                BlockBasedMoment(1000),
                BlockBasedMoment(1500),
                10,
                BlockBasedMoment(1500),
                BlockBasedMoment(1500),
            ),
            (
                random_block(100),
                BlockBasedMoment(0),
                BlockBasedMoment(u64::MAX),
                3,
                BlockBasedMoment(300),
                BlockBasedMoment(300),
            ),
            (
                fixed_time(60_000),
                TimeBasedMoment(0),
                TimeBasedMoment(100_000),
                5,
                TimeBasedMoment(100_000),
                TimeBasedMoment(100_000),
            ),
            // An interval whose product saturates u64: v0 wrapped the start on top of it.
            (
                fixed_block(u64::MAX),
                BlockBasedMoment(5),
                BlockBasedMoment(1000),
                MAX_NON_FIXED_CYCLES,
                BlockBasedMoment(1000),
                BlockBasedMoment(4),
            ),
            (
                fixed_time(u64::MAX),
                TimeBasedMoment(1),
                TimeBasedMoment(7),
                MAX_NON_FIXED_CYCLES,
                TimeBasedMoment(7),
                TimeBasedMoment(0),
            ),
        ];

        for (distribution, start, current, max_non_fixed_cycles, expected_v14, expected_v13) in
            cases
        {
            let scenario = format!("{distribution} from {start:?} at {current:?}");
            assert_eq!(
                distribution
                    .max_cycle_moment(start, current, max_non_fixed_cycles, latest())
                    .unwrap_or_else(|e| panic!("latest failed for {scenario}: {e}")),
                expected_v14,
                "latest: {scenario}"
            );
            assert_eq!(
                distribution
                    .max_cycle_moment(start, current, max_non_fixed_cycles, v13())
                    .unwrap_or_else(|e| panic!("v13 failed for {scenario}: {e}")),
                expected_v13,
                "v13: {scenario}"
            );
        }
    }

    #[test]
    fn should_reject_mismatched_moment_types_in_every_version() {
        for platform_version in [latest(), v13()] {
            let result = fixed_block(100).max_cycle_moment(
                BlockBasedMoment(0),
                TimeBasedMoment(50),
                10,
                platform_version,
            );
            assert!(matches!(
                result,
                Err(ProtocolError::CorruptedCodeExecution(_))
            ));
        }
    }

    #[test]
    fn should_reject_an_unknown_max_cycle_moment_version() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .dpp
            .token_versions
            .reward_distribution_max_cycle_moment_version = 2;
        let result = fixed_epoch(2).max_cycle_moment(
            EpochBasedMoment(6),
            EpochBasedMoment(10),
            MAX_NON_FIXED_CYCLES,
            &platform_version,
        );
        assert!(matches!(
            result,
            Err(ProtocolError::UnknownVersionMismatch { received: 2, .. })
        ));
    }
}
