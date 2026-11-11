mod v0;

pub use v0::*;

use crate::shielded::SerializedAction;
use crate::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use platform_value::{BinaryData, Identifier};

impl PrivateDocumentCreateFromShieldedPoolTransitionAccessorsV0
    for PrivateDocumentCreateFromShieldedPoolTransition
{
    fn data_contract_id(&self) -> Identifier {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.data_contract_id,
        }
    }

    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => {
                v0.data_contract_id = data_contract_id
            }
        }
    }

    fn document_type_name(&self) -> &str {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => &v0.document_type_name,
        }
    }

    fn set_document_type_name(&mut self, document_type_name: String) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => {
                v0.document_type_name = document_type_name
            }
        }
    }

    fn entries(&self) -> &[BinaryData] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => &v0.entries,
        }
    }

    fn set_entries(&mut self, entries: Vec<BinaryData>) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.entries = entries,
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => &v0.actions,
        }
    }

    fn set_actions(&mut self, actions: Vec<SerializedAction>) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.actions = actions,
        }
    }

    fn value_balance(&self) -> u64 {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.value_balance,
        }
    }

    fn set_value_balance(&mut self, value_balance: u64) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => {
                v0.value_balance = value_balance
            }
        }
    }

    fn anchor(&self) -> [u8; 32] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.anchor,
        }
    }

    fn set_anchor(&mut self, anchor: [u8; 32]) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.anchor = anchor,
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => &v0.proof,
        }
    }

    fn set_proof(&mut self, proof: Vec<u8>) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.proof = proof,
        }
    }

    fn binding_signature(&self) -> [u8; 64] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.binding_signature,
        }
    }

    fn set_binding_signature(&mut self, binding_signature: [u8; 64]) {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => {
                v0.binding_signature = binding_signature
            }
        }
    }
}
