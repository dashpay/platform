use crate::address_funds::AddressFundsFeeStrategy;
use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use crate::state_transition::StateTransitionAddressesFeeStrategy;

impl StateTransitionAddressesFeeStrategy for AddressFundsTransferTransition {
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressFundsTransferTransition::V0(transition) => &transition.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            AddressFundsTransferTransition::V0(transition) => {
                transition.fee_strategy = fee_strategy;
            }
        }
    }
}
