use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::basic::value_error::ValueError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use dpp::state_transition::StateTransitionIdentityIdFromInputs;
use std::collections::BTreeMap;

impl IdentityCreateFromAddressesTransitionActionV0 {
    /// Transforms the state transition into an action using pre-validated inputs with remaining balances.
    pub fn try_from_transition(
        value: &IdentityCreateFromAddressesTransitionV0,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> ConsensusValidationResult<Self> {
        let identity_id = match value.identity_id_from_inputs() {
            Ok(id) => id,
            Err(e) => {
                return ConsensusValidationResult::new_with_error(ConsensusError::from(
                    ValueError::new_from_string(format!(
                        "Failed to calculate identity id from inputs: {}",
                        e
                    )),
                ));
            }
        };

        let IdentityCreateFromAddressesTransitionV0 {
            inputs,
            output,
            fee_strategy,
            public_keys,
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
                        "Input sum overflow in identity create transformer".to_string(),
                    )
                    .into(),
                )
            }
        };

        // Subtract the output amount if present
        let fund_identity_amount = match output {
            Some((_, output_amount)) => match total_inputs.checked_sub(*output_amount) {
                Some(diff) => diff,
                None => {
                    return ConsensusValidationResult::new_with_error(
                        OverflowError::new(
                            "Output exceeds input sum in identity create transformer".to_string(),
                        )
                        .into(),
                    )
                }
            },
            None => total_inputs,
        };

        ConsensusValidationResult::new_with_data(IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance,
            output: *output,
            fee_strategy: fee_strategy.clone(),
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            identity_id,
            fund_identity_amount,
            user_fee_increase: *user_fee_increase,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::PlatformAddress;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;

    /// Build a minimal V0 transition. `inputs` is always set empty so
    /// `identity_id_from_inputs` would fail; callers override as needed.
    fn base_transition_v0() -> IdentityCreateFromAddressesTransitionV0 {
        IdentityCreateFromAddressesTransitionV0::default()
    }

    #[test]
    fn empty_inputs_produce_identity_id_derivation_error() {
        // Per dpp: `identity_id_from_input_addresses` returns ParsingError when
        // inputs is empty. The transformer wraps this as ValueError.
        let value = base_transition_v0();
        let result = IdentityCreateFromAddressesTransitionActionV0::try_from_transition(
            &value,
            BTreeMap::new(),
        );
        assert!(!result.is_valid());
        let errors = result.errors;
        assert!(!errors.is_empty());
        // Any ValueError variant is acceptable -- specific wording tested in fmt
        match &errors[0] {
            ConsensusError::BasicError(BasicError::ValueError(e)) => {
                assert!(
                    e.to_string().contains("Failed to calculate identity id"),
                    "unexpected ValueError message: {}",
                    e
                );
            }
            other => panic!("expected ValueError, got {other:?}"),
        }
    }

    #[test]
    fn overflowing_input_sum_produces_overflow_error() {
        // Two inputs whose balances summed overflow u64
        let mut v0 = base_transition_v0();
        let addr_a = PlatformAddress::P2pkh([1u8; 20]);
        let addr_b = PlatformAddress::P2pkh([2u8; 20]);
        // Each ~= 2/3 of u64::MAX, summed they overflow
        let almost_max = u64::MAX - 10;
        v0.inputs.insert(addr_a, (0u32, almost_max));
        v0.inputs.insert(addr_b, (0u32, almost_max));
        // Provide non-empty balance map to pass identity_id derivation.
        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityCreateFromAddressesTransitionActionV0::try_from_transition(
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
        // inputs sum to 100 but output is 1000 -- must error in checked_sub branch
        let mut v0 = base_transition_v0();
        let addr = PlatformAddress::P2pkh([7u8; 20]);
        v0.inputs.insert(addr, (0u32, 100));
        v0.output = Some((PlatformAddress::P2sh([9u8; 20]), 1000));

        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityCreateFromAddressesTransitionActionV0::try_from_transition(
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
    fn valid_inputs_without_output_succeed_and_fund_equals_sum() {
        let mut v0 = base_transition_v0();
        let addr_a = PlatformAddress::P2pkh([4u8; 20]);
        let addr_b = PlatformAddress::P2pkh([5u8; 20]);
        v0.inputs.insert(addr_a, (1u32, 300));
        v0.inputs.insert(addr_b, (2u32, 200));
        v0.output = None;

        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityCreateFromAddressesTransitionActionV0::try_from_transition(
            &v0,
            inputs_remaining,
        );
        assert!(result.is_valid(), "errors: {:?}", result.errors);
        let action = result.into_data().expect("action");
        assert_eq!(action.fund_identity_amount, 500);
        assert!(action.output.is_none());
    }

    #[test]
    fn valid_inputs_with_output_subtract_from_fund_amount() {
        let mut v0 = base_transition_v0();
        let addr = PlatformAddress::P2pkh([8u8; 20]);
        v0.inputs.insert(addr, (1u32, 1_000));
        v0.output = Some((PlatformAddress::P2sh([0xAA; 20]), 250));

        let inputs_remaining: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            v0.inputs.iter().map(|(k, v)| (*k, *v)).collect();

        let result = IdentityCreateFromAddressesTransitionActionV0::try_from_transition(
            &v0,
            inputs_remaining,
        );
        assert!(result.is_valid(), "errors: {:?}", result.errors);
        let action = result.into_data().expect("action");
        assert_eq!(action.fund_identity_amount, 750);
        assert_eq!(
            action.output,
            Some((PlatformAddress::P2sh([0xAA; 20]), 250))
        );
    }
}
