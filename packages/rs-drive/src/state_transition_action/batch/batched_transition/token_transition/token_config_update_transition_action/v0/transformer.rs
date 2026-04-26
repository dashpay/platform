use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_config_update_transition::v0::TokenConfigUpdateTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::v0::TokenConfigUpdateTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenConfigUpdateTransitionActionV0 {
    /// Converts a `TokenConfigUpdateTransitionV0` into a `TokenConfigUpdateTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token config update transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for config update.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the config update transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenConfigUpdateTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `block_info` - Information about the current block to calculate fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<TokenConfigUpdateTransitionActionV0>, Error>` - Returns the constructed `TokenConfigUpdateTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_config_update_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenConfigUpdateTransitionV0,
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
        let TokenConfigUpdateTransitionV0 {
            base,

            update_token_configuration_item,
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
            BatchedTransitionAction::TokenAction(TokenTransitionAction::ConfigUpdateAction(
                TokenConfigUpdateTransitionActionV0 {
                    base: base_action,
                    update_token_configuration_item,
                    public_note: change_note.unwrap_or(public_note),
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenConfigUpdateTransitionV0` into a `TokenConfigUpdateTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token config update transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant config update logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the config update transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenConfigUpdateTransitionV0` struct containing the transition data, including token
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
    /// * `Result<(ConsensusValidationResult<TokenConfigUpdateTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenConfigUpdateTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_config_update_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenConfigUpdateTransitionV0,
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
        let TokenConfigUpdateTransitionV0 {
            base,
            update_token_configuration_item,
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
            BatchedTransitionAction::TokenAction(TokenTransitionAction::ConfigUpdateAction(
                TokenConfigUpdateTransitionActionV0 {
                    base: base_action,
                    update_token_configuration_item: update_token_configuration_item.clone(),
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
    //! Unit tests for the logic fragments of
    //! `try_from_{borrowed_,}token_config_update_transition_with_contract_lookup` that
    //! can be exercised without a full `Drive`.
    //!
    //! These cover:
    //!   * `change_note.unwrap_or(public_note)` priority rule (owned + borrowed)
    //!   * cloning semantics of `TokenConfigurationChangeItem`
    //!   * every `TokenConfigurationChangeItem` variant survives a clone round-trip
    //!   * destructuring of `TokenConfigUpdateTransitionV0` preserves all fields

    use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;

    #[test]
    fn config_update_change_note_some_some_wins() {
        let change_note: Option<Option<String>> = Some(Some("resolved".to_string()));
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert_eq!(merged, Some("resolved".to_string()));
    }

    #[test]
    fn config_update_change_note_some_none_clears_user_note() {
        let change_note: Option<Option<String>> = Some(None);
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert!(merged.is_none());
    }

    #[test]
    fn config_update_change_note_none_falls_back_to_user_note() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("user".to_string());
        let merged = change_note.unwrap_or(public_note);
        assert_eq!(merged, Some("user".to_string()));
    }

    /// The borrowed transformer uses `change_note.unwrap_or(public_note.clone())`
    /// to avoid moving from a borrowed reference. Verify the clone round-trips.
    #[test]
    fn config_update_borrowed_note_clone_round_trip() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("survivor".to_string());
        let merged = change_note.unwrap_or(public_note.clone());
        assert_eq!(merged, Some("survivor".to_string()));
        // Still available for reuse.
        assert_eq!(public_note, Some("survivor".to_string()));
    }

    /// The borrowed transformer writes
    /// `update_token_configuration_item: update_token_configuration_item.clone()`
    /// so the item must round-trip cleanly through Clone for the variants the
    /// transformer may encounter.
    #[test]
    fn no_change_variant_clone_round_trip() {
        let orig = TokenConfigurationChangeItem::TokenConfigurationNoChange;
        let cloned = orig.clone();
        assert!(matches!(
            cloned,
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ));
    }

    #[test]
    fn max_supply_none_variant_clone_round_trip() {
        let orig = TokenConfigurationChangeItem::MaxSupply(None);
        let cloned = orig.clone();
        assert!(matches!(
            cloned,
            TokenConfigurationChangeItem::MaxSupply(None)
        ));
    }

    #[test]
    fn max_supply_some_variant_clone_round_trip() {
        let orig = TokenConfigurationChangeItem::MaxSupply(Some(10_000));
        let cloned = orig.clone();
        match cloned {
            TokenConfigurationChangeItem::MaxSupply(Some(v)) => assert_eq!(v, 10_000),
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn manual_minting_variant_clone_round_trip() {
        let orig = TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::NoOne);
        let cloned = orig.clone();
        match cloned {
            TokenConfigurationChangeItem::ManualMinting(AuthorizedActionTakers::NoOne) => {}
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn manual_burning_variant_clone_round_trip() {
        let orig =
            TokenConfigurationChangeItem::ManualBurning(AuthorizedActionTakers::ContractOwner);
        let cloned = orig.clone();
        match cloned {
            TokenConfigurationChangeItem::ManualBurning(AuthorizedActionTakers::ContractOwner) => {}
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn freeze_variant_clone_round_trip() {
        let orig = TokenConfigurationChangeItem::Freeze(AuthorizedActionTakers::MainGroup);
        let cloned = orig.clone();
        match cloned {
            TokenConfigurationChangeItem::Freeze(AuthorizedActionTakers::MainGroup) => {}
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn new_tokens_destination_identity_variant_clone_round_trip() {
        let id = dpp::identifier::Identifier::new([0x88; 32]);
        let orig = TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(id));
        let cloned = orig.clone();
        match cloned {
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(Some(got)) => {
                assert_eq!(got, id);
            }
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    #[test]
    fn main_control_group_none_variant_clone_round_trip() {
        let orig = TokenConfigurationChangeItem::MainControlGroup(None);
        let cloned = orig.clone();
        assert!(matches!(
            cloned,
            TokenConfigurationChangeItem::MainControlGroup(None)
        ));
    }

    #[test]
    fn minting_allow_choosing_destination_variant_clone_round_trip() {
        let orig = TokenConfigurationChangeItem::MintingAllowChoosingDestination(true);
        let cloned = orig.clone();
        match cloned {
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(b) => assert!(b),
            other => panic!("unexpected variant: {:?}", other),
        }
    }

    /// Destructuring mirrors the transformer's `let TokenConfigUpdateTransitionV0 { base, update_token_configuration_item, public_note } = value;`
    /// pattern. Ensure all fields survive.
    #[test]
    fn destructure_owned_transition_preserves_fields() {
        use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
        use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
        use dpp::state_transition::batch_transition::token_config_update_transition::v0::TokenConfigUpdateTransitionV0;

        let base = TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 99,
            token_contract_position: 0,
            data_contract_id: dpp::identifier::Identifier::new([0x01; 32]),
            token_id: dpp::identifier::Identifier::new([0x02; 32]),
            using_group_info: None,
        });

        let v0 = TokenConfigUpdateTransitionV0 {
            base,
            update_token_configuration_item: TokenConfigurationChangeItem::MaxSupply(Some(777)),
            public_note: Some("note".to_string()),
        };

        let TokenConfigUpdateTransitionV0 {
            base,
            update_token_configuration_item,
            public_note,
        } = v0;

        match base {
            TokenBaseTransition::V0(v) => {
                assert_eq!(v.identity_contract_nonce, 99);
            }
        }
        match update_token_configuration_item {
            TokenConfigurationChangeItem::MaxSupply(Some(v)) => assert_eq!(v, 777),
            other => panic!("unexpected item variant: {:?}", other),
        }
        assert_eq!(public_note, Some("note".to_string()));
    }
}
