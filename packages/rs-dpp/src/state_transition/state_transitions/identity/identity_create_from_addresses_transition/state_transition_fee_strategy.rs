use crate::address_funds::AddressFundsFeeStrategy;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::StateTransitionAddressesFeeStrategy;

impl StateTransitionAddressesFeeStrategy for IdentityCreateFromAddressesTransition {
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => &transition.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            IdentityCreateFromAddressesTransition::V0(transition) => {
                transition.fee_strategy = fee_strategy
            }
        }
    }
}
