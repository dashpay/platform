use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{TokenAmountUnderMinimumSaleAmount, TokenDirectPurchaseUserPriceTooLow, TokenNotForDirectSale};
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
use dpp::ProtocolError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::v0::TokenDirectPurchaseTransitionActionV0;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
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
    /// This method processes the token direct_purchasing transition and constructs the corresponding transition action while
    /// looking up necessary data contracts and applying the relevant direct_purchasing logic. It does not require `drive_operations`
    /// to be passed as a parameter, but it manages them internally.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance that handles data storage and retrieval.
    /// * `owner_id` - The identifier of the owner initiating the direct_purchasing transition. This is typically the identity
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use dpp::balances::credits::TokenAmount;
    use dpp::fee::Credits;

    /// Proves that the `SetPrices` pricing branch in the direct purchase transformer
    /// uses bare multiplication (`*matched_price * token_count`) which wraps on
    /// overflow in release builds, allowing an attacker to pay almost nothing.
    ///
    /// The `SinglePrice` branch correctly uses `saturating_mul`, but the `SetPrices`
    /// branch does not. This test replicates the exact arithmetic from both branches
    /// and shows the divergent behavior.
    ///
    /// Bug location: transformer.rs line 194
    ///   `let required_total = *matched_price * token_count;`
    ///
    /// In release mode, this wraps silently. For example, a price of
    /// `u64::MAX / 3 + 1` multiplied by `token_count = 3` yields `3` after
    /// wrapping, meaning the user would pay almost nothing.
    #[test]
    fn prove_set_prices_multiplication_overflow_wraps_to_small_value() {
        // Chosen so that matched_price * token_count overflows u64 and wraps
        // to a very small number.
        //
        // u64::MAX = 18_446_744_073_709_551_615
        // 2^64     = 18_446_744_073_709_551_616
        // matched_price = u64::MAX / 3 + 1 = 6_148_914_691_236_517_206
        // token_count = 3
        // True product = 6_148_914_691_236_517_206 * 3 = 18_446_744_073_709_551_618
        // Wrapped result = 18_446_744_073_709_551_618 - 2^64 = 2
        let matched_price: Credits = u64::MAX / 3 + 1; // 6_148_914_691_236_517_206
        let token_count: TokenAmount = 3;

        // ---- Replicate the BUGGY SetPrices arithmetic (line 194) ----
        // The actual code does: `let required_total = *matched_price * token_count;`
        // In release mode, this is wrapping_mul. We use wrapping_mul explicitly
        // so this test passes in both debug and release modes.
        let required_total_buggy = matched_price.wrapping_mul(token_count);

        // The wrapped result is tiny -- just 2 credits instead of ~18.4 quintillion.
        assert_eq!(
            required_total_buggy, 2,
            "Bare multiplication wraps: the attacker would only need to pay 2 credits"
        );

        // ---- Replicate the CORRECT SinglePrice arithmetic (line 161) ----
        // The actual code does: `let required_price = price_per_token.saturating_mul(*token_count);`
        let required_total_correct = matched_price.saturating_mul(token_count);

        // Saturating mul correctly caps at u64::MAX.
        assert_eq!(
            required_total_correct,
            u64::MAX,
            "saturating_mul caps at u64::MAX, preventing the attacker from paying less"
        );

        // The attacker's "savings" from the overflow:
        // They pay 3 instead of u64::MAX -- effectively free tokens.
        assert!(
            required_total_buggy < required_total_correct,
            "The buggy result ({}) is drastically less than the correct result ({})",
            required_total_buggy,
            required_total_correct,
        );
    }

    /// Additional test demonstrating the overflow with realistic SetPrices schedule
    /// parameters, using `BTreeMap::range` exactly as the transformer code does.
    #[test]
    fn prove_set_prices_overflow_via_btreemap_range_lookup() {
        // Build a SetPrices schedule with a high per-token price at the 1-token tier.
        // This mirrors how the transformer looks up pricing in a BTreeMap.
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        // Tier: buying >= 1 token costs a large price per token
        let large_price: Credits = (1u64 << 63) + 1; // 9_223_372_036_854_775_809
        set_prices.insert(1, large_price);

        // Attacker requests 2 tokens
        let token_count: TokenAmount = 2;

        // Replicate the exact BTreeMap lookup from the transformer (line 191):
        //   match set_prices.range(..=token_count).next_back()
        let matched_price = match set_prices.range(..=token_count).next_back() {
            Some((_matched_quantity, price)) => *price,
            None => panic!("Should have found a matching price tier"),
        };

        assert_eq!(matched_price, large_price);

        // Buggy path: bare multiplication wraps
        let buggy_total = matched_price.wrapping_mul(token_count);

        // True mathematical result: (2^63 + 1) * 2 = 2^64 + 2 = u64::MAX + 3
        // Wrapped: 2
        assert_eq!(
            buggy_total, 2,
            "Overflow wraps to 2: attacker pays 2 credits for 2 tokens worth ~9.2 quintillion each"
        );

        // Correct path: saturating_mul
        let correct_total = matched_price.saturating_mul(token_count);
        assert_eq!(correct_total, u64::MAX);

        // Demonstrate the attack: the user sets total_agreed_price to the
        // wrapped value and the check `total_agreed_price < required_total`
        // passes, allowing the purchase at the absurdly low price.
        let attacker_agreed_price: Credits = buggy_total; // just 2 credits
        assert!(
            attacker_agreed_price >= buggy_total,
            "Attacker's price of {} passes the check against buggy total of {}",
            attacker_agreed_price,
            buggy_total,
        );

        // But the attacker's price should NOT pass against the correct total:
        assert!(
            attacker_agreed_price < correct_total,
            "Attacker's price of {} should be rejected against correct total of {}",
            attacker_agreed_price,
            correct_total,
        );
    }

    /// Verify that SinglePrice branch (saturating_mul) is safe for the same inputs.
    #[test]
    fn single_price_saturating_mul_is_safe() {
        let price_per_token: Credits = u64::MAX / 3 + 1;
        let token_count: TokenAmount = 3;

        // SinglePrice branch uses saturating_mul (line 161)
        let required_price = price_per_token.saturating_mul(token_count);

        // Should saturate to u64::MAX, not wrap
        assert_eq!(required_price, u64::MAX);

        // Any attacker offering less than u64::MAX would be rejected
        let attacker_price: Credits = 1_000_000;
        assert!(attacker_price < required_price);
    }
}
