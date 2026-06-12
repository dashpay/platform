use std::collections::BTreeMap;
use std::sync::Arc;
use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::state::token::{TokenAmountUnderMinimumSaleAmount, TokenDirectPurchaseUserPriceTooLow, TokenNotForDirectSale};
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
use dpp::ProtocolError;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
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
                // All of the `SetPrices` resolution logic (tier lookup, overflow, under-minimum,
                // and the empty-schedule case that must never `.expect()` a key) lives in a pure
                // helper so it can be unit-tested directly. On rejection we bump the nonce and
                // surface the consensus error.
                match resolve_set_prices_direct_purchase_price(
                    base.token_id(),
                    &set_prices,
                    *token_count,
                    *total_agreed_price,
                ) {
                    Ok(required_total) => required_total,
                    Err(error) => {
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
                                vec![error],
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

/// Resolves the required total price for a `SetPrices` (tiered) direct purchase.
///
/// Returns the required total in credits on success, or the consensus error that must reject
/// the purchase. This is a pure function so every rejection branch — in particular the
/// empty-schedule case — can be unit-tested directly without standing up a `Drive`.
///
/// An empty `SetPrices` map is a representable, storable value, so this function must NOT
/// assume the map is non-empty: the original inline code did
/// `set_prices.keys().next().expect("Map is not empty")`, which panics on an empty map. That
/// panic was uncaught during per-state-transition processing and would deterministically halt
/// the chain across the quorum. Here an empty schedule resolves to `TokenNotForDirectSale`.
fn resolve_set_prices_direct_purchase_price(
    token_id: Identifier,
    set_prices: &BTreeMap<TokenAmount, Credits>,
    token_count: TokenAmount,
    total_agreed_price: Credits,
) -> Result<Credits, ConsensusError> {
    match set_prices.range(..=token_count).next_back() {
        Some((_matched_quantity, matched_price)) => {
            // The user-set price is bounded in structure validation, so a failed multiplication
            // here can only be a genuine u64 overflow.
            let required_total = matched_price.checked_mul(token_count).ok_or_else(|| {
                ConsensusError::BasicError(dpp::consensus::basic::BasicError::OverflowError(
                    OverflowError::new(
                        "overflow when calculating required total price in SetPrices direct purchase"
                            .to_string(),
                    ),
                ))
            })?;

            if total_agreed_price < required_total {
                return Err(ConsensusError::StateError(
                    StateError::TokenDirectPurchaseUserPriceTooLow(
                        TokenDirectPurchaseUserPriceTooLow::new(
                            token_id,
                            total_agreed_price,
                            required_total,
                        ),
                    ),
                ));
            }

            Ok(required_total)
        }
        // `range(..=token_count).next_back()` returns `None` in two situations:
        //   1. the schedule has tiers but the smallest one is above `token_count`
        //      (the buyer is under the minimum sale amount), or
        //   2. the schedule is empty — the token has no usable direct-sale price.
        None => Err(match set_prices.keys().next() {
            // At least one tier exists: the buyer is below the minimum sale amount.
            Some(minimum_sale_amount) => {
                ConsensusError::StateError(StateError::TokenAmountUnderMinimumSaleAmount(
                    TokenAmountUnderMinimumSaleAmount::new(
                        token_id,
                        token_count,
                        *minimum_sale_amount,
                    ),
                ))
            }
            // Empty schedule: the token has no direct-sale price at all.
            None => ConsensusError::StateError(StateError::TokenNotForDirectSale(
                TokenNotForDirectSale::new(token_id),
            )),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::nonminimal_bool)]
mod tests {
    use super::resolve_set_prices_direct_purchase_price;
    use dpp::balances::credits::TokenAmount;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::fee::Credits;
    use dpp::identifier::Identifier;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use std::collections::BTreeMap;

    /// Verifies that `checked_mul` correctly detects overflow for the values
    /// that previously would have silently wrapped in the `SetPrices` branch.
    ///
    /// With the fix, `matched_price.checked_mul(token_count)` returns `None`
    /// when the product would overflow u64, causing the transformer to return
    /// an `OverflowError` consensus error instead of a wrapped value.
    #[test]
    fn set_prices_checked_mul_returns_none_on_overflow() {
        // These values cause overflow:
        // matched_price = u64::MAX / 3 + 1 = 6_148_914_691_236_517_206
        // token_count = 3
        // True product = 18_446_744_073_709_551_618 > u64::MAX
        let matched_price: Credits = u64::MAX / 3 + 1;
        let token_count: TokenAmount = 3;

        // checked_mul returns None on overflow -- this is what the fixed code uses
        assert_eq!(
            matched_price.checked_mul(token_count),
            None,
            "checked_mul must return None when the product overflows u64"
        );

        // For comparison, wrapping_mul would silently produce 2 (the old bug)
        assert_eq!(
            matched_price.wrapping_mul(token_count),
            2,
            "wrapping_mul silently wraps to 2 -- this was the vulnerability"
        );
    }

    /// Verifies the fix with realistic SetPrices schedule parameters, using
    /// `BTreeMap::range` exactly as the transformer code does.
    ///
    /// With the fix, overflow returns an error rather than allowing an
    /// attacker to purchase tokens at a wrapped (near-zero) price.
    #[test]
    fn set_prices_overflow_detected_via_btreemap_range_lookup() {
        // Build a SetPrices schedule with a high per-token price at the 1-token tier.
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        let large_price: Credits = (1u64 << 63) + 1; // 9_223_372_036_854_775_809
        set_prices.insert(1, large_price);

        let token_count: TokenAmount = 2;

        // Replicate the exact BTreeMap lookup from the transformer
        let matched_price = match set_prices.range(..=token_count).next_back() {
            Some((_matched_quantity, price)) => *price,
            None => panic!("Should have found a matching price tier"),
        };

        assert_eq!(matched_price, large_price);

        // The fixed code uses checked_mul, which returns None on overflow
        assert_eq!(
            matched_price.checked_mul(token_count),
            None,
            "checked_mul detects overflow: the transformer will return an OverflowError"
        );
    }

    /// Verifies that normal (non-overflowing) SetPrices purchases still work
    /// correctly with `checked_mul`.
    #[test]
    fn set_prices_checked_mul_works_for_normal_values() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        let price_per_token: Credits = 1_000_000; // 1 million credits per token
        set_prices.insert(1, price_per_token);

        let token_count: TokenAmount = 100;

        let matched_price = match set_prices.range(..=token_count).next_back() {
            Some((_matched_quantity, price)) => *price,
            None => panic!("Should have found a matching price tier"),
        };

        // checked_mul returns Some for values that fit in u64
        let required_total = matched_price.checked_mul(token_count);
        assert_eq!(
            required_total,
            Some(100_000_000),
            "Normal multiplication should succeed and return the correct product"
        );

        // A user paying the correct amount passes the price check
        let user_agreed_price: Credits = 100_000_000;
        assert!(user_agreed_price >= required_total.unwrap());
    }

    /// Verify that SinglePrice branch (saturating_mul) is safe for the same inputs.
    #[test]
    fn single_price_saturating_mul_is_safe() {
        let price_per_token: Credits = u64::MAX / 3 + 1;
        let token_count: TokenAmount = 3;

        // SinglePrice branch uses saturating_mul
        let required_price = price_per_token.saturating_mul(token_count);

        // Should saturate to u64::MAX, not wrap
        assert_eq!(required_price, u64::MAX);

        // Any attacker offering less than u64::MAX would be rejected
        let attacker_price: Credits = 1_000_000;
        assert!(attacker_price < required_price);
    }

    // =========================================================================
    // Additional coverage for branches that the above tests do not exercise:
    // - SinglePrice branch success path (user pays at least required_price)
    // - SinglePrice branch with zero token_count
    // - SetPrices branch: exact-match lookup
    // - SetPrices branch: range find picks the highest applicable tier
    // - SetPrices branch: token_count below all defined thresholds -> None branch
    // - required_total user-price gating on the SetPrices branch
    // - Zero price boundary (SinglePrice price_per_token = 0)
    // =========================================================================

    /// Happy path: SinglePrice(p) * token_count succeeds and any user who agrees to
    /// pay exactly that amount passes the price check (>= required_price).
    #[test]
    fn single_price_happy_path_exact_payment() {
        let price_per_token: Credits = 250;
        let token_count: TokenAmount = 4;
        let required_price = price_per_token.saturating_mul(token_count);
        assert_eq!(required_price, 1_000);

        // An agreed_price equal to required_price satisfies `!(total_agreed_price < required_price)`.
        let total_agreed_price: Credits = 1_000;
        assert!(!(total_agreed_price < required_price));
    }

    /// A user overpayment is accepted. The transformer records `required_price`, not the agreed amount.
    #[test]
    fn single_price_overpayment_is_accepted_and_required_price_is_stored() {
        let price_per_token: Credits = 10;
        let token_count: TokenAmount = 7;
        let required_price = price_per_token.saturating_mul(token_count);
        assert_eq!(required_price, 70);

        let total_agreed_price: Credits = 100;
        assert!(!(total_agreed_price < required_price));
        // Transformer would store `required_price`, not `total_agreed_price`.
    }

    /// A user underpayment is rejected via `TokenDirectPurchaseUserPriceTooLow`.
    #[test]
    fn single_price_underpayment_triggers_price_too_low() {
        let price_per_token: Credits = 100;
        let token_count: TokenAmount = 5;
        let required_price = price_per_token.saturating_mul(token_count);
        assert_eq!(required_price, 500);

        let total_agreed_price: Credits = 499;
        assert!(total_agreed_price < required_price);
    }

    /// SinglePrice where price_per_token is 0 means tokens are free.
    /// required_price is 0 for any token_count, so any non-negative agreed_price is accepted.
    #[test]
    fn single_price_zero_per_token_requires_zero_total() {
        let price_per_token: Credits = 0;
        let token_count: TokenAmount = 1_000_000;
        let required_price = price_per_token.saturating_mul(token_count);
        assert_eq!(required_price, 0);

        // Even a user offering 0 is accepted.
        let total_agreed_price: Credits = 0;
        assert!(!(total_agreed_price < required_price));
    }

    /// SetPrices happy path with tiered lookup:
    /// tiers at {1: 100, 10: 80, 100: 50}. Buying 50 should match tier 10 with price 80.
    #[test]
    fn set_prices_tiered_lookup_picks_highest_applicable_tier() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 100);
        set_prices.insert(10, 80);
        set_prices.insert(100, 50);
        let token_count: TokenAmount = 50;

        let (matched_quantity, matched_price) = set_prices
            .range(..=token_count)
            .next_back()
            .expect("tier should be found");
        assert_eq!(*matched_quantity, 10);
        assert_eq!(*matched_price, 80);

        let required_total = matched_price.checked_mul(token_count).expect("no overflow");
        assert_eq!(required_total, 4_000);
    }

    /// SetPrices exact-match lookup — asking for exactly the tier boundary should hit that tier.
    #[test]
    fn set_prices_exact_tier_boundary_match() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 100);
        set_prices.insert(10, 80);
        let token_count: TokenAmount = 10;

        let (matched_quantity, matched_price) = set_prices
            .range(..=token_count)
            .next_back()
            .expect("tier should be found");
        assert_eq!(*matched_quantity, 10);
        assert_eq!(*matched_price, 80);
    }

    /// SetPrices with token_count above the highest tier uses the highest tier.
    #[test]
    fn set_prices_count_above_all_tiers_uses_top_tier() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 100);
        set_prices.insert(10, 80);
        set_prices.insert(100, 50);
        let token_count: TokenAmount = 10_000;

        let (matched_quantity, matched_price) = set_prices
            .range(..=token_count)
            .next_back()
            .expect("tier should be found");
        assert_eq!(*matched_quantity, 100);
        assert_eq!(*matched_price, 50);
    }

    /// Below-minimum-tier token_count: `range(..=token_count).next_back()` returns `None`,
    /// triggering the `TokenAmountUnderMinimumSaleAmount` consensus error.
    #[test]
    fn set_prices_below_minimum_tier_returns_none() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(5, 200);
        set_prices.insert(10, 150);
        let token_count: TokenAmount = 2;

        let matched = set_prices.range(..=token_count).next_back();
        assert!(matched.is_none());

        // The transformer then reads `set_prices.keys().next()` for the minimum threshold.
        let min_threshold = *set_prices.keys().next().expect("non-empty");
        assert_eq!(min_threshold, 5);
    }

    /// Regression test for the token direct-purchase chain-halt bug, exercised through the
    /// real resolution helper (not a `BTreeMap` re-implementation).
    ///
    /// An empty `SetPrices` schedule is a representable, storable value. The original inline
    /// code did `set_prices.keys().next().expect("Map is not empty")`, which panics on an empty
    /// map — an uncaught panic during per-state-transition processing that deterministically
    /// halts the chain across the quorum. The helper must instead resolve an empty schedule to a
    /// `TokenNotForDirectSale` consensus error. A future regression in the `None` arm (e.g.
    /// reintroducing `.expect()`) fails this test rather than leaving it green.
    #[test]
    fn resolve_set_prices_empty_map_returns_not_for_sale_without_panicking() {
        let set_prices = BTreeMap::<TokenAmount, Credits>::new();

        let result =
            resolve_set_prices_direct_purchase_price(Identifier::default(), &set_prices, 5, 1_000);

        assert!(
            matches!(
                result,
                Err(ConsensusError::StateError(
                    StateError::TokenNotForDirectSale(_)
                ))
            ),
            "empty schedule must resolve to TokenNotForDirectSale, got {result:?}"
        );
    }

    /// The other `None`-arm branch: a non-empty schedule whose smallest tier is above
    /// `token_count` resolves to `TokenAmountUnderMinimumSaleAmount`. This proves the helper
    /// distinguishes "empty" from "below minimum" through real code.
    #[test]
    fn resolve_set_prices_below_minimum_tier_returns_under_minimum_error() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(5, 200);
        set_prices.insert(10, 150);

        let result =
            resolve_set_prices_direct_purchase_price(Identifier::default(), &set_prices, 2, 1_000);

        assert!(
            matches!(
                result,
                Err(ConsensusError::StateError(
                    StateError::TokenAmountUnderMinimumSaleAmount(_)
                ))
            ),
            "below-minimum purchase must resolve to TokenAmountUnderMinimumSaleAmount, got {result:?}"
        );
    }

    /// Happy path through the helper: `token_count` matches the highest applicable tier and the
    /// agreed price covers it, so the required total is returned.
    #[test]
    fn resolve_set_prices_matched_tier_returns_required_total() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 100);
        set_prices.insert(10, 80);

        // token_count = 50 matches tier 10 (price 80) => required_total = 4_000.
        let result =
            resolve_set_prices_direct_purchase_price(Identifier::default(), &set_prices, 50, 4_000);

        assert_eq!(result.expect("matched tier with sufficient payment"), 4_000);
    }

    /// Underpayment through the helper resolves to `TokenDirectPurchaseUserPriceTooLow`.
    #[test]
    fn resolve_set_prices_underpayment_returns_price_too_low() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 100);

        // required_total = 100 * 5 = 500; the user only agreed to 499.
        let result =
            resolve_set_prices_direct_purchase_price(Identifier::default(), &set_prices, 5, 499);

        assert!(
            matches!(
                result,
                Err(ConsensusError::StateError(
                    StateError::TokenDirectPurchaseUserPriceTooLow(_)
                ))
            ),
            "underpayment must resolve to TokenDirectPurchaseUserPriceTooLow, got {result:?}"
        );
    }

    /// Overflow through the helper resolves to an `OverflowError` rather than wrapping.
    #[test]
    fn resolve_set_prices_overflow_returns_overflow_error() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        // price * token_count overflows u64: (u64::MAX/3 + 1) * 3 > u64::MAX.
        set_prices.insert(1, u64::MAX / 3 + 1);

        let result = resolve_set_prices_direct_purchase_price(
            Identifier::default(),
            &set_prices,
            3,
            u64::MAX,
        );

        assert!(
            matches!(
                result,
                Err(ConsensusError::BasicError(
                    dpp::consensus::basic::BasicError::OverflowError(_)
                ))
            ),
            "overflowing required total must resolve to OverflowError, got {result:?}"
        );
    }

    /// SetPrices underpayment: matched_price * token_count computes to required_total but
    /// the user agreed to less, so the transformer returns `TokenDirectPurchaseUserPriceTooLow`.
    #[test]
    fn set_prices_underpayment_triggers_price_too_low() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 100);
        set_prices.insert(10, 50);
        let token_count: TokenAmount = 20;

        let matched_price = *set_prices
            .range(..=token_count)
            .next_back()
            .expect("tier found")
            .1;
        let required_total = matched_price.checked_mul(token_count).expect("no overflow");
        assert_eq!(required_total, 1_000);

        let total_agreed_price: Credits = 999;
        assert!(total_agreed_price < required_total);
    }

    /// SetPrices exact-payment accepted.
    #[test]
    fn set_prices_exact_payment_accepted() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 10);
        let token_count: TokenAmount = 5;

        let matched_price = *set_prices
            .range(..=token_count)
            .next_back()
            .expect("tier found")
            .1;
        let required_total = matched_price.checked_mul(token_count).expect("no overflow");
        assert_eq!(required_total, 50);

        let total_agreed_price: Credits = 50;
        assert!(!(total_agreed_price < required_total));
    }

    /// SetPrices with a single tier that maps to price 0 — free up to any amount >= threshold.
    #[test]
    fn set_prices_free_tier_allows_any_agreed_price() {
        let mut set_prices = BTreeMap::<TokenAmount, Credits>::new();
        set_prices.insert(1, 0);
        let token_count: TokenAmount = 100;
        let matched_price = *set_prices
            .range(..=token_count)
            .next_back()
            .expect("tier")
            .1;
        let required_total = matched_price.checked_mul(token_count).expect("no overflow");
        assert_eq!(required_total, 0);
        // Any user-agreed price >= 0 trivially satisfies the comparison.
        assert!(!(0u64 < required_total));
    }

    /// When both SinglePrice and SetPrices are represented as enum variants, ensure
    /// pattern matching distinguishes them correctly (the transformer branches on this).
    #[test]
    fn pricing_schedule_enum_dispatch() {
        let single = TokenPricingSchedule::SinglePrice(500);
        let mut map = BTreeMap::new();
        map.insert(1u64, 50u64);
        let multi = TokenPricingSchedule::SetPrices(map);

        assert!(matches!(single, TokenPricingSchedule::SinglePrice(_)));
        assert!(matches!(multi, TokenPricingSchedule::SetPrices(_)));
    }

    /// SinglePrice(price) * 0 tokens yields 0 required_price. Any agreed price >= 0 clears it.
    /// This documents the behavior of the edge case where `token_count == 0`.
    #[test]
    fn single_price_zero_token_count_yields_zero_required_price() {
        let price_per_token: Credits = 1_000;
        let token_count: TokenAmount = 0;
        let required_price = price_per_token.saturating_mul(token_count);
        assert_eq!(required_price, 0);
    }
}
