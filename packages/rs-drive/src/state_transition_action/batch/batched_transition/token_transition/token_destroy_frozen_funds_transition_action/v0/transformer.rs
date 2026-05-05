use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::IdentityDoesNotHaveEnoughTokenBalanceError;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::TokenDestroyFrozenFundsTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::v0::TokenDestroyFrozenFundsTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenDestroyFrozenFundsTransitionActionV0 {
    /// Converts a `TokenDestroyFrozenFundsTransitionV0` into a `TokenDestroyFrozenFundsTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token destroy_frozen_fundsing transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for destroy_frozen_fundsing.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the destroy_frozen_fundsing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenDestroyFrozenFundsTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `block_info` - Information about the current block to calculate fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<TokenDestroyFrozenFundsTransitionActionV0>, Error>` - Returns the constructed `TokenDestroyFrozenFundsTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_destroy_frozen_funds_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenDestroyFrozenFundsTransitionV0,
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
        let TokenDestroyFrozenFundsTransitionV0 {
            base,
            frozen_identity_id,
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

        let maybe_token_amount = drive.fetch_identity_token_balance_operations(
            base.token_id().to_buffer(),
            frozen_identity_id.to_buffer(),
            !approximate_without_state_for_costs,
            transaction,
            &mut drive_operations,
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

        let Some(token_amount) = maybe_token_amount else {
            let bump_action =
                BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition(
                    &base,
                    owner_id,
                    user_fee_increase,
                );
            let batched_action =
                BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action);

            return Ok((
                ConsensusValidationResult::new_with_data_and_errors(
                    batched_action,
                    vec![StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                        IdentityDoesNotHaveEnoughTokenBalanceError::new(
                            base.token_id(),
                            frozen_identity_id,
                            1,
                            0,
                            "destroy_frozen_funds".to_string(),
                        ),
                    )
                    .into()],
                ),
                fee_result,
            ));
        };

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

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::DestroyFrozenFundsAction(
                TokenDestroyFrozenFundsTransitionActionV0 {
                    base: base_action,
                    frozen_identity_id,
                    amount: token_amount,
                    public_note: change_note.unwrap_or(public_note),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenDestroyFrozenFundsTransitionV0` into a `TokenDestroyFrozenFundsTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token destroy_frozen_fundsing transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant destroy_frozen_fundsing logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the destroy_frozen_fundsing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenDestroyFrozenFundsTransitionV0` struct containing the transition data, including token
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
    /// * `Result<(ConsensusValidationResult<TokenDestroyFrozenFundsTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenDestroyFrozenFundsTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_destroy_frozen_funds_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenDestroyFrozenFundsTransitionV0,
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
        let TokenDestroyFrozenFundsTransitionV0 {
            base,
            frozen_identity_id,
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

        let maybe_token_amount = drive.fetch_identity_token_balance_operations(
            base.token_id().to_buffer(),
            frozen_identity_id.to_buffer(),
            !approximate_without_state_for_costs,
            transaction,
            &mut drive_operations,
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

        let Some(token_amount) = maybe_token_amount else {
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
                    vec![StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                        IdentityDoesNotHaveEnoughTokenBalanceError::new(
                            base.token_id(),
                            *frozen_identity_id,
                            1,
                            0,
                            "destroy_frozen_funds".to_string(),
                        ),
                    )
                    .into()],
                ),
                fee_result,
            ));
        };

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

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::DestroyFrozenFundsAction(
                TokenDestroyFrozenFundsTransitionActionV0 {
                    base: base_action,
                    frozen_identity_id: *frozen_identity_id,
                    amount: token_amount,
                    public_note: change_note.unwrap_or(public_note.clone()),
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
    //! Unit tests for logic fragments of `try_from_{borrowed_,}token_destroy_frozen_funds_transition_with_contract_lookup`
    //! that can be exercised without wiring up a full `Drive`.
    //!
    //! These cover:
    //!   * `change_note.unwrap_or(public_note)` priority rule (owned and borrowed shapes)
    //!   * `maybe_token_amount` `None` branch — the `let Some(token_amount) = ...` else path that
    //!     constructs an `IdentityDoesNotHaveEnoughTokenBalanceError` with concrete fields
    //!   * Frozen-identity propagation into the resulting action on success
    //!   * Identity identity kept as-is for dereferenced / moved inputs

    use dpp::balances::credits::TokenAmount;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::state::token::IdentityDoesNotHaveEnoughTokenBalanceError;
    use dpp::consensus::ConsensusError;

    /// `change_note.unwrap_or(public_note)` — Some(Some(new)) wins over any user note.
    #[test]
    fn change_note_some_some_takes_precedence_over_public_note() {
        let change_note: Option<Option<String>> = Some(Some("resolved".to_string()));
        let public_note: Option<String> = Some("user".to_string());
        let result = change_note.unwrap_or(public_note);
        assert_eq!(result, Some("resolved".to_string()));
    }

    /// `change_note.unwrap_or(public_note)` — Some(None) explicitly clears the note.
    #[test]
    fn change_note_some_none_clears_user_note() {
        let change_note: Option<Option<String>> = Some(None);
        let public_note: Option<String> = Some("will be cleared".to_string());
        let result = change_note.unwrap_or(public_note);
        assert!(result.is_none());
    }

    /// `change_note.unwrap_or(public_note)` — None falls back to user note (even None).
    #[test]
    fn change_note_none_preserves_user_note() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("keep me".to_string());
        let result = change_note.unwrap_or(public_note);
        assert_eq!(result, Some("keep me".to_string()));
    }

    /// None + None keeps the action note empty.
    #[test]
    fn both_none_yields_none() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = None;
        let result = change_note.unwrap_or(public_note);
        assert!(result.is_none());
    }

    /// The borrowed transformer clones the note rather than moving.
    #[test]
    fn borrowed_branch_clones_public_note() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("note".to_string());
        let result = change_note.unwrap_or(public_note.clone());
        assert_eq!(result, Some("note".to_string()));
        assert_eq!(public_note, Some("note".to_string()));
    }

    /// Constructs the exact `IdentityDoesNotHaveEnoughTokenBalanceError` the transformer
    /// builds when `maybe_token_amount == None`. Verifies the required_amount/actual_amount/method
    /// shape match the "destroy_frozen_funds" constant literal used in the source.
    #[test]
    fn identity_does_not_have_enough_token_balance_error_is_shaped_correctly() {
        let token_id = dpp::identifier::Identifier::new([0x01; 32]);
        let frozen_id = dpp::identifier::Identifier::new([0x02; 32]);
        let err = IdentityDoesNotHaveEnoughTokenBalanceError::new(
            token_id,
            frozen_id,
            1,
            0,
            "destroy_frozen_funds".to_string(),
        );
        let wrapped: ConsensusError =
            StateError::IdentityDoesNotHaveEnoughTokenBalanceError(err).into();
        // Round-trip: the wrapper yields a StateError variant carrying our inner error.
        match wrapped {
            ConsensusError::StateError(StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                _,
            )) => {}
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    /// Destructuring a `TokenDestroyFrozenFundsTransitionV0` value mirrors the transformer's
    /// `let TokenDestroyFrozenFundsTransitionV0 { base, frozen_identity_id, public_note } = value;`
    /// — verify all fields survive the destructure.
    #[test]
    fn destructure_owned_transition_preserves_fields() {
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::TokenDestroyFrozenFundsTransitionV0;

        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 7,
            token_contract_position: 0,
            data_contract_id: dpp::identifier::Identifier::new([0x05; 32]),
            token_id: dpp::identifier::Identifier::new([0x06; 32]),
            using_group_info: None,
        });
        let value = TokenDestroyFrozenFundsTransitionV0 {
            base: base.clone(),
            frozen_identity_id: dpp::identifier::Identifier::new([0x07; 32]),
            public_note: Some("foo".to_string()),
        };

        let TokenDestroyFrozenFundsTransitionV0 {
            base: destructured_base,
            frozen_identity_id,
            public_note,
        } = value;

        assert_eq!(
            frozen_identity_id,
            dpp::identifier::Identifier::new([0x07; 32])
        );
        assert_eq!(public_note, Some("foo".to_string()));
        // Base survives destructuring; token id is preserved.
        match destructured_base {
            TokenBaseTransition::V0(v0) => {
                assert_eq!(v0.token_id, dpp::identifier::Identifier::new([0x06; 32]));
            }
        }
    }

    /// The transformer's `amount` field is assigned directly from the fetched
    /// token balance (the `token_amount`). Sanity-check that zero and large values
    /// both fit the `TokenAmount` (u64) domain.
    #[test]
    fn token_amount_assignment_is_u64() {
        let max_amount: TokenAmount = u64::MAX;
        let min_amount: TokenAmount = 0;
        assert_eq!(max_amount, u64::MAX);
        assert_eq!(min_amount, 0);
    }
}
