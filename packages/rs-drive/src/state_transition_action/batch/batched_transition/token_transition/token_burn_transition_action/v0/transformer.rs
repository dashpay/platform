use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_burn_transition::v0::TokenBurnTransitionV0;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupActionAccessors;
use dpp::prelude::UserFeeIncrease;
use dpp::tokens::token_event::TokenEvent;
use dpp::validation::ConsensusValidationResult;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::v0::TokenBurnTransitionActionV0;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenBurnTransitionActionV0 {
    /// Attempts to create a `TokenBurnTransitionActionV0` from the given `TokenBurnTransitionV0` value.
    ///
    /// This function extracts the necessary data from the provided `TokenBurnTransitionV0` and
    /// delegates to the `try_from_base_transition_with_contract_lookup` function to construct the
    /// base action. It then constructs the `TokenBurnTransitionActionV0` struct by including the
    /// `burn_amount` and `public_note` values, along with the base action.
    ///
    /// # Parameters
    /// - `drive`: A reference to the `Drive` struct which provides access to the system.
    /// - `owner_id`: The identifier of the owner initiating the burn transition.
    /// - `value`: The `TokenBurnTransitionV0` containing the details for the token burn.
    /// - `approximate_without_state_for_costs`: A flag indicating whether to approximate state costs.
    /// - `transaction`: The transaction argument used for state changes.
    /// - `block_info`: Information about the current block to calculate fees.
    /// - `get_data_contract`: A closure function that looks up the data contract for a given identifier.
    /// - `platform_version`: The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    /// A `Result` containing the constructed `TokenBurnTransitionActionV0` on success, or an error
    /// if any issues occur during the process.
    ///
    /// # Errors
    /// - Returns an `Error` if any error occurs while trying to create the base action or process the burn.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenBurnTransitionV0,
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
        let TokenBurnTransitionV0 {
            base,
            burn_amount,
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

        let burn_from_identifier =
            if let Some(original_group_action) = base_action.original_group_action() {
                let GroupActionEvent::TokenEvent(TokenEvent::Burn(_, burn_from_id, _)) =
                    original_group_action.event()
                else {
                    return Err(ProtocolError::CorruptedCodeExecution(format!(
                        "received a non burn token event: {}",
                        original_group_action.event()
                    ))
                    .into());
                };
                *burn_from_id
            } else {
                owner_id
            };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::BurnAction(
                TokenBurnTransitionActionV0 {
                    base: base_action,
                    burn_from_identifier,
                    burn_amount,
                    public_note: change_note.unwrap_or(public_note),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }

    /// Attempts to create a `TokenBurnTransitionActionV0` from the borrowed `TokenBurnTransitionV0` value.
    ///
    /// This function is similar to `try_from_token_burn_transition_with_contract_lookup`, but it
    /// operates on a borrowed `TokenBurnTransitionV0` to avoid ownership transfer. It delegates
    /// to `try_from_borrowed_base_transition_with_contract_lookup` for constructing the base action,
    /// then combines it with the `burn_amount` and `public_note` to form a `TokenBurnTransitionActionV0`.
    ///
    /// # Parameters
    /// - `drive`: A reference to the `Drive` struct which provides access to the system.
    /// - `owner_id`: The identifier of the owner initiating the burn transition.
    /// - `value`: A borrowed reference to the `TokenBurnTransitionV0` containing the details for the token burn.
    /// - `approximate_without_state_for_costs`: A flag indicating whether to approximate state costs.
    /// - `transaction`: The transaction argument used for state changes.
    /// - `block_info`: Information about the current block to calculate fees.
    /// - `get_data_contract`: A closure function that looks up the data contract for a given identifier.
    /// - `platform_version`: The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    /// A `Result` containing the constructed `TokenBurnTransitionActionV0` on success, or an error
    /// if any issues occur during the process.
    ///
    /// # Errors
    /// - Returns an `Error` if any error occurs while trying to create the base action or process the burn.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenBurnTransitionV0,
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
        let TokenBurnTransitionV0 {
            base,
            burn_amount,
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

        let burn_from_identifier =
            if let Some(original_group_action) = base_action.original_group_action() {
                let GroupActionEvent::TokenEvent(TokenEvent::Burn(_, burn_from_id, _)) =
                    original_group_action.event()
                else {
                    return Err(ProtocolError::CorruptedCodeExecution(format!(
                        "received a non burn token event: {}",
                        original_group_action.event()
                    ))
                    .into());
                };
                *burn_from_id
            } else {
                owner_id
            };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::BurnAction(
                TokenBurnTransitionActionV0 {
                    base: base_action,
                    burn_from_identifier,
                    burn_amount: *burn_amount,
                    public_note: change_note.unwrap_or(public_note.clone()),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
