use crate::drive::Drive;
use crate::error::Error;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
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

impl Drive {
    /// Version 0 of [`Drive::evonode_participation_rewards`]: the claim every release ran up to
    /// protocol version 13, frozen.
    ///
    /// It reads the finalized epochs from `last_paid_epoch` (included) to the block's epoch
    /// (excluded), at most `drive_abci.query.max_returned_elements` of them, and evaluates the
    /// whole range up to `max_cycle_moment`, which it also pays through. A cycle whose epoch
    /// the read did not return fails a function other than a fixed amount with
    /// `MissingEpochInfo`, and the claim with it; a fixed amount weighs the whole range by the
    /// evonode's share of the blocks in the epochs that were returned.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn evonode_participation_rewards_v0(
        &self,
        evonode_id: Identifier,
        distribution_type: &RewardDistributionType,
        distribution_start: RewardDistributionMoment,
        last_paid_epoch: EpochIndex,
        max_cycle_moment: RewardDistributionMoment,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(TokenAmount, RewardDistributionMoment), Error> {
        let epochs: BTreeMap<EpochIndex, FinalizedEpochInfo> = self.get_finalized_epoch_infos(
            last_paid_epoch,
            true,
            block_info.epoch.index,
            false,
            platform_version.drive_abci.query.max_returned_elements,
            transaction,
            platform_version,
        )?;

        let rewards = distribution_type.rewards_in_interval(
            distribution_start,
            RewardDistributionMoment::EpochBasedMoment(last_paid_epoch),
            max_cycle_moment,
            Some(|range_epoch_index: RangeInclusive<EpochIndex>| {
                if range_epoch_index.start() == range_epoch_index.end() {
                    epochs
                        .get(range_epoch_index.start())
                        .map(|epoch_info| RewardRatio {
                            numerator: epoch_info
                                .block_proposers()
                                .get(&evonode_id)
                                .copied()
                                .unwrap_or_default(),
                            denominator: epoch_info.total_blocks_in_epoch(),
                        })
                } else {
                    let mut total_blocks = 0;
                    let mut total_proposed_blocks = 0;

                    for epoch_index in range_epoch_index {
                        if let Some(epoch_info) = epochs.get(&epoch_index) {
                            total_blocks += epoch_info.total_blocks_in_epoch();
                            total_proposed_blocks += epoch_info
                                .block_proposers()
                                .get(&evonode_id)
                                .copied()
                                .unwrap_or_default();
                        }
                    }

                    // Return ratio if we have non-zero total blocks
                    if total_blocks > 0 {
                        Some(RewardRatio {
                            numerator: total_proposed_blocks,
                            denominator: total_blocks,
                        })
                    } else {
                        None
                    }
                }
            }),
            platform_version,
        )?;

        Ok((rewards, max_cycle_moment))
    }
}
