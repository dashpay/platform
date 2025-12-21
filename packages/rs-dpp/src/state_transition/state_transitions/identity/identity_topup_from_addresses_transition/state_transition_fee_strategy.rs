use crate::address_funds::AddressFundsFeeStrategy;
use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use crate::state_transition::StateTransitionAddressesFeeStrategy;

impl StateTransitionAddressesFeeStrategy for IdentityTopUpFromAddressesTransition {
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => &transition.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            IdentityTopUpFromAddressesTransition::V0(transition) => {
                transition.fee_strategy = fee_strategy
            }
        }
    }
}
