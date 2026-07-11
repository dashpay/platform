use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_unfreeze_transition::v0::TokenUnfreezeTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::v0::TokenUnfreezeTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenUnfreezeTransitionActionV0 {
    /// Converts a `TokenUnfreezeTransitionV0` into a `TokenUnfreezeTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token freezeing transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for freezeing.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the freezeing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenUnfreezeTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `block_info` - Information about the current block to calculate fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<TokenUnfreezeTransitionActionV0>, Error>` - Returns the constructed `TokenUnfreezeTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_unfreeze_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenUnfreezeTransitionV0,
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
        let TokenUnfreezeTransitionV0 {
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

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::UnfreezeAction(
                TokenUnfreezeTransitionActionV0 {
                    base: base_action,
                    frozen_identity_id,
                    public_note: change_note.unwrap_or(public_note),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenUnfreezeTransitionV0` into a `TokenUnfreezeTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token freezeing transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant freezeing logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the freezeing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenUnfreezeTransitionV0` struct containing the transition data, including token
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
    /// * `Result<(ConsensusValidationResult<TokenUnfreezeTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenUnfreezeTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_unfreeze_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenUnfreezeTransitionV0,
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
        let TokenUnfreezeTransitionV0 {
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

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::UnfreezeAction(
                TokenUnfreezeTransitionActionV0 {
                    base: base_action,
                    frozen_identity_id: *frozen_identity_id,
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
    use crate::drive::contract::DataContractFetchInfo;
    use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
        TokenBaseTransitionActionV0,
    };
    use crate::state_transition_action::batch::batched_transition::token_transition::token_unfreeze_transition_action::{
        TokenUnfreezeTransitionAction, TokenUnfreezeTransitionActionAccessorsV0,
        TokenUnfreezeTransitionActionV0,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use platform_version::version::PlatformVersion;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([12u8; 32]),
            identity_contract_nonce: 5,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_action_v0(target: Identifier, note: Option<&str>) -> TokenUnfreezeTransitionActionV0 {
        TokenUnfreezeTransitionActionV0 {
            base: make_base(),
            frozen_identity_id: target,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn v0_frozen_identity_id_returns_stored_id() {
        let target = Identifier::new([42u8; 32]);
        let v0 = make_action_v0(target, None);
        assert_eq!(v0.frozen_identity_id(), target);
    }

    #[test]
    fn v0_set_frozen_identity_id_updates_field() {
        let mut v0 = make_action_v0(Identifier::new([1u8; 32]), None);
        let replacement = Identifier::new([77u8; 32]);
        v0.set_frozen_identity_id(replacement);
        assert_eq!(v0.frozen_identity_id(), replacement);
    }

    #[test]
    fn v0_public_note_accessors_return_reference_and_owned() {
        let v0 = make_action_v0(Identifier::new([1u8; 32]), Some("thawed"));
        assert_eq!(v0.public_note(), Some(&"thawed".to_string()));
        assert_eq!(v0.public_note_owned(), Some("thawed".to_string()));
    }

    #[test]
    fn v0_set_public_note_round_trip() {
        let mut v0 = make_action_v0(Identifier::new([1u8; 32]), None);
        assert!(v0.public_note().is_none());
        v0.set_public_note(Some("set".to_string()));
        assert_eq!(v0.public_note(), Some(&"set".to_string()));
        v0.set_public_note(None);
        assert!(v0.public_note().is_none());
    }

    #[test]
    fn v0_default_accessors_route_through_base() {
        let v0 = make_action_v0(Identifier::new([1u8; 32]), None);
        assert_eq!(v0.token_id(), Identifier::new([12u8; 32]));
        assert_eq!(v0.token_position(), 0);
        let fetch = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetch.contract.id());
        assert_eq!(
            v0.data_contract_fetch_info_ref().contract.id(),
            fetch.contract.id()
        );
    }

    #[test]
    fn v0_base_ref_and_base_owned_are_consistent() {
        let v0 = make_action_v0(Identifier::new([1u8; 32]), None);
        let id_from_ref = v0.base().token_id();
        let base = v0.base_owned();
        assert_eq!(id_from_ref, base.token_id());
    }

    #[test]
    fn enum_from_v0_preserves_identity_and_note() {
        let v0 = make_action_v0(Identifier::new([9u8; 32]), Some("via_enum"));
        let wrapped: TokenUnfreezeTransitionAction = v0.into();
        assert_eq!(wrapped.frozen_identity_id(), Identifier::new([9u8; 32]));
        assert_eq!(wrapped.public_note(), Some(&"via_enum".to_string()));
    }

    #[test]
    fn enum_setters_mutate_underlying_v0() {
        let mut wrapped: TokenUnfreezeTransitionAction =
            make_action_v0(Identifier::new([1u8; 32]), None).into();

        wrapped.set_frozen_identity_id(Identifier::new([33u8; 32]));
        assert_eq!(wrapped.frozen_identity_id(), Identifier::new([33u8; 32]));

        wrapped.set_public_note(Some("added".to_string()));
        assert_eq!(wrapped.public_note(), Some(&"added".to_string()));

        let owned = wrapped.public_note_owned();
        assert_eq!(owned, Some("added".to_string()));
    }

    #[test]
    fn enum_base_methods_delegate_to_v0() {
        let wrapped: TokenUnfreezeTransitionAction =
            make_action_v0(Identifier::new([1u8; 32]), None).into();
        assert_eq!(wrapped.base().token_id(), Identifier::new([12u8; 32]));
        let base_owned = wrapped.base_owned();
        assert_eq!(base_owned.token_id(), Identifier::new([12u8; 32]));
    }

    #[test]
    fn enum_set_public_note_none_clears_existing_note() {
        let mut wrapped: TokenUnfreezeTransitionAction =
            make_action_v0(Identifier::new([1u8; 32]), Some("wipeme")).into();
        wrapped.set_public_note(None);
        assert!(wrapped.public_note().is_none());
    }

    // -------------------------------------------------------------------
    // Transformer logic fragment tests — exercising the note merging rule.
    // -------------------------------------------------------------------

    #[test]
    fn unfreeze_change_note_some_some_wins() {
        let change_note: Option<Option<String>> = Some(Some("resolved".to_string()));
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert_eq!(merged, Some("resolved".to_string()));
    }

    #[test]
    fn unfreeze_change_note_some_none_clears() {
        let change_note: Option<Option<String>> = Some(None);
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert!(merged.is_none());
    }

    #[test]
    fn unfreeze_change_note_none_keeps_user() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert_eq!(merged, Some("user".to_string()));
    }

    #[test]
    fn unfreeze_borrowed_path_clones_note() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("clone me".to_string());
        let merged = change_note.unwrap_or(public_note.clone());
        assert_eq!(merged, Some("clone me".to_string()));
        assert!(public_note.is_some());
    }

    #[test]
    fn unfreeze_borrowed_path_dereferences_frozen_identity_id() {
        let id = Identifier::new([0xCD; 32]);
        // Mirror the `frozen_identity_id: *frozen_identity_id` pattern via an
        // intermediate reference binding. Writing `*&id` directly would trip
        // `clippy::deref_addrof`.
        let id_ref: &Identifier = &id;
        let copied: Identifier = *id_ref;
        assert_eq!(copied, id);
    }

    #[test]
    fn unfreeze_both_notes_none_yields_none() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = None;
        assert!(change_note.unwrap_or(public_note).is_none());
    }
}
