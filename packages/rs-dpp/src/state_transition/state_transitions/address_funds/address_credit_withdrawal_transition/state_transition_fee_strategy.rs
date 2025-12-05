use crate::address_funds::AddressFundsFeeStrategy;
use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use crate::state_transition::StateTransitionAddressesFeeStrategy;

impl StateTransitionAddressesFeeStrategy for AddressCreditWithdrawalTransition {
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => &v0.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.fee_strategy = fee_strategy,
        }
    }
}
