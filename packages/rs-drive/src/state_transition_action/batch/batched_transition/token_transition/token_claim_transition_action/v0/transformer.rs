use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{InvalidTokenClaimNoCurrentRewards, InvalidTokenClaimPropertyMismatch, InvalidTokenClaimWrongClaimant};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_key::{TokenDistributionInfo, TokenDistributionType};
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::reward_ratio::RewardRatio;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::{TokenDistributionRecipient, TokenDistributionResolvedRecipient};
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::{TokenPerpetualDistributionV0Accessors, TokenPerpetualDistributionV0Methods};
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_claim_transition::v0::TokenClaimTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::v0::TokenClaimTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, TimestampMillis, UserFeeIncrease};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenClaimTransitionActionV0 {
    /// Converts a `TokenClaimTransitionV0` into a `TokenClaimTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token releasing transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for releasing.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the releasing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenClaimTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `block_info` - Information about the current block to calculate fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<TokenClaimTransitionActionV0>, Error>` - Returns the constructed `TokenClaimTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_claim_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenClaimTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        Self::try_from_borrowed_token_claim_transition_with_contract_lookup(
            drive,
            owner_id,
            &value,
            approximate_without_state_for_costs,
            transaction,
            block_info,
            user_fee_increase,
            get_data_contract,
            platform_version,
        )
    }

    /// Converts a borrowed `TokenClaimTransitionV0` into a `TokenClaimTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token releasing transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant releasing logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the releasing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenClaimTransitionV0` struct containing the transition data, including token
    ///   amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to indicate whether costs should be approximated without full
    ///   state consideration. Useful for optimizing transaction cost calculations in scenarios where full state is not needed.
    /// * `transaction` - The transaction context, which includes the necessary state and other details for the transition.
    /// * `block_info` - Information about the current block (e.g., epoch) to help calculate transaction fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version to ensure the transition respects version-specific logic.
    ///
    //// # Returns
    ///
    /// * `Result<(ConsensusValidationResult<TokenClaimTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenClaimTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_claim_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenClaimTransitionV0,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        let TokenClaimTransitionV0 {
            base,
            distribution_type,
            public_note,
        } = value;

        let mut drive_operations = vec![];

        let base_action_validation_result =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                drive,
                owner_id,
                base,
                approximate_without_state_for_costs,
                transaction,
                &mut drive_operations,
                get_data_contract,
                platform_version,
            )?;

        let mut fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        // We can not change the note on a claim
        let (base_action, _change_note) = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                        base,
                        owner_id,
                        user_fee_increase,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        base_action_validation_result.errors,
                    ),
                    fee_result,
                ));
            }
        };

        let token_config = base_action.token_configuration()?;

        let (amount, distribution_info) = match distribution_type {
            TokenDistributionType::PreProgrammed => {
                let Some(pre_programmed_distribution) = token_config
                    .distribution_rules()
                    .pre_programmed_distribution()
                else {
                    let bump_action =
                        BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                            base,
                            owner_id,
                            user_fee_increase,
                        );
                    let batched_action =
                        BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                    return Ok((
                        ConsensusValidationResult::new_with_data_and_errors(
                            batched_action,
                            vec![ConsensusError::StateError(
                                StateError::InvalidTokenClaimPropertyMismatch(
                                    InvalidTokenClaimPropertyMismatch::new(
                                        "pre programmed distribution",
                                        base.token_id(),
                                    ),
                                ),
                            )],
                        ),
                        fee_result,
                    ));
                };

                // We need to find the oldest pre-programmed distribution that wasn't yet claimed
                // for this identity

                let times = pre_programmed_distribution.distributions();

                let mut last_paid_time_operations = vec![];

                let last_paid_moment = drive
                    .fetch_pre_programmed_distribution_last_paid_time_ms_operations(
                        base.token_id().to_buffer(),
                        owner_id,
                        &mut last_paid_time_operations,
                        transaction,
                        platform_version,
                    )?;

                let last_paid_time_fee_result = Drive::calculate_fee(
                    None,
                    Some(last_paid_time_operations),
                    &block_info.epoch,
                    drive.config.epochs_per_era,
                    platform_version,
                    None,
                )?;

                fee_result.checked_add_assign(last_paid_time_fee_result)?;

                let distributions_in_past_for_owner: BTreeMap<TimestampMillis, TokenAmount> = times
                    .iter()
                    .filter_map(|(timestamp, distribution)| {
                        if timestamp > &block_info.time_ms {
                            // Don't get the ones in the future
                            None
                        } else {
                            distribution
                                .get(&owner_id)
                                .map(|amount| (*timestamp, *amount))
                        }
                    })
                    .collect();

                let distribution_after_last_paid: Option<(TimestampMillis, TokenAmount)> =
                    if let Some(last_paid) = last_paid_moment {
                        // Find the earliest distribution after the last paid moment
                        distributions_in_past_for_owner
                            .range((last_paid + 1)..)
                            .next()
                            .map(|(timestamp, amount)| (*timestamp, *amount))
                    } else {
                        // If never paid, take the earliest available distribution
                        distributions_in_past_for_owner
                            .first_key_value()
                            .map(|(timestamp, amount)| (*timestamp, *amount))
                    };

                let Some((payout_time, amount)) = distribution_after_last_paid else {
                    let bump_action =
                        BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                            base,
                            owner_id,
                            user_fee_increase,
                        );
                    let batched_action =
                        BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                    return Ok((
                        ConsensusValidationResult::new_with_data_and_errors(
                            batched_action,
                            vec![ConsensusError::StateError(
                                StateError::InvalidTokenClaimNoCurrentRewards(
                                    InvalidTokenClaimNoCurrentRewards::new(
                                        value.base().token_id(),
                                        owner_id,
                                        RewardDistributionMoment::TimeBasedMoment(
                                            block_info.time_ms,
                                        ),
                                        last_paid_moment
                                            .map(RewardDistributionMoment::TimeBasedMoment),
                                    ),
                                ),
                            )],
                        ),
                        fee_result,
                    ));
                };

                (
                    amount,
                    TokenDistributionInfo::PreProgrammed(payout_time, owner_id),
                )
            }
            TokenDistributionType::Perpetual => {
                // we need to validate that we have a perpetual distribution
                let Some(perpetual_distribution) =
                    token_config.distribution_rules().perpetual_distribution()
                else {
                    let bump_action =
                        BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                            base,
                            owner_id,
                            user_fee_increase,
                        );
                    let batched_action =
                        BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                    return Ok((
                        ConsensusValidationResult::new_with_data_and_errors(
                            batched_action,
                            vec![ConsensusError::StateError(
                                StateError::InvalidTokenClaimPropertyMismatch(
                                    InvalidTokenClaimPropertyMismatch::new(
                                        "perpetual distribution",
                                        value.base().token_id(),
                                    ),
                                ),
                            )],
                        ),
                        fee_result,
                    ));
                };

                // Let's start by checking that we have a valid claimant
                let wrong_claimant_error = match perpetual_distribution.distribution_recipient() {
                    TokenDistributionRecipient::ContractOwner
                        if base_action.data_contract_fetch_info().contract.owner_id()
                            != owner_id =>
                    {
                        Some(base_action.data_contract_fetch_info().contract.owner_id())
                    }
                    TokenDistributionRecipient::Identity(identifier) if identifier != owner_id => {
                        Some(identifier)
                    }
                    _ => None,
                };

                if let Some(expected_claimant) = wrong_claimant_error {
                    let bump_action =
                        BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                            base,
                            owner_id,
                            user_fee_increase,
                        );
                    let batched_action =
                        BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                    return Ok((
                        ConsensusValidationResult::new_with_data_and_errors(
                            batched_action,
                            vec![ConsensusError::StateError(
                                StateError::InvalidTokenClaimWrongClaimant(
                                    InvalidTokenClaimWrongClaimant::new(
                                        value.base().token_id(),
                                        expected_claimant,
                                        owner_id,
                                    ),
                                ),
                            )],
                        ),
                        fee_result,
                    ));
                }

                let mut last_paid_time_operations = vec![];

                let last_paid_moment = drive
                    .fetch_perpetual_distribution_last_paid_moment_operations(
                        base.token_id().to_buffer(),
                        owner_id,
                        perpetual_distribution.distribution_type(),
                        &mut last_paid_time_operations,
                        transaction,
                        platform_version,
                    )?;

                // if the token has never been paid then we use the token creation

                let contract_creation_moment = perpetual_distribution
                    .distribution_type()
                    .contract_creation_moment(&base_action.data_contract_fetch_info().contract)
                    .ok_or(Error::Drive(DriveError::ContractDoesNotHaveAStartMoment(
                        base_action.data_contract_fetch_info().contract.id(),
                    )))?;

                let contract_creation_cycle_start = contract_creation_moment
                    .cycle_start(perpetual_distribution.distribution_type().interval())?;

                let start_from_moment_for_distribution =
                    last_paid_moment.unwrap_or(contract_creation_cycle_start);

                let last_paid_time_fee_result = Drive::calculate_fee(
                    None,
                    Some(last_paid_time_operations),
                    &block_info.epoch,
                    drive.config.epochs_per_era,
                    platform_version,
                    None,
                )?;

                fee_result.checked_add_assign(last_paid_time_fee_result)?;

                let current_cycle_moment = perpetual_distribution.current_interval(block_info);

                // We need to get the max cycles allowed
                let max_cycles = platform_version.system_limits.max_token_redemption_cycles;
                let max_cycle_moment = perpetual_distribution
                    .distribution_type()
                    .max_cycle_moment(
                        start_from_moment_for_distribution,
                        current_cycle_moment,
                        max_cycles,
                    )?;

                let (recipient, amount) = match perpetual_distribution.distribution_recipient() {
                    TokenDistributionRecipient::ContractOwner => (
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(
                            base_action.data_contract_fetch_info().contract.owner_id(),
                        ),
                        perpetual_distribution
                            .distribution_type()
                            .rewards_in_interval::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>(
                                contract_creation_cycle_start,
                                start_from_moment_for_distribution,
                                max_cycle_moment,
                                None,
                                platform_version,
                            )?,
                    ),
                    TokenDistributionRecipient::Identity(identifier) => (
                        TokenDistributionResolvedRecipient::Identity(identifier),
                        perpetual_distribution
                            .distribution_type()
                            .rewards_in_interval::<fn(RangeInclusive<EpochIndex>) -> Option<RewardRatio>>(
                                contract_creation_cycle_start,
                                start_from_moment_for_distribution,
                                max_cycle_moment,
                                None,
                                platform_version,
                            )?,
                    ),
                    TokenDistributionRecipient::EvonodesByParticipation => {
                        let RewardDistributionMoment::EpochBasedMoment(epoch_index) =
                            start_from_moment_for_distribution
                        else {
                            return Err(Error::Drive(DriveError::NotSupported(
                                "evonodes by participation can only use epoch based distribution",
                            )));
                        };

                        let epochs: BTreeMap<EpochIndex, FinalizedEpochInfo> = drive
                            .get_finalized_epoch_infos(
                                epoch_index,
                                true,
                                block_info.epoch.index,
                                false,
                                transaction,
                                platform_version,
                            )?;

                        let rewards = perpetual_distribution
                            .distribution_type()
                            .rewards_in_interval(
                                contract_creation_cycle_start,
                                start_from_moment_for_distribution,
                                max_cycle_moment,
                                Some(|range_epoch_index: RangeInclusive<EpochIndex>| {
                                    if range_epoch_index.start() == range_epoch_index.end() {
                                        epochs.get(range_epoch_index.start()).map(|epoch_info| RewardRatio {
                                            numerator: epoch_info
                                                .block_proposers()
                                                .get(&owner_id)
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
                                                    .get(&owner_id)
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

                        (
                            TokenDistributionResolvedRecipient::Evonode(owner_id),
                            rewards,
                        )
                    }
                };

                if amount == 0 {
                    let bump_action =
                        BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                            base,
                            owner_id,
                            user_fee_increase,
                        );
                    let batched_action =
                        BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                    return Ok((
                        ConsensusValidationResult::new_with_data_and_errors(
                            batched_action,
                            vec![ConsensusError::StateError(
                                StateError::InvalidTokenClaimNoCurrentRewards(
                                    InvalidTokenClaimNoCurrentRewards::new(
                                        value.base().token_id(),
                                        owner_id,
                                        current_cycle_moment,
                                        last_paid_moment,
                                    ),
                                ),
                            )],
                        ),
                        fee_result,
                    ));
                }

                (
                    amount,
                    TokenDistributionInfo::Perpetual(max_cycle_moment, recipient),
                )
            }
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::ClaimAction(
                TokenClaimTransitionActionV0 {
                    base: base_action,
                    amount,
                    distribution_info,
                    public_note: public_note.clone(),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_literal_unwrap)]
mod tests {
    //! Unit tests for the logic fragments of `try_from_borrowed_token_claim_transition_with_contract_lookup`
    //! that can be exercised in isolation (without wiring up a full `Drive` instance).
    //!
    //! These cover:
    //!   * the pre-programmed distribution filtering + "distribution after last paid" lookup
    //!   * the `wrong_claimant_error` resolution logic
    //!   * `RewardRatio` computation used inside the `EvonodesByParticipation` closure
    //!   * recipient resolution for each `TokenDistributionInfo` variant
    //!   * the `From<TokenDistributionResolvedRecipient> for TokenDistributionRecipient` roundtrip
    //!   * `ClaimAction` variant dispatch / clone / enum wrapper accessors
    use dpp::balances::credits::TokenAmount;
    use dpp::block::epoch::EpochIndex;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionInfo;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::reward_ratio::RewardRatio;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::{
        TokenDistributionRecipient, TokenDistributionResolvedRecipient,
    };
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
    use dpp::identifier::Identifier;
    use dpp::prelude::TimestampMillis;
    use std::collections::BTreeMap;

    /// Replica of the pre-programmed distribution filter used inside the
    /// transformer: exclude future timestamps and keep only entries that
    /// actually target `owner_id`.
    fn filter_past_for_owner(
        times: &BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
        owner_id: Identifier,
        now_ms: TimestampMillis,
    ) -> BTreeMap<TimestampMillis, TokenAmount> {
        times
            .iter()
            .filter_map(|(timestamp, distribution)| {
                if *timestamp > now_ms {
                    None
                } else {
                    distribution
                        .get(&owner_id)
                        .map(|amount| (*timestamp, *amount))
                }
            })
            .collect()
    }

    fn owner() -> Identifier {
        Identifier::from([0xAB; 32])
    }

    fn stranger() -> Identifier {
        Identifier::from([0xCD; 32])
    }

    #[test]
    fn pre_programmed_filter_excludes_future_timestamps() {
        let me = owner();
        let mut past = BTreeMap::new();
        past.insert(me, 100u64);
        let mut future = BTreeMap::new();
        future.insert(me, 999u64);

        let mut times: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>> =
            BTreeMap::new();
        times.insert(500, past);
        times.insert(2_000, future);

        let now = 1_000u64;
        let result = filter_past_for_owner(&times, me, now);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&500), Some(&100));
    }

    #[test]
    fn pre_programmed_filter_ignores_non_owner_entries() {
        let me = owner();
        let other = stranger();
        let mut only_other = BTreeMap::new();
        only_other.insert(other, 42u64);
        let mut has_me = BTreeMap::new();
        has_me.insert(other, 7u64);
        has_me.insert(me, 13u64);

        let mut times: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>> =
            BTreeMap::new();
        times.insert(100, only_other);
        times.insert(200, has_me);

        let result = filter_past_for_owner(&times, me, 1_000);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&200), Some(&13));
    }

    #[test]
    fn pre_programmed_filter_keeps_entries_exactly_at_now() {
        // The filter uses strict `> now`, so entries with `timestamp == now` must be kept.
        let me = owner();
        let mut dist = BTreeMap::new();
        dist.insert(me, 11u64);

        let mut times: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>> =
            BTreeMap::new();
        times.insert(1_000, dist);

        let result = filter_past_for_owner(&times, me, 1_000);
        assert_eq!(result.get(&1_000), Some(&11));
    }

    #[test]
    fn distribution_after_last_paid_uses_first_when_never_paid() {
        // Mirrors `if let Some(last_paid) ... else { first_key_value() }` branch.
        let mut distributions: BTreeMap<TimestampMillis, TokenAmount> = BTreeMap::new();
        distributions.insert(100, 5);
        distributions.insert(200, 7);
        distributions.insert(300, 9);

        let last_paid: Option<TimestampMillis> = None;
        let picked = if let Some(last) = last_paid {
            distributions
                .range((last + 1)..)
                .next()
                .map(|(ts, amount)| (*ts, *amount))
        } else {
            distributions
                .first_key_value()
                .map(|(ts, amount)| (*ts, *amount))
        };
        assert_eq!(picked, Some((100, 5)));
    }

    #[test]
    fn distribution_after_last_paid_returns_next_entry_strictly_after() {
        let mut distributions: BTreeMap<TimestampMillis, TokenAmount> = BTreeMap::new();
        distributions.insert(100, 5);
        distributions.insert(200, 7);
        distributions.insert(300, 9);

        // last_paid == 100 => earliest AFTER it is 200.
        let last_paid: Option<TimestampMillis> = Some(100);
        let picked = distributions
            .range((last_paid.unwrap() + 1)..)
            .next()
            .map(|(ts, amount)| (*ts, *amount));
        assert_eq!(picked, Some((200, 7)));
    }

    #[test]
    fn distribution_after_last_paid_returns_none_when_all_claimed() {
        // When last_paid is at/above every available timestamp, the lookup returns None,
        // which in the transformer yields `InvalidTokenClaimNoCurrentRewards`.
        let mut distributions: BTreeMap<TimestampMillis, TokenAmount> = BTreeMap::new();
        distributions.insert(100, 5);
        distributions.insert(200, 7);

        let last_paid = Some(200u64);
        let picked = distributions
            .range((last_paid.unwrap() + 1)..)
            .next()
            .map(|(ts, amount)| (*ts, *amount));
        assert!(picked.is_none());
    }

    #[test]
    fn wrong_claimant_error_contract_owner_mismatch() {
        // Replicates the wrong_claimant_error match in the Perpetual branch:
        // ContractOwner recipient + owner_id != contract.owner_id => Some(contract.owner_id)
        let contract_owner = stranger();
        let caller = owner();
        let recipient = TokenDistributionRecipient::ContractOwner;
        let wrong_claimant_error = match recipient {
            TokenDistributionRecipient::ContractOwner if contract_owner != caller => {
                Some(contract_owner)
            }
            TokenDistributionRecipient::Identity(identifier) if identifier != caller => {
                Some(identifier)
            }
            _ => None,
        };
        assert_eq!(wrong_claimant_error, Some(contract_owner));
    }

    #[test]
    fn wrong_claimant_error_contract_owner_match_returns_none() {
        let caller = owner();
        let recipient = TokenDistributionRecipient::ContractOwner;
        let wrong = match recipient {
            TokenDistributionRecipient::ContractOwner if caller != caller => Some(caller),
            TokenDistributionRecipient::Identity(identifier) if identifier != caller => {
                Some(identifier)
            }
            _ => None,
        };
        assert!(wrong.is_none());
    }

    #[test]
    fn wrong_claimant_error_identity_mismatch() {
        let caller = owner();
        let expected = stranger();
        let recipient = TokenDistributionRecipient::Identity(expected);
        let wrong = match recipient {
            TokenDistributionRecipient::ContractOwner => None,
            TokenDistributionRecipient::Identity(identifier) if identifier != caller => {
                Some(identifier)
            }
            _ => None,
        };
        assert_eq!(wrong, Some(expected));
    }

    #[test]
    fn wrong_claimant_error_identity_match_returns_none() {
        let caller = owner();
        let recipient = TokenDistributionRecipient::Identity(caller);
        let wrong = match recipient {
            TokenDistributionRecipient::ContractOwner => None,
            TokenDistributionRecipient::Identity(identifier) if identifier != caller => {
                Some(identifier)
            }
            _ => None,
        };
        assert!(wrong.is_none());
    }

    #[test]
    fn wrong_claimant_error_evonodes_by_participation_is_always_none() {
        // EvonodesByParticipation isn't treated as a wrong-claimant error: it falls through
        // to the `_ => None` arm in the match.
        let caller = owner();
        let recipient = TokenDistributionRecipient::EvonodesByParticipation;
        let wrong = match recipient {
            TokenDistributionRecipient::ContractOwner if caller != caller => Some(caller),
            TokenDistributionRecipient::Identity(identifier) if identifier != caller => {
                Some(identifier)
            }
            _ => None,
        };
        assert!(wrong.is_none());
    }

    #[test]
    fn distribution_info_pre_programmed_recipient_resolution() {
        // Mirrors `TokenClaimTransitionActionV0::recipient` for the PreProgrammed variant.
        let id = Identifier::from([0xEE; 32]);
        let info = TokenDistributionInfo::PreProgrammed(1_000, id);
        let recipient = match &info {
            TokenDistributionInfo::PreProgrammed(_, identifier) => {
                TokenDistributionRecipient::Identity(*identifier)
            }
            TokenDistributionInfo::Perpetual(_, resolved_recipient) => resolved_recipient.into(),
        };
        assert_eq!(recipient, TokenDistributionRecipient::Identity(id));
    }

    #[test]
    fn distribution_info_perpetual_contract_owner_recipient() {
        let id = Identifier::from([0x1A; 32]);
        let info = TokenDistributionInfo::Perpetual(
            RewardDistributionMoment::TimeBasedMoment(42),
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(id),
        );
        let recipient = match &info {
            TokenDistributionInfo::PreProgrammed(_, identifier) => {
                TokenDistributionRecipient::Identity(*identifier)
            }
            TokenDistributionInfo::Perpetual(_, resolved_recipient) => resolved_recipient.into(),
        };
        assert_eq!(recipient, TokenDistributionRecipient::ContractOwner);
    }

    #[test]
    fn distribution_info_perpetual_identity_recipient() {
        let id = Identifier::from([0x2B; 32]);
        let info = TokenDistributionInfo::Perpetual(
            RewardDistributionMoment::BlockBasedMoment(7),
            TokenDistributionResolvedRecipient::Identity(id),
        );
        let recipient = match &info {
            TokenDistributionInfo::PreProgrammed(_, identifier) => {
                TokenDistributionRecipient::Identity(*identifier)
            }
            TokenDistributionInfo::Perpetual(_, resolved_recipient) => resolved_recipient.into(),
        };
        assert_eq!(recipient, TokenDistributionRecipient::Identity(id));
    }

    #[test]
    fn distribution_info_perpetual_evonode_recipient() {
        let id = Identifier::from([0x3C; 32]);
        let info = TokenDistributionInfo::Perpetual(
            RewardDistributionMoment::EpochBasedMoment(9),
            TokenDistributionResolvedRecipient::Evonode(id),
        );
        let recipient = match &info {
            TokenDistributionInfo::PreProgrammed(_, identifier) => {
                TokenDistributionRecipient::Identity(*identifier)
            }
            TokenDistributionInfo::Perpetual(_, resolved_recipient) => resolved_recipient.into(),
        };
        assert_eq!(
            recipient,
            TokenDistributionRecipient::EvonodesByParticipation
        );
    }

    /// Reimplements the closure passed to `rewards_in_interval` inside the
    /// `EvonodesByParticipation` branch and checks its single-epoch output
    /// matches the blocks proposed by the owner.
    #[test]
    fn evonodes_reward_ratio_single_epoch() {
        let me = owner();
        let other = stranger();
        let mut block_proposers: BTreeMap<Identifier, u64> = BTreeMap::new();
        block_proposers.insert(me, 7);
        block_proposers.insert(other, 3);
        let total_blocks_in_epoch = 10u64;

        let ratio = RewardRatio {
            numerator: block_proposers.get(&me).copied().unwrap_or_default(),
            denominator: total_blocks_in_epoch,
        };
        assert_eq!(ratio.numerator, 7);
        assert_eq!(ratio.denominator, 10);
    }

    /// Multi-epoch branch should sum totals and only count blocks proposed by the owner.
    #[test]
    fn evonodes_reward_ratio_multi_epoch_sum() {
        let me = owner();
        let mut epochs: BTreeMap<EpochIndex, (u64, BTreeMap<Identifier, u64>)> = BTreeMap::new();

        let mut bp1 = BTreeMap::new();
        bp1.insert(me, 5);
        epochs.insert(0, (20, bp1));

        let mut bp2 = BTreeMap::new();
        bp2.insert(me, 2);
        epochs.insert(1, (30, bp2));

        let mut total_blocks = 0u64;
        let mut total_proposed_blocks = 0u64;
        for idx in 0u16..=1u16 {
            if let Some((blocks, proposers)) = epochs.get(&idx) {
                total_blocks += *blocks;
                total_proposed_blocks += proposers.get(&me).copied().unwrap_or_default();
            }
        }
        assert_eq!(total_blocks, 50);
        assert_eq!(total_proposed_blocks, 7);
        let ratio = if total_blocks > 0 {
            Some(RewardRatio {
                numerator: total_proposed_blocks,
                denominator: total_blocks,
            })
        } else {
            None
        };
        assert_eq!(
            ratio,
            Some(RewardRatio {
                numerator: 7,
                denominator: 50,
            })
        );
    }

    /// With no epochs in range, total_blocks remains 0 so the ratio is None.
    #[test]
    fn evonodes_reward_ratio_empty_range_returns_none() {
        let total_blocks = 0u64;
        let total_proposed_blocks = 0u64;
        let ratio = if total_blocks > 0 {
            Some(RewardRatio {
                numerator: total_proposed_blocks,
                denominator: total_blocks,
            })
        } else {
            None
        };
        assert!(ratio.is_none());
    }

    /// When an epoch index exists but the owner never proposed, numerator is 0.
    #[test]
    fn evonodes_reward_ratio_owner_has_no_proposed_blocks() {
        let me = owner();
        let other = stranger();
        let mut bp: BTreeMap<Identifier, u64> = BTreeMap::new();
        bp.insert(other, 4);
        let total_blocks = 10u64;

        let ratio = RewardRatio {
            numerator: bp.get(&me).copied().unwrap_or_default(),
            denominator: total_blocks,
        };
        assert_eq!(ratio.numerator, 0);
        assert_eq!(ratio.denominator, 10);
    }

    /// The `last_paid_moment.unwrap_or(contract_creation_cycle_start)` fallback
    /// should pick the explicit last-paid if set, otherwise the cycle start.
    #[test]
    fn start_from_moment_fallback_uses_cycle_start_when_never_paid() {
        let cycle_start = RewardDistributionMoment::TimeBasedMoment(500);
        let last_paid: Option<RewardDistributionMoment> = None;
        let start = last_paid.unwrap_or(cycle_start);
        assert_eq!(start, RewardDistributionMoment::TimeBasedMoment(500));
    }

    #[test]
    fn start_from_moment_fallback_uses_last_paid_when_present() {
        let cycle_start = RewardDistributionMoment::TimeBasedMoment(500);
        let last_paid = Some(RewardDistributionMoment::TimeBasedMoment(1_000));
        let start = last_paid.unwrap_or(cycle_start);
        assert_eq!(start, RewardDistributionMoment::TimeBasedMoment(1_000));
    }

    /// The `From` impl converting `TokenDistributionResolvedRecipient` back to
    /// `TokenDistributionRecipient` is used inside the `recipient()` accessor.
    #[test]
    fn resolved_recipient_into_recipient_contract_owner_identity() {
        let r =
            TokenDistributionResolvedRecipient::ContractOwnerIdentity(Identifier::from([1u8; 32]));
        let into: TokenDistributionRecipient = (&r).into();
        assert_eq!(into, TokenDistributionRecipient::ContractOwner);
    }

    #[test]
    fn resolved_recipient_into_recipient_evonode() {
        let r = TokenDistributionResolvedRecipient::Evonode(Identifier::from([2u8; 32]));
        let into: TokenDistributionRecipient = (&r).into();
        assert_eq!(into, TokenDistributionRecipient::EvonodesByParticipation);
    }

    #[test]
    fn resolved_recipient_into_recipient_identity_preserves_identifier() {
        let id = Identifier::from([3u8; 32]);
        let r = TokenDistributionResolvedRecipient::Identity(id);
        let into: TokenDistributionRecipient = (&r).into();
        assert_eq!(into, TokenDistributionRecipient::Identity(id));
    }
}
