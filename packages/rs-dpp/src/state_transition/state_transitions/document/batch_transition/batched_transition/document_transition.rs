use platform_value::{Identifier, Value};
use std::collections::BTreeMap;
use derive_more::{Display, From};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use bincode::{Encode, Decode};
use crate::prelude::{IdentityNonce, Revision};
use crate::state_transition::batch_transition::{DocumentCreateTransition, DocumentDeleteTransition, DocumentReplaceTransition, TokenBurnTransition, TokenConfigUpdateTransition, TokenDestroyFrozenFundsTransition, TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition, TokenClaimTransition, TokenTransferTransition, TokenUnfreezeTransition, TokenDirectPurchaseTransition, TokenSetPriceForDirectPurchaseTransition};
use crate::state_transition::batch_transition::batched_transition::{DocumentPurchaseTransition, DocumentTransferTransition, DocumentUpdatePriceTransition};
use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;

#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum DocumentTransition {
    #[display("CreateDocumentTransition({})", "_0")]
    Create(DocumentCreateTransition),

    #[display("ReplaceDocumentTransition({})", "_0")]
    Replace(DocumentReplaceTransition),

    #[display("DeleteDocumentTransition({})", "_0")]
    Delete(DocumentDeleteTransition),

    #[display("TransferDocumentTransition({})", "_0")]
    Transfer(DocumentTransferTransition),

    #[display("UpdatePriceDocumentTransition({})", "_0")]
    UpdatePrice(DocumentUpdatePriceTransition),

    #[display("PurchaseDocumentTransition({})", "_0")]
    Purchase(DocumentPurchaseTransition),
}

