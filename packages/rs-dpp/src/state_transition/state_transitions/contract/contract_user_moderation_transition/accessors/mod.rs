mod v0;

use crate::prelude::IdentityNonce;
use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationAction;
use crate::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
use platform_value::Identifier;
pub use v0::*;

impl ContractUserModerationTransitionAccessorsV0 for ContractUserModerationTransition {
    fn set_owner_id(&mut self, id: Identifier) {
        match self {
            ContractUserModerationTransition::V0(transition) => transition.set_owner_id(id),
        }
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        match self {
            ContractUserModerationTransition::V0(transition) => transition.set_data_contract_id(id),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            ContractUserModerationTransition::V0(transition) => transition.data_contract_id(),
        }
    }

    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            ContractUserModerationTransition::V0(transition) => {
                transition.set_identity_contract_nonce(nonce)
            }
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            ContractUserModerationTransition::V0(transition) => {
                transition.identity_contract_nonce()
            }
        }
    }

    fn set_action(&mut self, action: ContractUserModerationAction) {
        match self {
            ContractUserModerationTransition::V0(transition) => transition.set_action(action),
        }
    }

    fn action(&self) -> &ContractUserModerationAction {
        match self {
            ContractUserModerationTransition::V0(transition) => transition.action(),
        }
    }
}
