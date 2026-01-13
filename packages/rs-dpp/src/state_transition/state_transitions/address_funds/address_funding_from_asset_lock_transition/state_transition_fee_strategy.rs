use crate::address_funds::AddressFundsFeeStrategy;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::state_transition::StateTransitionAddressesFeeStrategy;

impl StateTransitionAddressesFeeStrategy for AddressFundingFromAssetLockTransition {
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => &v0.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            AddressFundingFromAssetLockTransition::V0(v0) => v0.fee_strategy = fee_strategy,
        }
    }
}
