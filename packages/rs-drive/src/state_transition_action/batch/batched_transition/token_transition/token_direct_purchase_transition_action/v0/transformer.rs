use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::errors::consensus::ConsensusError;
use dpp::errors::consensus::state::state_error::StateError;
use dpp::errors::consensus::state::token::{TokenAmountUnderMinimumSaleAmount, TokenDirectPurchaseUserPriceTooLow, TokenNotForDirectSale};
use dpp::identifier::Identifier;
use dpp::state_transition::state_transitions::document::batch_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::v0::TokenDirectPurchaseTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::state_transitions::document::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::BatchedTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

impl TokenDirectPurchaseTransitionActionV0 {
    /// Converts a borrowed `TokenDirectPurchaseTransitionV0` into a `TokenDirectPurchaseTransitionActionV0` using the provided contract lookup.
    ///
    /// This method processes the token direct_purchaseing transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant direct_purchaseing logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the direct_purchaseing transition. This is typically the identity
    ///   performing the transaction, such as the user's ID.
    /// * `value` - A reference to the `TokenDirectPurchaseTransitionV0` struct containing the transition data, including token
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
    /// * `Result<(ConsensusValidationResult<TokenDirectPurchaseTransitionActionV0>, FeeResult), Error>` - Returns a tuple containing the constructed
    ///   `TokenDirectPurchaseTransitionActionV0` and a `FeeResult` if successful. If an error occurs (e.g., missing data or
    ///   invalid state transition), it returns an `Error`.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_direct_purchase_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenDirectPurchaseTransitionV0,
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
        let TokenDirectPurchaseTransitionV0 {
            base,
            token_count,
            total_agreed_price,
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

        // We can not change the note on a direct purchase
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

        // We need to make sure the amount we want to pay is the amount we are expected to pay
        let (pricing_schedule, fetch_token_direct_purchase_fee) = drive
            .fetch_token_direct_purchase_price_with_costs(
                base.token_id().to_buffer(),
                block_info,
                true,
                transaction,
                platform_version,
            )?;

        fee_result.checked_add_assign(fetch_token_direct_purchase_fee)?;

        let Some(pricing_schedule) = pricing_schedule else {
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
                        StateError::TokenNotForDirectSale(TokenNotForDirectSale::new(
                            base.token_id(),
                        )),
                    )],
                ),
                fee_result,
            ));
        };

        let required_price = match pricing_schedule {
            TokenPricingSchedule::SinglePrice(price_per_token) => {
                // We've already checked the user set price in structure validation
                // Hence we can do a saturating mul.
                let required_price = price_per_token.saturating_mul(*token_count);
                if *total_agreed_price < required_price {
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
                                StateError::TokenDirectPurchaseUserPriceTooLow(
                                    TokenDirectPurchaseUserPriceTooLow::new(
                                        base.token_id(),
                                        *total_agreed_price,
                                        required_price,
                                    ),
                                ),
                            )],
                        ),
                        fee_result,
                    ));
                }
                required_price
            }
            TokenPricingSchedule::SetPrices(set_prices) => {
                match set_prices.range(..=token_count).next_back() {
                    Some((_matched_quantity, matched_price)) => {
                        // Use matched_quantity and matched_price to compute required cost
                        let required_total = *matched_price * token_count;

                        if *total_agreed_price < required_total {
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
                                        StateError::TokenDirectPurchaseUserPriceTooLow(
                                            TokenDirectPurchaseUserPriceTooLow::new(
                                                base.token_id(),
                                                *total_agreed_price,
                                                required_total,
                                            ),
                                        ),
                                    )],
                                ),
                                fee_result,
                            ));
                        }
                        required_total
                    }
                    None => {
                        // Token count is below all defined thresholds — this is an invalid purchase
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
                                    StateError::TokenAmountUnderMinimumSaleAmount(
                                        TokenAmountUnderMinimumSaleAmount::new(
                                            base.token_id(),
                                            *token_count,
                                            *set_prices.keys().next().expect("Map is not empty"),
                                        ),
                                    ),
                                )],
                            ),
                            fee_result,
                        ));
                    }
                }
            }
        };

        Ok((
            BatchedTransitionAction::TokenAction(TokenTransitionAction::DirectPurchaseAction(
                TokenDirectPurchaseTransitionActionV0 {
                    base: base_action,
                    token_count: *token_count,
                    total_agreed_price: required_price,
                }
                .into(),
            ))
            .into(),
            fee_result,
        ))
    }
}