impl BatchTransitionResolversV0 for DocumentTransition {
    fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        if let Self::Create(ref t) = self {
            Some(t)
        } else {
            None
        }
    }
    fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        if let Self::Replace(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        if let Self::Delete(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_transfer(&self) -> Option<&DocumentTransferTransition> {
        if let Self::Transfer(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_purchase(&self) -> Option<&DocumentPurchaseTransition> {
        if let Self::Purchase(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    fn as_transition_token_burn(&self) -> Option<&TokenBurnTransition> {
        None
    }

    fn as_transition_token_mint(&self) -> Option<&TokenMintTransition> {
        None
    }

    fn as_transition_token_transfer(&self) -> Option<&TokenTransferTransition> {
        None
    }

    fn as_transition_token_freeze(&self) -> Option<&TokenFreezeTransition> {
        None
    }

    fn as_transition_token_unfreeze(&self) -> Option<&TokenUnfreezeTransition> {
        None
    }

    fn as_transition_token_destroy_frozen_funds(
        &self,
    ) -> Option<&TokenDestroyFrozenFundsTransition> {
        None
    }

    fn as_transition_token_claim(&self) -> Option<&TokenClaimTransition> {
        None
    }

    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition> {
        None
    }

    fn as_transition_token_config_update(&self) -> Option<&TokenConfigUpdateTransition> {
        None
    }

    fn as_transition_token_direct_purchase(&self) -> Option<&TokenDirectPurchaseTransition> {
        None
    }

    fn as_transition_token_set_price_for_direct_purchase(
        &self,
    ) -> Option<&TokenSetPriceForDirectPurchaseTransition> {
        None
    }
}

pub trait DocumentTransitionV0Methods {
    fn base(&self) -> &DocumentBaseTransition;
    /// returns the value of dynamic property. The dynamic property is a property that is not specified in protocol
    /// the `path` supports dot-syntax: i.e: property.internal_property
    fn get_dynamic_property(&self, path: &str) -> Option<&Value>;
    ///  get the id
    fn get_id(&self) -> Identifier;
    /// get the entropy
    fn entropy(&self) -> Option<Vec<u8>>;
    fn document_type_name(&self) -> &String;
    /// get the data contract id
    fn data_contract_id(&self) -> Identifier;
    /// get the data of the transition if exits
    fn data(&self) -> Option<&BTreeMap<String, Value>>;
    /// get the revision of transition if exits
    fn revision(&self) -> Option<Revision>;

    /// get the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;
    #[cfg(test)]
    /// Inserts the dynamic property into the document
    fn insert_dynamic_property(&mut self, property_name: String, value: Value);
    /// set data contract's ID
    fn set_data_contract_id(&mut self, id: Identifier);
    fn base_mut(&mut self) -> &mut DocumentBaseTransition;
    fn data_mut(&mut self) -> Option<&mut BTreeMap<String, Value>>;

    // sets revision of the transition
    fn set_revision(&mut self, revision: Revision);

    // sets identity contract nonce
    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce);
}

impl DocumentTransitionV0Methods for DocumentTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentTransition::Create(t) => t.base(),
            DocumentTransition::Replace(t) => t.base(),
            DocumentTransition::Delete(t) => t.base(),
            DocumentTransition::Transfer(t) => t.base(),
            DocumentTransition::UpdatePrice(t) => t.base(),
            DocumentTransition::Purchase(t) => t.base(),
        }
    }

    fn get_dynamic_property(&self, path: &str) -> Option<&Value> {
        match self {
            DocumentTransition::Create(t) => t.data().get(path),
            DocumentTransition::Replace(t) => t.data().get(path),
            DocumentTransition::Delete(_) => None,
            DocumentTransition::Transfer(_) => None,
            DocumentTransition::UpdatePrice(_) => None,
            DocumentTransition::Purchase(_) => None,
        }
    }

    fn get_id(&self) -> Identifier {
        self.base().id()
    }

    fn document_type_name(&self) -> &String {
        self.base().document_type_name()
    }

    fn entropy(&self) -> Option<Vec<u8>> {
        match self {
            DocumentTransition::Create(t) => Some(Vec::from(t.entropy())),
            DocumentTransition::Replace(_) => None,
            DocumentTransition::Delete(_) => None,
            DocumentTransition::Transfer(_) => None,
            DocumentTransition::UpdatePrice(_) => None,
            DocumentTransition::Purchase(_) => None,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    fn data(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            DocumentTransition::Create(t) => Some(t.data()),
            DocumentTransition::Replace(t) => Some(t.data()),
            DocumentTransition::Delete(_) => None,
            DocumentTransition::Transfer(_) => None,
            DocumentTransition::UpdatePrice(_) => None,
            DocumentTransition::Purchase(_) => None,
        }
    }

    fn revision(&self) -> Option<Revision> {
        match self {
            DocumentTransition::Create(_) => Some(1),
            DocumentTransition::Replace(t) => Some(t.revision()),
            DocumentTransition::Delete(_) => None,
            DocumentTransition::Transfer(t) => Some(t.revision()),
            DocumentTransition::UpdatePrice(t) => Some(t.revision()),
            DocumentTransition::Purchase(t) => Some(t.revision()),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DocumentTransition::Create(t) => t.base().identity_contract_nonce(),
            DocumentTransition::Replace(t) => t.base().identity_contract_nonce(),
            DocumentTransition::Delete(t) => t.base().identity_contract_nonce(),
            DocumentTransition::Transfer(t) => t.base().identity_contract_nonce(),
            DocumentTransition::UpdatePrice(t) => t.base().identity_contract_nonce(),
            DocumentTransition::Purchase(t) => t.base().identity_contract_nonce(),
        }
    }

    #[cfg(test)]
    fn insert_dynamic_property(&mut self, property_name: String, value: Value) {
        match self {
            DocumentTransition::Create(document_create_transition) => {
                document_create_transition
                    .data_mut()
                    .insert(property_name, value);
            }
            DocumentTransition::Replace(document_replace_transition) => {
                document_replace_transition
                    .data_mut()
                    .insert(property_name, value);
            }
            DocumentTransition::Delete(_) => {}
            DocumentTransition::Transfer(_) => {}
            DocumentTransition::UpdatePrice(_) => {}
            DocumentTransition::Purchase(_) => {}
        }
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        self.base_mut().set_data_contract_id(id)
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentTransition::Create(t) => t.base_mut(),
            DocumentTransition::Replace(t) => t.base_mut(),
            DocumentTransition::Delete(t) => t.base_mut(),
            DocumentTransition::Transfer(t) => t.base_mut(),
            DocumentTransition::UpdatePrice(t) => t.base_mut(),
            DocumentTransition::Purchase(t) => t.base_mut(),
        }
    }

    fn data_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        match self {
            DocumentTransition::Create(t) => Some(t.data_mut()),
            DocumentTransition::Replace(t) => Some(t.data_mut()),
            DocumentTransition::Delete(_) => None,
            DocumentTransition::Transfer(_) => None,
            DocumentTransition::UpdatePrice(_) => None,
            DocumentTransition::Purchase(_) => None,
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            DocumentTransition::Create(_) => {}
            DocumentTransition::Replace(ref mut t) => t.set_revision(revision),
            DocumentTransition::Delete(_) => {}
            DocumentTransition::Transfer(ref mut t) => t.set_revision(revision),
            DocumentTransition::UpdatePrice(ref mut t) => t.set_revision(revision),
            DocumentTransition::Purchase(ref mut t) => t.set_revision(revision),
        }
    }

    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            DocumentTransition::Create(t) => t.base_mut().set_identity_contract_nonce(nonce),
            DocumentTransition::Replace(t) => t.base_mut().set_identity_contract_nonce(nonce),
            DocumentTransition::Delete(t) => t.base_mut().set_identity_contract_nonce(nonce),
            DocumentTransition::Transfer(t) => t.base_mut().set_identity_contract_nonce(nonce),
            DocumentTransition::UpdatePrice(t) => t.base_mut().set_identity_contract_nonce(nonce),
            DocumentTransition::Purchase(t) => t.base_mut().set_identity_contract_nonce(nonce),
        }
    }
}
