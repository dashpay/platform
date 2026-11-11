mod v0;
mod v1;

use crate::contract_group::{
    generate_contract_group_id, ContractGroupMembership, ContractGroupRegistration,
};
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use platform_value::Identifier;

use crate::prelude::IdentityNonce;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
pub use v0::*;
pub use v1::*;

impl DataContractCreateTransitionAccessorsV0 for DataContractCreateTransition {
    fn data_contract(&self) -> &DataContractInSerializationFormat {
        match self {
            DataContractCreateTransition::V0(transition) => &transition.data_contract,
            DataContractCreateTransition::V1(transition) => &transition.data_contract,
        }
    }

    fn identity_nonce(&self) -> IdentityNonce {
        match self {
            DataContractCreateTransition::V0(transition) => transition.identity_nonce,
            DataContractCreateTransition::V1(transition) => transition.identity_nonce,
        }
    }

    fn set_data_contract(&mut self, data_contract: DataContractInSerializationFormat) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.data_contract = data_contract;
            }
            DataContractCreateTransition::V1(transition) => {
                transition.data_contract = data_contract;
            }
        }
    }
}

impl DataContractCreateTransitionAccessorsV1 for DataContractCreateTransition {
    fn contract_group(&self) -> Option<&ContractGroupRegistration> {
        match self {
            DataContractCreateTransition::V0(_) => None,
            DataContractCreateTransition::V1(transition) => transition.contract_group.as_ref(),
        }
    }

    fn contract_group_id(&self) -> Option<Identifier> {
        self.contract_group().map(|_| {
            generate_contract_group_id(&self.data_contract().owner_id(), self.identity_nonce())
        })
    }

    fn contract_group_memberships(&self) -> &[ContractGroupMembership] {
        match self {
            DataContractCreateTransition::V0(_) => &[],
            DataContractCreateTransition::V1(transition) => {
                transition.contract_group_memberships.as_slice()
            }
        }
    }
}
