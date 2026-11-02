use crate::error::Error;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use crate::state_transition_action::system::bump_address_input_nonces_action::{
    BumpAddressInputNoncesAction, BumpAddressInputNoncesActionV0,
};
use dpp::fee::Credits;
use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;

impl BumpAddressInputNoncesAction {
    // IdentityCreateFromAddresses transformers

    /// from a failed IdentityCreateFromAddresses transition and the action it was transformed
    /// into: every input keeps its whole balance and pays the fee and `penalty_credits`, see
    /// [`BumpAddressInputNoncesActionV0::from_failed_identity_create_from_addresses_transition`]
    pub fn from_failed_identity_create_from_addresses_transition(
        value: &IdentityCreateFromAddressesTransition,
        action: &IdentityCreateFromAddressesTransitionAction,
        penalty_credits: Credits,
    ) -> Result<Self, Error> {
        match value {
            IdentityCreateFromAddressesTransition::V0(v0) => {
                BumpAddressInputNoncesActionV0::from_failed_identity_create_from_addresses_transition(
                    v0,
                    action.inputs_with_remaining_balance(),
                    penalty_credits,
                )
                .map(Into::into)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
    use crate::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNonceActionAccessorsV0;
    use dpp::address_funds::fee_strategy::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep};
    use dpp::address_funds::PlatformAddress;
    use dpp::identifier::Identifier;
    use dpp::prelude::AddressNonce;
    use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
    use std::collections::BTreeMap;

    const TEST_FEE: u16 = 7;
    const TEST_PENALTY: Credits = 900;

    fn make_inputs() -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (10_u32, 5000_u64));
        inputs
    }

    fn make_strategy() -> AddressFundsFeeStrategy {
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]
    }

    #[test]
    fn should_give_a_failed_identity_create_from_addresses_its_whole_input_balances() {
        let strategy = make_strategy();
        let address = PlatformAddress::P2pkh([0xAA; 20]);
        let mut inputs = BTreeMap::new();
        inputs.insert(address, (10_u32, 1000_u64));
        let transition =
            IdentityCreateFromAddressesTransition::V0(IdentityCreateFromAddressesTransitionV0 {
                public_keys: vec![],
                inputs,
                output: None,
                fee_strategy: strategy.clone(),
                user_fee_increase: TEST_FEE,
                input_witnesses: vec![],
            });
        let action = IdentityCreateFromAddressesTransitionAction::V0(
            IdentityCreateFromAddressesTransitionActionV0 {
                inputs_with_remaining_balance: make_inputs(),
                output: None,
                fee_strategy: strategy,
                public_keys: vec![],
                identity_id: Identifier::from([0xCC; 32]),
                fund_identity_amount: 1000,
                user_fee_increase: TEST_FEE,
            },
        );
        let bump =
            BumpAddressInputNoncesAction::from_failed_identity_create_from_addresses_transition(
                &transition,
                &action,
                TEST_PENALTY,
            )
            .expect("the balances cover the inputs");
        assert!(matches!(bump, BumpAddressInputNoncesAction::V0(_)));
        assert_eq!(bump.user_fee_increase(), TEST_FEE);
        assert_eq!(bump.penalty_credits(), TEST_PENALTY);
        // 5000 remained after spending 1000, so the input keeps all 6000.
        assert_eq!(
            bump.inputs_with_remaining_balance().get(&address),
            Some(&(10_u32, 6000_u64))
        );
    }
}
