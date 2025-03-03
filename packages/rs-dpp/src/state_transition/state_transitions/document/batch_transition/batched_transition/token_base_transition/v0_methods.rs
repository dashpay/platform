use crate::data_contract::GroupContractPosition;
use crate::group::GroupStateTransitionInfo;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use platform_value::Identifier;

impl TokenBaseTransitionV0Methods for TokenBaseTransition {
    fn token_contract_position(&self) -> u16 {
        match self {
            TokenBaseTransition::V0(v0) => v0.token_contract_position(),
        }
    }

    fn set_token_contract_position(&mut self, token_id: u16) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_token_contract_position(token_id),
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            TokenBaseTransition::V0(v0) => v0.data_contract_id(),
        }
    }

    fn data_contract_id_ref(&self) -> &Identifier {
        match self {
            TokenBaseTransition::V0(v0) => v0.data_contract_id_ref(),
        }
    }

    fn token_id(&self) -> Identifier {
        match self {
            TokenBaseTransition::V0(v0) => v0.token_id(),
        }
    }

    fn token_id_ref(&self) -> &Identifier {
        match self {
            TokenBaseTransition::V0(v0) => v0.token_id_ref(),
        }
    }

    fn set_token_id(&mut self, token_id: Identifier) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_token_id(token_id),
        }
    }

    fn group_position(&self) -> Option<GroupContractPosition> {
        match self {
            TokenBaseTransition::V0(v0) => v0.group_position(),
        }
    }

    fn using_group_info(&self) -> Option<GroupStateTransitionInfo> {
        match self {
            TokenBaseTransition::V0(v0) => v0.using_group_info(),
        }
    }

    fn set_using_group_info(&mut self, group_info: Option<GroupStateTransitionInfo>) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_using_group_info(group_info),
        }
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            TokenBaseTransition::V0(v0) => v0.set_data_contract_id(data_contract_id),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            TokenBaseTransition::V0(v0) => v0.identity_contract_nonce,
        }
    }

    fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            TokenBaseTransition::V0(v0) => v0.identity_contract_nonce = identity_contract_nonce,
        }
    }
}
