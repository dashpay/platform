mod v0;

use crate::data_contract::document_type::action_fees::ContractFeePot;
use crate::prelude::IdentityNonce;
use crate::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use platform_value::Identifier;
pub use v0::*;

impl ContractFeeClaimTransitionAccessorsV0 for ContractFeeClaimTransition {
    fn set_owner_id(&mut self, id: Identifier) {
        match self {
            ContractFeeClaimTransition::V0(transition) => transition.set_owner_id(id),
        }
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        match self {
            ContractFeeClaimTransition::V0(transition) => transition.set_data_contract_id(id),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            ContractFeeClaimTransition::V0(transition) => transition.data_contract_id(),
        }
    }

    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            ContractFeeClaimTransition::V0(transition) => {
                transition.set_identity_contract_nonce(nonce)
            }
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            ContractFeeClaimTransition::V0(transition) => transition.identity_contract_nonce(),
        }
    }

    fn set_pot(&mut self, pot: ContractFeePot) {
        match self {
            ContractFeeClaimTransition::V0(transition) => transition.set_pot(pot),
        }
    }

    fn pot(&self) -> ContractFeePot {
        match self {
            ContractFeeClaimTransition::V0(transition) => transition.pot(),
        }
    }
}
