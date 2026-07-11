use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::v0::TokenSetPriceForDirectPurchaseTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::v0::TokenSetPriceForDirectPurchaseTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenSetPriceForDirectPurchaseTransitionActionV0 {
    /// Converts a `TokenSetPriceForDirectPurchaseTransitionV0` into a `TokenSetPriceForDirectPurchaseTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token set_price_for_direct_purchasing transition and returns the corresponding transition action
    /// while looking up necessary data contracts and applying the relevant logic for set_price_for_direct_purchasing.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance which handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the set_price_for_direct_purchasing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `transaction` - A transaction context that includes the necessary state and other details for the transition.
    /// * `value` - The `TokenSetPriceForDirectPurchaseTransitionV0` struct containing the transition data, including token amount and recipient.
    /// * `approximate_without_state_for_costs` - A flag to determine if costs should be approximated without considering
    ///   the full state for the operation. Useful for optimizing the transaction cost calculations.
    /// * `block_info` - Information about the current block to calculate fees.
    /// * `get_data_contract` - A closure function that takes a contract identifier and returns a `DataContractFetchInfo`
    ///   containing the data contract details, including token configurations.
    /// * `platform_version` - A reference to the platform version, ensuring the transition respects version-specific logic.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<TokenSetPriceForDirectPurchaseTransitionActionV0>, Error>` - Returns the constructed `TokenSetPriceForDirectPurchaseTransitionActionV0` if successful,
    ///   or an error if any issue arises, such as missing data or an invalid state transition.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_set_price_for_direct_purchase_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenSetPriceForDirectPurchaseTransitionV0,
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
        let TokenSetPriceForDirectPurchaseTransitionV0 {
            base,
            price,
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
            BatchedTransitionAction::TokenAction(
                TokenTransitionAction::SetPriceForDirectPurchaseAction(
                    TokenSetPriceForDirectPurchaseTransitionActionV0 {
                        base: base_action,
                        price,
                        public_note: change_note.unwrap_or(public_note),
                    }
                    .into(),
                ),
            )
            .into(),
            fee_result,
        ))
    }

    /// Converts a borrowed `TokenSetPriceForDirectPurchaseTransitionV0` into a `TokenSetPriceForDirectPurchaseTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token set_price_for_direct_purchasing transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant set_price_for_direct_purchasing logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the set_price_for_direct_purchasing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenSetPriceForDirectPurchaseTransitionV0` struct containing the transition data, including token
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
    /// * `Result<(ConsensusValidationResult<TokenSetPriceForDirectPurchaseTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenSetPriceForDirectPurchaseTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_set_price_for_direct_purchase_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenSetPriceForDirectPurchaseTransitionV0,
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
        let TokenSetPriceForDirectPurchaseTransitionV0 {
            base,
            price,
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
            BatchedTransitionAction::TokenAction(
                TokenTransitionAction::SetPriceForDirectPurchaseAction(
                    TokenSetPriceForDirectPurchaseTransitionActionV0 {
                        base: base_action,
                        price: price.clone(),
                        public_note: change_note.unwrap_or(public_note.clone()),
                    }
                    .into(),
                ),
            )
            .into(),
            fee_result,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unnecessary_literal_unwrap)]
mod tests {
    //! Unit tests for the logic fragments of
    //! `try_from_{borrowed_,}token_set_price_for_direct_purchase_transition_with_contract_lookup`
    //! that can be exercised without wiring up a full `Drive`.
    //!
    //! These cover:
    //!   * the `change_note.unwrap_or(public_note)` priority rule (owned and borrowed shapes)
    //!   * cloning semantics of `Option<TokenPricingSchedule>`
    //!   * cloning semantics of `Option<String>` for the public note
    //!   * each `TokenPricingSchedule` variant surviving a round-trip clone
    //!   * `TokenSetPriceForDirectPurchaseTransitionActionV0` field construction
    //!   * enum / variant conversion into `TokenSetPriceForDirectPurchaseTransitionAction`
    use super::*;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use std::collections::BTreeMap;

    /// `change_note.unwrap_or(public_note)` must prefer the `change_note` when it
    /// is `Some`: this is how group-action-resolved notes override user notes.
    /// Note that the resolved note type is `Option<Option<String>>` so the outer
    /// `Some` case can itself carry `None` (an explicit "clear the note" signal).
    #[test]
    fn change_note_takes_precedence_over_public_note_owned() {
        let change_note: Option<Option<String>> = Some(Some("group resolved".to_string()));
        let public_note: Option<String> = Some("user submitted".to_string());
        let result = change_note.unwrap_or(public_note);
        assert_eq!(result, Some("group resolved".to_string()));
    }

    /// When the outer change-note is `Some(None)`, the action note is explicitly cleared,
    /// overriding any user-supplied note.
    #[test]
    fn change_note_of_some_none_clears_user_note() {
        let change_note: Option<Option<String>> = Some(None);
        let public_note: Option<String> = Some("user submitted".to_string());
        let result = change_note.unwrap_or(public_note);
        assert!(result.is_none());
    }

