use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::token::{ChoosingTokenMintRecipientNotAllowedError, DestinationIdentityForTokenMintingNotSetError};
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_mint_transition::v0::TokenMintTransitionV0;
use dpp::ProtocolError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::v0::TokenMintTransitionActionV0;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenMintTransitionActionV0 {
    /// Converts a `TokenMintTransitionV0` into a `TokenMintTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token minting transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for minting.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the minting transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenMintTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `block_info` - Information about the current block to calculate fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<TokenMintTransitionActionV0>, Error>` - Returns the constructed `TokenMintTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_mint_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenMintTransitionV0,
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
        let TokenMintTransitionV0 {
            base,
            issued_to_identity_id,
            amount,
            public_note,
        } = value;

        let position = base.token_contract_position();

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

        let (base_action, change_note) = match base_action_validation_result.is_valid() {
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
                        batched_action,
                        base_action_validation_result.errors,
                    ),
                    fee_result,
                ));
            }
        };

        if !base_action
            .token_configuration()?
            .distribution_rules()
            .minting_allow_choosing_destination()
            && issued_to_identity_id.is_some()
        {
            let bump_action =
                BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(
                    &base_action,
                    owner_id,
                    user_fee_increase,
                );
            let batched_action =
                BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

            return Ok((
                ConsensusValidationResult::new_with_data_and_errors(
                    batched_action,
                    vec![BasicError::ChoosingTokenMintRecipientNotAllowedError(
                        ChoosingTokenMintRecipientNotAllowedError::new(base_action.token_id()),
                    )
                    .into()],
                ),
                fee_result,
            ));
        }

        let identity_balance_holder_id = match issued_to_identity_id.or_else(|| {
            base_action
                .data_contract_fetch_info_ref()
                .contract
                .tokens()
                .get(&position)
                .and_then(|token_configuration| {
                    token_configuration
                        .distribution_rules()
                        .new_tokens_destination_identity()
                        .copied()
                })
        }) {
            Some(identity_balance_holder_id) => identity_balance_holder_id,
            None => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(
                        &base_action,
                        owner_id,
                        0,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        vec![BasicError::DestinationIdentityForTokenMintingNotSetError(
                            DestinationIdentityForTokenMintingNotSetError::new(
                                base_action.token_id(),
                            ),
                        )
                        .into()],
                    ),
                    fee_result,
                ));
            }
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::MintAction(
                TokenMintTransitionActionV0 {
                    base: base_action,
                    mint_amount: amount,
                    identity_balance_holder_id,
                    public_note: change_note.unwrap_or(public_note),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenMintTransitionV0` into a `TokenMintTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token minting transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant minting logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the minting transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenMintTransitionV0` struct containing the transition data, including token
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
    /// * `Result<(ConsensusValidationResult<TokenMintTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenMintTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_mint_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenMintTransitionV0,
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
        let TokenMintTransitionV0 {
            base,
            issued_to_identity_id,
            amount,
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

        let fee_result = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            drive.config.epochs_per_era,
            platform_version,
            None,
        )?;

        let (base_action, change_note) = match base_action_validation_result.is_valid() {
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

        if !base_action
            .token_configuration()?
            .distribution_rules()
            .minting_allow_choosing_destination()
            && issued_to_identity_id.is_some()
        {
            let bump_action =
                BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(
                    &base_action,
                    owner_id,
                    user_fee_increase,
                );
            let batched_action =
                BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

            return Ok((
                ConsensusValidationResult::new_with_data_and_errors(
                    batched_action,
                    vec![BasicError::ChoosingTokenMintRecipientNotAllowedError(
                        ChoosingTokenMintRecipientNotAllowedError::new(base_action.token_id()),
                    )
                    .into()],
                ),
                fee_result,
            ));
        }

        let identity_balance_holder_id = match issued_to_identity_id.or_else(|| {
            base_action
                .data_contract_fetch_info_ref()
                .contract
                .tokens()
                .get(&base.token_contract_position())
                .and_then(|token_configuration| {
                    token_configuration
                        .distribution_rules()
                        .new_tokens_destination_identity()
                        .copied()
                })
        }) {
            Some(identity_balance_holder_id) => identity_balance_holder_id,
            None => {
                let bump_action =
                    BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(
                        &base_action,
                        owner_id,
                        0,
                    );
                let batched_action =
                    BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

                return Ok((
                    ConsensusValidationResult::new_with_data_and_errors(
                        batched_action,
                        vec![BasicError::DestinationIdentityForTokenMintingNotSetError(
                            DestinationIdentityForTokenMintingNotSetError::new(
                                base_action.token_id(),
                            ),
                        )
                        .into()],
                    ),
                    fee_result,
                ));
            }
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::MintAction(
                TokenMintTransitionActionV0 {
                    base: base_action,
                    mint_amount: *amount,
                    identity_balance_holder_id,
                    public_note: change_note.unwrap_or(public_note.clone()),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
