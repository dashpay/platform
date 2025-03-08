use std::collections::BTreeMap;
use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::InvalidTokenClaimPropertyMismatch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_key::{TokenDistributionInfo, TokenDistributionType};
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::{TokenDistributionRecipient, TokenDistributionResolvedRecipient};
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_claim_transition::v0::TokenClaimTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_claim_transition_action::v0::TokenClaimTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
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
                &base,
                approximate_without_state_for_costs,
                transaction,
                &mut drive_operations,
                get_data_contract,
                platform_version,
            )?;

        let fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        let base_action = match base_action_validation_result.is_valid() {
            true => base_action_validation_result.into_data()?,
            false => {
                let bump_action = BumpIdentityDataContractNonceAction::from_token_base_transition(
                    base,
                    owner_id,
                    user_fee_increase,
                );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action.into(),
                        base_action_validation_result.errors,
                    ),
                    fee_result,
                ));
            }
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::ClaimAction(
                TokenClaimTransitionActionV0 {
                    base: base_action,
                    amount: 0, //todo
                    distribution_info: todo!(),
                    public_note,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
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

        let base_action = match base_action_validation_result.is_valid() {
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
                        batched_action.into(),
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
                            batched_action.into(),
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
                let oldest_time = 0;

                let amount = 0;

                (amount, TokenDistributionInfo::PreProgrammed(0, owner_id))
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
                            batched_action.into(),
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

                let start_from_moment_for_distribution = last_paid_moment
                    .or(perpetual_distribution
                        .distribution_type()
                        .contract_creation_moment(&base_action.data_contract_fetch_info().contract))
                    .ok_or(Error::Drive(DriveError::ContractDoesNotHaveAStartMoment(
                        base_action.data_contract_fetch_info().contract.id(),
                    )))?;

                let last_paid_time_fee_result = Drive::calculate_fee(
                    None,
                    Some(last_paid_time_operations),
                    &block_info.epoch,
                    drive.config.epochs_per_era,
                    platform_version,
                    None,
                )?;

                fee_result.checked_add_assign(last_paid_time_fee_result)?;

                let recipient = match perpetual_distribution.distribution_recipient() {
                    TokenDistributionRecipient::ContractOwner => {
                        TokenDistributionResolvedRecipient::ContractOwnerIdentity(
                            base_action.data_contract_fetch_info().contract.owner_id(),
                        )
                    }
                    TokenDistributionRecipient::Identity(identifier) => {
                        TokenDistributionResolvedRecipient::Identity(identifier)
                    }
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
                        TokenDistributionResolvedRecipient::Evonode(owner_id)
                    }
                };

                let amount = perpetual_distribution
                    .distribution_type()
                    .rewards_in_interval(start_from_moment_for_distribution, block_info)?;

                (
                    amount,
                    TokenDistributionInfo::Perpetual(
                        RewardDistributionMoment::TimeBasedMoment(0),
                        RewardDistributionMoment::TimeBasedMoment(0),
                        recipient,
                    ),
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
