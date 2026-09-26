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