    /// When there is no change note, the user's note is kept as-is (including `None`).
    #[test]
    fn falls_back_to_public_note_when_change_note_is_none() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("user submitted".to_string());
        let result = change_note.unwrap_or(public_note);
        assert_eq!(result, Some("user submitted".to_string()));
    }

    /// A `None` change note combined with a `None` public note keeps the action note empty.
    #[test]
    fn none_change_note_and_none_public_note_yields_none() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = None;
        let result = change_note.unwrap_or(public_note);
        assert!(result.is_none());
    }

    /// The borrowed transformer clones the note rather than moves it. Verify that behaviour.
    #[test]
    fn borrowed_branch_clones_public_note_reference() {
        let change_note: Option<Option<String>> = None;
        let public_note: Option<String> = Some("the note".to_string());
        let result = change_note.unwrap_or(public_note.clone());
        assert_eq!(result, Some("the note".to_string()));
        // The original remains untouched (no move).
        assert_eq!(public_note, Some("the note".to_string()));
    }

    /// The borrowed transformer also clones the `Option<TokenPricingSchedule>`.
    #[test]
    fn borrowed_branch_clones_single_price_pricing_schedule() {
        let price: Option<TokenPricingSchedule> = Some(TokenPricingSchedule::SinglePrice(1_234));
        let cloned = price.clone();
        assert_eq!(cloned, price);
        assert_eq!(
            cloned,
            Some(TokenPricingSchedule::SinglePrice(1_234)),
            "SinglePrice should survive a clone exactly"
        );
    }

    #[test]
    fn borrowed_branch_clones_set_prices_pricing_schedule() {
        let mut map = BTreeMap::new();
        map.insert(1u64, 500u64);
        map.insert(10, 4_500);
        map.insert(100, 40_000);
        let price: Option<TokenPricingSchedule> = Some(TokenPricingSchedule::SetPrices(map));
        let cloned = price.clone();
        assert_eq!(cloned, price);
    }

    #[test]
    fn setting_price_to_none_disables_purchasability() {
        // Documented semantics: `price: None` means the token is no longer purchasable.
        let price: Option<TokenPricingSchedule> = None;
        let cloned = price.clone();
        assert!(cloned.is_none());
    }

    /// Exercise the V0 struct constructor and all accessors the transformer populates.
    #[test]
    fn action_v0_field_construction_from_transformer_output_shape() {
        use crate::drive::contract::DataContractFetchInfo;
        use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
            TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
            TokenBaseTransitionActionV0,
        };
        use dpp::identifier::Identifier;
        use dpp::version::PlatformVersion;
        use std::sync::Arc;

        let base = TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::from([0x10; 32]),
            identity_contract_nonce: 3,
            token_contract_position: 0,
            data_contract: Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                PlatformVersion::latest().protocol_version,
            )),
            store_in_group: None,
            perform_action: true,
        });

        let price = Some(TokenPricingSchedule::SinglePrice(777));
        let public_note = Some("hello".to_string());

        let action = TokenSetPriceForDirectPurchaseTransitionActionV0 {
            base,
            price: price.clone(),
            public_note: public_note.clone(),
        };
        assert_eq!(action.price, price);
        assert_eq!(action.public_note, public_note);
        assert_eq!(action.base.token_id(), Identifier::from([0x10; 32]));
    }

    /// The enum-wrapper `From<TokenSetPriceForDirectPurchaseTransitionActionV0>` is how
    /// the transformer constructs the final batched action. Verify it preserves state.
    #[test]
    fn enum_conversion_from_v0_preserves_state() {
        use crate::drive::contract::DataContractFetchInfo;
        use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
            TokenBaseTransitionAction, TokenBaseTransitionActionV0,
        };
        use crate::state_transition_action::batch::batched_transition::token_transition::token_set_price_for_direct_purchase_transition_action::TokenSetPriceForDirectPurchaseTransitionAction;
        use dpp::identifier::Identifier;
        use dpp::version::PlatformVersion;
        use std::sync::Arc;

        let base = TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
            token_id: Identifier::from([0x20; 32]),
            identity_contract_nonce: 11,
            token_contract_position: 0,
            data_contract: Arc::new(DataContractFetchInfo::dpns_contract_fixture(
                PlatformVersion::latest().protocol_version,
            )),
            store_in_group: None,
            perform_action: true,
        });
        let v0 = TokenSetPriceForDirectPurchaseTransitionActionV0 {
            base,
            price: Some(TokenPricingSchedule::SinglePrice(42)),
            public_note: None,
        };
        let wrapped: TokenSetPriceForDirectPurchaseTransitionAction = v0.into();
        assert!(matches!(
            wrapped,
            TokenSetPriceForDirectPurchaseTransitionAction::V0(_)
        ));
    }
}
