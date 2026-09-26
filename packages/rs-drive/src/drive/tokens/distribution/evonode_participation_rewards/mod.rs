mod v0;
mod v1;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Computes the tokens an evonode earns from an `EvonodesByParticipation` perpetual
    /// distribution in one claim, and the moment the claim pays through.
    ///
    /// Every cycle's emission is weighted by the evonode's share of the blocks proposed in the
    /// epochs the cycle spans, which the finalized epoch infos of those epochs record. The
    /// moment returned is what the claim stores as the evonode's last paid moment, so the next
    /// claim starts after it.
    ///
    /// # Parameters
    ///
    /// - `evonode_id`: The claimant, whose proposed blocks are counted.
    /// - `distribution_type`: The token's epoch-based perpetual distribution.
    /// - `distribution_start`: The cycle start of the distribution's first moment, which the
    ///   distribution function counts its steps from.
    /// - `last_paid_epoch`: The epoch the claim starts after: the evonode's last paid moment,
    ///   or `distribution_start` for a first claim.
    /// - `max_cycle_moment`: The furthest moment the claim may pay through, as
    ///   `RewardDistributionType::max_cycle_moment` caps it.
    /// - `block_info`: The block the claim executes in.
    /// - `transaction`: The GroveDB transaction.
    /// - `platform_version`: Selects the version through
    ///   `drive.methods.token.distribution.evonode_participation_rewards`.
    ///
    /// # Returns
    ///
    /// The amount earned and the moment the claim pays through: `max_cycle_moment` in version
    /// 0; in version 1 the last whole cycle the claim read, or `last_paid_epoch` when it earns
    /// nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn evonode_participation_rewards(
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
        match platform_version
            .drive
            .methods
            .token
            .distribution
            .evonode_participation_rewards
        {
            0 => self.evonode_participation_rewards_v0(
                evonode_id,
                distribution_type,
                distribution_start,
                last_paid_epoch,
                max_cycle_moment,
                block_info,
                transaction,
                platform_version,
            ),
            1 => self.evonode_participation_rewards_v1(
                evonode_id,
                distribution_type,
                distribution_start,
                last_paid_epoch,
                max_cycle_moment,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "evonode_participation_rewards".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
