use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::balances::credits::TokenAmount;
use dpp::block::epoch::EpochIndex;
use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::reward_ratio::RewardRatio;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

/// The start of the cycle `epoch` falls in: the greatest multiple of `interval` not above it.
/// `None` for an interval of zero, which registration refuses from protocol version 14 and
/// which fails the claim before its rewards are computed on earlier contracts.
fn cycle_start(epoch: EpochIndex, interval: EpochIndex) -> Option<EpochIndex> {
    // A remainder never exceeds its dividend.
    epoch
        .checked_rem(interval)
        .map(|offset| epoch.saturating_sub(offset))
}

impl Drive {
    /// Version 1 of [`Drive::evonode_participation_rewards`], selected from protocol version
    /// 14: a claim covers only the epochs it read.
    ///
    /// It reads the finalized epochs after the cycle start of `last_paid_epoch` up to
    /// `max_cycle_moment`, at most `SystemLimits::max_evonode_reward_claim_epochs` of them, or
    /// one whole cycle when a cycle is longer, and pays through the last whole cycle ending at
    /// or before the last epoch read. An evonode further behind than that is paid over several
    /// claims, each continuing from the moment the previous one stored. When the last epoch
    /// the claim may pay has not been finalized yet (its info is written by the fee
    /// distribution of the first block of the next epoch, after that block's transitions),
    /// the claim pays through the cycles before it and leaves it for a later claim.
    ///
    /// Version 0 evaluated the whole range up to `max_cycle_moment` whatever it read: past the
    /// read limit or the last finalized epoch a function other than a fixed amount failed as
    /// an internal error, so the last paid moment never advanced and the evonode could never
    /// claim the token again, and a fixed amount weighed the whole range by the share of the
    /// epochs read.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn evonode_participation_rewards_v1(
        &self,
        evonode_id: Identifier,
        distribution_type: &RewardDistributionType,
        distribution_start: RewardDistributionMoment,
        last_paid_epoch: EpochIndex,
        max_cycle_moment: RewardDistributionMoment,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(TokenAmount, RewardDistributionMoment), Error> {
        let (
            RewardDistributionMoment::EpochBasedMoment(interval),
            RewardDistributionMoment::EpochBasedMoment(max_cycle_epoch),
        ) = (distribution_type.interval(), max_cycle_moment)
        else {
            return Err(Error::Drive(DriveError::NotSupported(
                "evonodes by participation can only use epoch based distribution",
            )));
        };

        let nothing_earned = (
            0,
            RewardDistributionMoment::EpochBasedMoment(last_paid_epoch),
        );

        if max_cycle_epoch <= last_paid_epoch {
            return Ok(nothing_earned);
        }

        // Every epoch the evaluation weighs comes after the cycle start of the last paid
        // epoch: that is the last paid epoch itself when it sits on a cycle boundary, and a
        // last paid moment a version 13 claim left off a boundary sits inside a cycle whose
        // earlier epochs are still to be weighed.
        let read_after = cycle_start(last_paid_epoch, interval).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "an epoch interval of zero fails the claim before its rewards are computed",
            ),
        ))?;

        // A cycle longer than the limit is still read whole, or no claim could ever pay it.
        let read_limit = interval.max(
            platform_version
                .system_limits
                .max_evonode_reward_claim_epochs,
        );

        let epochs: BTreeMap<EpochIndex, FinalizedEpochInfo> = self.get_finalized_epoch_infos(
            read_after,
            false,
            max_cycle_epoch,
            true,
            read_limit,
            transaction,
            platform_version,
        )?;

        let Some(&last_epoch_read) = epochs.keys().next_back() else {
            return Ok(nothing_earned);
        };

        let last_cycle_read = cycle_start(last_epoch_read, interval).ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution(
                "an epoch interval of zero fails the claim before its rewards are computed",
            ),
        ))?;
        let paid_through_epoch = max_cycle_epoch.min(last_cycle_read);

        if paid_through_epoch <= last_paid_epoch {
            return Ok(nothing_earned);
        }

        let paid_through = RewardDistributionMoment::EpochBasedMoment(paid_through_epoch);

        // Every epoch weighed lies after `read_after` and at or before the last epoch read, and
        // the ascending read returned every finalized epoch in that span. An epoch without
        // finalized info there will never get one: the fee distribution finalizes epochs in
        // order and skips the ones in which no block was produced, and epochs before protocol
        // version 9 were never finalized. Such an epoch adds no blocks to either side of the
        // ratio, and a cycle without any block pays nobody.
        let participation = |cycle_epochs: RangeInclusive<EpochIndex>| {
            let mut total_blocks: u64 = 0;
            let mut proposed_blocks: u64 = 0;
            for epoch_index in cycle_epochs {
                if let Some(epoch_info) = epochs.get(&epoch_index) {
                    // Every block belongs to one epoch, so neither sum can pass the chain
                    // height.
                    total_blocks = total_blocks.checked_add(epoch_info.total_blocks_in_epoch())?;
                    proposed_blocks = proposed_blocks.checked_add(
                        epoch_info
                            .block_proposers()
                            .get(&evonode_id)
                            .copied()
                            .unwrap_or_default(),
                    )?;
                }
            }
            Some(if total_blocks == 0 {
                RewardRatio {
                    numerator: 0,
                    denominator: 1,
                }
            } else {
                RewardRatio {
                    numerator: proposed_blocks,
                    denominator: total_blocks,
                }
            })
        };

        let rewards = distribution_type.rewards_in_interval(
            distribution_start,
            RewardDistributionMoment::EpochBasedMoment(last_paid_epoch),
            paid_through,
            Some(participation),
            platform_version,
        )?;

        Ok((rewards, paid_through))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;

    /// The epoch the distribution counts its cycles from.
    const DISTRIBUTION_START: EpochIndex = 0;

    /// Epochs in one cycle of the distribution claimed below. More than one, so that the cycle a
    /// claim pays through is not simply the last epoch it read, and not a divisor of the read
    /// limit, so that the limit falls inside a cycle rather than on its boundary.
    const INTERVAL: EpochIndex = 3;

    /// Blocks produced in each finalized epoch, and how many of them the claimant proposed. Its
    /// share of a whole cycle is therefore nine blocks of twelve; the rest go to another evonode,
    /// so a claim that counted every proposer's blocks as the claimant's would pay visibly more.
    const BLOCKS_PER_EPOCH: u64 = 4;
    const BLOCKS_PROPOSED_BY_CLAIMANT: u64 = 3;

    /// The distribution emits this much in each cycle up to cycle 33, and this much from cycle
    /// 34, so that the amount a claim pays says which cycles it paid and not merely how many.
    const EMISSION_UP_TO_CYCLE_33: TokenAmount = 400;
    const EMISSION_FROM_CYCLE_34: TokenAmount = 1_200;

    /// What one cycle pays the claimant on either side of that boundary: the cycle's emission at
    /// nine blocks of twelve, three quarters, which divides exactly for both emissions.
    const PAID_PER_CYCLE_UP_TO_33: TokenAmount = 300;
    const PAID_PER_CYCLE_FROM_34: TokenAmount = 900;

    /// What the fifty cycles spanning epochs 1 to 150 pay the claimant in all: thirty-three
    /// cycles at 300 and seventeen at 900. A single claim would pay this if it could read every
    /// epoch at once, so a claim split in two must still add up to it.
    const PAID_OVER_EPOCHS_1_TO_150: TokenAmount = 25_200;

    fn claimant() -> Identifier {
        Identifier::from([1; 32])
    }

    fn other_evonode() -> Identifier {
        Identifier::from([2; 32])
    }

    fn distribution() -> RewardDistributionType {
        RewardDistributionType::EpochBasedDistribution {
            interval: INTERVAL,
            function: DistributionFunction::Stepwise(BTreeMap::from([
                (0, EMISSION_UP_TO_CYCLE_33),
                (34, EMISSION_FROM_CYCLE_34),
            ])),
        }
    }

    /// Records `BLOCKS_PER_EPOCH` blocks in each of `epochs`, the claimant proposing
    /// `BLOCKS_PROPOSED_BY_CLAIMANT` of them and another evonode the rest, as the fee
    /// distribution of the first block of the following epoch does once an epoch is over.
    fn seed_finalized_epochs(
        drive: &Drive,
        epochs: impl IntoIterator<Item = EpochIndex>,
        platform_version: &PlatformVersion,
    ) {
        let transaction = drive.grove.start_transaction();
        for epoch_index in epochs {
            let finalized_epoch_info: FinalizedEpochInfo = FinalizedEpochInfoV0 {
                first_block_time: u64::from(epoch_index) * 1_000,
                first_block_height: u64::from(epoch_index) * BLOCKS_PER_EPOCH,
                total_blocks_in_epoch: BLOCKS_PER_EPOCH,
                first_core_block_height: 1,
                next_epoch_start_core_block_height: 1,
                total_processing_fees: 0,
                total_distributed_storage_fees: 0,
                total_created_storage_fees: 0,
                core_block_rewards: 0,
                block_proposers: BTreeMap::from([
                    (claimant(), BLOCKS_PROPOSED_BY_CLAIMANT),
                    (
                        other_evonode(),
                        BLOCKS_PER_EPOCH - BLOCKS_PROPOSED_BY_CLAIMANT,
                    ),
                ]),
                fee_multiplier_permille: 1_000,
                protocol_version: platform_version.protocol_version,
            }
            .into();
            let operation = drive
                .add_epoch_final_info_operation(
                    &Epoch::new(epoch_index).expect("expected a valid epoch"),
                    finalized_epoch_info,
                    platform_version,
                )
                .expect("expected a finalized epoch info operation");
            drive
                .grove_apply_operation(
                    operation,
                    false,
                    Some(&transaction),
                    &platform_version.drive,
                )
                .expect("expected to store the finalized epoch info");
        }
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the finalized epoch infos");
    }

    /// Claims through the dispatcher, so that the version tables choose the generation the same
    /// way a block does, and panics if the claim fails rather than reporting what it paid.
    fn claim(
        drive: &Drive,
        last_paid_epoch: EpochIndex,
        max_cycle_epoch: EpochIndex,
        platform_version: &PlatformVersion,
    ) -> (TokenAmount, RewardDistributionMoment) {
        // The claim runs while the cycle after the capped one is still going, as a real claim
        // does: the cap is the last completed cycle moment.
        let block_info = BlockInfo {
            epoch: Epoch::new(max_cycle_epoch + INTERVAL).expect("expected a valid epoch"),
            ..Default::default()
        };
        drive
            .evonode_participation_rewards(
                claimant(),
                &distribution(),
                RewardDistributionMoment::EpochBasedMoment(DISTRIBUTION_START),
                last_paid_epoch,
                RewardDistributionMoment::EpochBasedMoment(max_cycle_epoch),
                &block_info,
                None,
                platform_version,
            )
            .expect("expected the claim to compute its rewards")
    }

    /// The epochs seeded below and the cycles they fall in are laid out around the read limit, so
    /// a change to the limit must be reflected here rather than quietly moving what is covered.
    fn assert_scenario_matches_read_limit(platform_version: &PlatformVersion) {
        assert_eq!(
            platform_version
                .system_limits
                .max_evonode_reward_claim_epochs,
            100,
            "the seeded epochs and cycle boundaries are chosen around a read limit of 100"
        );
    }

    /// A claim reaching further back than the read limit allows pays through the last whole cycle
    /// inside the epochs it read, and reports that cycle as the moment it paid through. Reporting
    /// the cap instead would drop the cycles the claim never weighed; reporting no advance at all
    /// would leave the claim unrepeatable, which is how an evonode lost the token for good.
    #[test]
    fn should_pay_through_the_last_whole_cycle_read_when_a_claim_outruns_the_read_limit() {
        let platform_version = PlatformVersion::latest();
        assert_scenario_matches_read_limit(platform_version);
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        seed_finalized_epochs(&drive, 1..=150, platform_version);

        let (rewards, paid_through) = claim(&drive, 0, 150, platform_version);

        // The read stops at the hundredth epoch after the last paid one, epoch 100, which sits
        // inside the cycle spanning epochs 100 to 102. The last cycle read whole is therefore
        // cycle 33, ending at epoch 99, well short of the cap at epoch 150.
        assert_eq!(
            paid_through,
            RewardDistributionMoment::EpochBasedMoment(99),
            "a capped claim pays through the last whole cycle it read, not the cap it was given"
        );
        // Cycles 1 to 33, each paying the claimant three quarters of its 400 emission.
        assert_eq!(rewards, 33 * PAID_PER_CYCLE_UP_TO_33);
    }

    /// The moment a capped claim reports is where the next claim starts, and the two together pay
    /// exactly what one claim over the whole range would have: the seam neither pays a cycle
    /// twice nor skips one, and a third claim finds the range exhausted.
    #[test]
    fn should_continue_a_capped_claim_from_the_moment_the_previous_one_paid_through() {
        let platform_version = PlatformVersion::latest();
        assert_scenario_matches_read_limit(platform_version);
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        seed_finalized_epochs(&drive, 1..=150, platform_version);

        let (first_rewards, first_paid_through) = claim(&drive, 0, 150, platform_version);
        assert_eq!(
            first_paid_through,
            RewardDistributionMoment::EpochBasedMoment(99)
        );
        assert_eq!(first_rewards, 33 * PAID_PER_CYCLE_UP_TO_33);

        let RewardDistributionMoment::EpochBasedMoment(continue_from) = first_paid_through else {
            panic!("an epoch based distribution pays through an epoch based moment");
        };
        let (second_rewards, second_paid_through) =
            claim(&drive, continue_from, 150, platform_version);

        // Continuing from epoch 99 the read covers epochs 100 to 150, whose last whole cycle ends
        // at the cap: cycles 34 to 50, the seventeen the first claim left, each paying three
        // quarters of the larger 1,200 emission.
        assert_eq!(
            second_paid_through,
            RewardDistributionMoment::EpochBasedMoment(150)
        );
        assert_eq!(second_rewards, 17 * PAID_PER_CYCLE_FROM_34);

        assert_eq!(
            first_rewards + second_rewards,
            PAID_OVER_EPOCHS_1_TO_150,
            "two claims over a capped range must pay what one unbounded claim would have"
        );

        let (third_rewards, third_paid_through) = claim(&drive, 150, 150, platform_version);
        assert_eq!(third_rewards, 0, "every cycle is paid once");
        assert_eq!(
            third_paid_through,
            RewardDistributionMoment::EpochBasedMoment(150),
            "a claim that earns nothing leaves the last paid moment where it was"
        );
    }

    /// An epoch whose finalized info the fee distribution has not written yet ends the epochs a
    /// claim can read. A cycle reaching into it is left whole for a later claim, rather than paid
    /// early on the epochs of it that are finalized, or failing the claim.
    #[test]
    fn should_leave_a_cycle_reaching_an_epoch_not_finalized_yet_to_a_later_claim() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        // Epochs 1 to 8 are finalized and epoch 9 is not, so the cycle spanning epochs 7 to 9 is
        // the first the claim cannot weigh, while the cap allows it.
        seed_finalized_epochs(&drive, 1..=8, platform_version);

        let (rewards, paid_through) = claim(&drive, 0, 9, platform_version);

        // Cycles 1 and 2 span epochs 1 to 6, the epochs before the unfinished cycle.
        assert_eq!(
            paid_through,
            RewardDistributionMoment::EpochBasedMoment(6),
            "a cycle reaching an epoch without finalized info is not paid through"
        );
        assert_eq!(rewards, 2 * PAID_PER_CYCLE_UP_TO_33);

        // Once that epoch is finalized the cycle is paid whole, from where the claim stopped.
        seed_finalized_epochs(&drive, 9..=9, platform_version);
        let (later_rewards, later_paid_through) = claim(&drive, 6, 9, platform_version);

        assert_eq!(
            later_paid_through,
            RewardDistributionMoment::EpochBasedMoment(9)
        );
        assert_eq!(later_rewards, PAID_PER_CYCLE_UP_TO_33);
    }
}
