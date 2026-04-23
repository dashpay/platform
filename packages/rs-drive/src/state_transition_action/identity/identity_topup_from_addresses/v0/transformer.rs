use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use std::collections::BTreeMap;

impl IdentityTopUpFromAddressesTransitionActionV0 {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    pub fn try_from_transition(
        value: &IdentityTopUpFromAddressesTransitionV0,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        let IdentityTopUpFromAddressesTransitionV0 {
            inputs,
            identity_id,
            output,
            fee_strategy,
            user_fee_increase,
            ..
        } = value;

        // Sum all balances from inputs (checked to prevent overflow)
        let total_inputs: Credits = match inputs
            .values()
            .try_fold(0u64, |acc, (_, balance)| acc.checked_add(*balance))
        {
            Some(sum) => sum,
            None => {
                return ConsensusValidationResult::new_with_error(
                    OverflowError::new(
                        "Input sum overflow in identity topup transformer".to_string(),
                    )
                    .into(),
                )
            }
        };

        // Subtract the output amount if present to get the topup amount
        let topup_amount = match output {
            Some((_, output_amount)) => match total_inputs.checked_sub(*output_amount) {
                Some(diff) => diff,
                None => {
                    return ConsensusValidationResult::new_with_error(
                        OverflowError::new(
                            "Output exceeds input sum in identity topup transformer".to_string(),
                        )
                        .into(),
                    )
                }
            },
            None => total_inputs,
        };

        ConsensusValidationResult::new_with_data(IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            output: *output,
            fee_strategy: fee_strategy.clone(),
            identity_id: *identity_id,
            topup_amount,
            user_fee_increase: *user_fee_increase,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::PlatformAddress;

    fn base_transition_v0() -> IdentityTopUpFromAddressesTransitionV0 {
        IdentityTopUpFromAddressesTransitionV0::default()
    }

    #[test]
    fn overflowing_input_sum_produces_overflow_error() {
        let mut v0 = base_transition_v0();
        v0.inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, u64::MAX - 5));
        v0.inputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), (0u32, u64::MAX - 5));

        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityTopUpFromAddressesTransitionActionV0::try_from_transition(
            &v0,
            inputs_remaining,
        );
        assert!(!result.is_valid());
        let errors = result.errors;
        assert!(!errors.is_empty());
        let msg = format!("{:?}", errors[0]);
        assert!(
            msg.to_lowercase().contains("overflow"),
            "expected overflow error, got {msg}"
        );
    }

    #[test]
    fn output_exceeding_inputs_produces_overflow_error() {
        let mut v0 = base_transition_v0();
        v0.inputs
            .insert(PlatformAddress::P2pkh([3u8; 20]), (0u32, 50));
        v0.output = Some((PlatformAddress::P2sh([9u8; 20]), 200));

        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityTopUpFromAddressesTransitionActionV0::try_from_transition(
            &v0,
            inputs_remaining,
        );
        assert!(!result.is_valid());
        let errors = result.errors;
        assert!(!errors.is_empty());
        let msg = format!("{:?}", errors[0]);
        assert!(
            msg.to_lowercase().contains("output exceeds")
                || msg.to_lowercase().contains("overflow"),
            "expected output-exceeds-input error, got {msg}"
        );
    }

    #[test]
    fn valid_inputs_without_output_succeed_and_topup_equals_sum() {
        let mut v0 = base_transition_v0();
        v0.inputs
            .insert(PlatformAddress::P2pkh([4u8; 20]), (1u32, 400));
        v0.inputs
            .insert(PlatformAddress::P2pkh([5u8; 20]), (2u32, 100));
        v0.output = None;

        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityTopUpFromAddressesTransitionActionV0::try_from_transition(
            &v0,
            inputs_remaining,
        );
        assert!(result.is_valid(), "errors: {:?}", result.errors);
        let action = result.into_data().expect("action");
        assert_eq!(action.topup_amount, 500);
        assert!(action.output.is_none());
    }

    #[test]
    fn valid_inputs_with_output_subtract_from_topup() {
        let mut v0 = base_transition_v0();
        v0.inputs
            .insert(PlatformAddress::P2pkh([8u8; 20]), (1u32, 1_000));
        v0.output = Some((PlatformAddress::P2sh([0xAA; 20]), 700));

        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityTopUpFromAddressesTransitionActionV0::try_from_transition(
            &v0,
            inputs_remaining,
        );
        assert!(result.is_valid(), "errors: {:?}", result.errors);
        let action = result.into_data().expect("action");
        assert_eq!(action.topup_amount, 300);
    }

    #[test]
    fn empty_inputs_produce_topup_of_zero_when_no_output() {
        // Edge case: zero inputs and no output should produce topup = 0.
        // This covers the "None" branch of the output match with 0-sum inputs.
        let v0 = base_transition_v0();
        let result =
            IdentityTopUpFromAddressesTransitionActionV0::try_from_transition(&v0, BTreeMap::new());
        assert!(result.is_valid(), "errors: {:?}", result.errors);
        let action = result.into_data().expect("action");
        assert_eq!(action.topup_amount, 0);
    }
}
