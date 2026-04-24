use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_emergency_action_transition::v0::TokenEmergencyActionTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::v0::TokenEmergencyActionTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenEmergencyActionTransitionActionV0 {
    /// Converts a `TokenEmergencyActionTransitionV0` into a `TokenEmergencyActionTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token emergency_actioning transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for emergency_actioning.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the emergency_actioning transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenEmergencyActionTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `block_info` - Information about the current block to calculate fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<TokenEmergencyActionTransitionActionV0>, Error>` - Returns the constructed `TokenEmergencyActionTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_emergency_action_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenEmergencyActionTransitionV0,
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
        let TokenEmergencyActionTransitionV0 {
            base,
            emergency_action,
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
            BatchedTransitionAction::TokenAction(TokenTransitionAction::EmergencyActionAction(
                TokenEmergencyActionTransitionActionV0 {
                    base: base_action,
                    emergency_action,
                    public_note: change_note.unwrap_or(public_note),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenEmergencyActionTransitionV0` into a `TokenEmergencyActionTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token emergency_actioning transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant emergency_actioning logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the emergency_actioning transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenEmergencyActionTransitionV0` struct containing the transition data, including token
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
    /// * `Result<(ConsensusValidationResult<TokenEmergencyActionTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenEmergencyActionTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_emergency_action_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenEmergencyActionTransitionV0,
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
        let TokenEmergencyActionTransitionV0 {
            base,
            emergency_action,
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
            BatchedTransitionAction::TokenAction(TokenTransitionAction::EmergencyActionAction(
                TokenEmergencyActionTransitionActionV0 {
                    base: base_action,
                    emergency_action: *emergency_action,
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
    use crate::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::{
        TokenEmergencyActionTransitionAction, TokenEmergencyActionTransitionActionAccessorsV0,
        TokenEmergencyActionTransitionActionV0,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use platform_version::version::PlatformVersion;
    use std::sync::Arc;

    fn make_base() -> TokenBaseTransitionAction {
        let fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::new([9u8; 32]),
            identity_contract_nonce: 7,
            token_contract_position: 0,
            data_contract: Arc::new(fetch_info),
            store_in_group: None,
            perform_action: true,
        })
    }

    fn make_action_v0(
        action: TokenEmergencyAction,
        note: Option<&str>,
    ) -> TokenEmergencyActionTransitionActionV0 {
        TokenEmergencyActionTransitionActionV0 {
            base: make_base(),
            emergency_action: action,
            public_note: note.map(|s| s.to_string()),
        }
    }

    #[test]
    fn v0_accessors_return_emergency_action_and_note() {
        let v0 = make_action_v0(TokenEmergencyAction::Pause, Some("pause-note"));
        assert_eq!(v0.emergency_action(), TokenEmergencyAction::Pause);
        assert_eq!(v0.public_note(), Some(&"pause-note".to_string()));
    }

    #[test]
    fn v0_set_emergency_action_swaps_variant() {
        let mut v0 = make_action_v0(TokenEmergencyAction::Pause, None);
        assert!(v0.emergency_action().paused());
        v0.set_emergency_action(TokenEmergencyAction::Resume);
        assert_eq!(v0.emergency_action(), TokenEmergencyAction::Resume);
        assert!(!v0.emergency_action().paused());
    }

    #[test]
    fn v0_set_public_note_updates_note() {
        let mut v0 = make_action_v0(TokenEmergencyAction::Resume, None);
        assert!(v0.public_note().is_none());
        v0.set_public_note(Some("hi".to_string()));
        assert_eq!(v0.public_note(), Some(&"hi".to_string()));
        v0.set_public_note(None);
        assert!(v0.public_note().is_none());
    }

    #[test]
    fn v0_public_note_owned_consumes_self() {
        let v0 = make_action_v0(TokenEmergencyAction::Pause, Some("owned"));
        let owned: Option<String> = v0.public_note_owned();
        assert_eq!(owned, Some("owned".to_string()));
    }

    #[test]
    fn v0_base_ref_and_base_owned_preserve_token_id() {
        let v0 = make_action_v0(TokenEmergencyAction::Pause, None);
        assert_eq!(v0.base().token_id(), Identifier::new([9u8; 32]));
        let base = v0.base_owned();
        assert_eq!(base.token_id(), Identifier::new([9u8; 32]));
    }

    #[test]
    fn v0_default_accessors_delegate_to_base() {
        let v0 = make_action_v0(TokenEmergencyAction::Pause, None);
        assert_eq!(v0.token_id(), Identifier::new([9u8; 32]));
        assert_eq!(v0.token_position(), 0);
        // data_contract_id comes from the dpns contract fixture
        let fetched = v0.data_contract_fetch_info();
        assert_eq!(v0.data_contract_id(), fetched.contract.id());
        // ref and owned return same id
        assert_eq!(
            v0.data_contract_fetch_info_ref().contract.id(),
            fetched.contract.id()
        );
    }

    #[test]
    fn enum_from_v0_produces_v0_variant() {
        let v0 = make_action_v0(TokenEmergencyAction::Resume, Some("n"));
        let action: TokenEmergencyActionTransitionAction = v0.into();
        assert_eq!(action.emergency_action(), TokenEmergencyAction::Resume);
        assert_eq!(action.public_note(), Some(&"n".to_string()));
    }

    #[test]
    fn enum_accessors_route_through_v0() {
        let v0 = make_action_v0(TokenEmergencyAction::Pause, Some("start"));
        let mut action: TokenEmergencyActionTransitionAction = v0.into();
        assert_eq!(action.emergency_action(), TokenEmergencyAction::Pause);

        action.set_emergency_action(TokenEmergencyAction::Resume);
        assert_eq!(action.emergency_action(), TokenEmergencyAction::Resume);

        action.set_public_note(Some("replaced".to_string()));
        assert_eq!(action.public_note(), Some(&"replaced".to_string()));

        let base = action.clone().base_owned();
        assert_eq!(base.token_id(), Identifier::new([9u8; 32]));

        assert_eq!(action.base().token_id(), Identifier::new([9u8; 32]));

        let note = action.public_note_owned();
        assert_eq!(note, Some("replaced".to_string()));
    }

    #[test]
    fn enum_set_public_note_none_clears() {
        let mut action: TokenEmergencyActionTransitionAction =
            make_action_v0(TokenEmergencyAction::Pause, Some("x")).into();
        action.set_public_note(None);
        assert!(action.public_note().is_none());
    }

    #[test]
    fn emergency_action_paused_helper_matches_variant() {
        // Sanity check the upstream TokenEmergencyAction helper used by this module.
        assert!(TokenEmergencyAction::Pause.paused());
        assert!(!TokenEmergencyAction::Resume.paused());
    }

    // -------------------------------------------------------------------
    // Transformer logic fragment tests — note precedence and Copy semantics of
    // TokenEmergencyAction (exercised by the `emergency_action: *emergency_action`
    // pattern in the borrowed transformer).
    // -------------------------------------------------------------------

    #[test]
    fn emergency_change_note_some_some_wins() {
        let change_note: Option<Option<String>> = Some(Some("group-override".to_string()));
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert_eq!(merged, Some("group-override".to_string()));
    }

    #[test]
    fn emergency_change_note_some_none_clears_user_note() {
        let change_note: Option<Option<String>> = Some(None);
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert!(merged.is_none());
    }

    #[test]
    fn emergency_change_note_none_keeps_user_note() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert_eq!(merged, Some("user".to_string()));
    }

    #[test]
    fn emergency_borrowed_copies_action_via_star_deref() {
        // The borrowed transformer writes `emergency_action: *emergency_action`
        // which relies on `TokenEmergencyAction: Copy`. We mirror that via an
        // intermediate reference binding (a direct `*&pause` would trip
        // `clippy::deref_addrof`).
        let pause = TokenEmergencyAction::Pause;
        let pause_ref: &TokenEmergencyAction = &pause;
        let copied: TokenEmergencyAction = *pause_ref;
        assert_eq!(copied, TokenEmergencyAction::Pause);

        let resume = TokenEmergencyAction::Resume;
        let resume_ref: &TokenEmergencyAction = &resume;
        let copied: TokenEmergencyAction = *resume_ref;
        assert_eq!(copied, TokenEmergencyAction::Resume);
    }

    #[test]
    fn emergency_borrowed_path_clones_public_note_without_consuming() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("bound".to_string());
        let merged = change_note.unwrap_or(public_note.clone());
        assert_eq!(merged, Some("bound".to_string()));
        assert!(public_note.is_some());
    }
}
