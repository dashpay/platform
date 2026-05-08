use platform_value::{Identifier, Value};
use std::collections::BTreeMap;
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
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
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::batch_transition::batched_transition::document_create_transition::DocumentCreateTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_replace_transition::DocumentReplaceTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransitionV0;
    use crate::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use platform_value::Value;

    fn make_base() -> DocumentBaseTransition {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::default(),
            identity_contract_nonce: 1,
            document_type_name: "test_doc".to_string(),
            data_contract_id: Identifier::default(),
        })
    }

    fn make_create_transition(data: BTreeMap<String, Value>) -> DocumentTransition {
        DocumentTransition::Create(DocumentCreateTransition::V0(DocumentCreateTransitionV0 {
            base: make_base(),
            entropy: [0u8; 32],
            data,
            prefunded_voting_balance: None,
        }))
    }

    fn make_replace_transition(data: BTreeMap<String, Value>) -> DocumentTransition {
        DocumentTransition::Replace(DocumentReplaceTransition::V0(DocumentReplaceTransitionV0 {
            base: make_base(),
            revision: 2,
            data,
        }))
    }

    fn make_delete_transition() -> DocumentTransition {
        DocumentTransition::Delete(DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
            base: make_base(),
        }))
    }

    fn make_transfer_transition() -> DocumentTransition {
        DocumentTransition::Transfer(DocumentTransferTransition::V0(
            DocumentTransferTransitionV0 {
                base: make_base(),
                revision: 3,
                recipient_owner_id: Identifier::from([5u8; 32]),
            },
        ))
    }

    fn make_update_price_transition() -> DocumentTransition {
        DocumentTransition::UpdatePrice(DocumentUpdatePriceTransition::V0(
            DocumentUpdatePriceTransitionV0 {
                base: make_base(),
                revision: 4,
                price: 100,
            },
        ))
    }

    fn make_purchase_transition() -> DocumentTransition {
        DocumentTransition::Purchase(DocumentPurchaseTransition::V0(
            DocumentPurchaseTransitionV0 {
                base: make_base(),
                revision: 5,
                price: 200,
            },
        ))
    }

    // -----------------------------------------------------------------------
    // get_dynamic_property
    // -----------------------------------------------------------------------

    #[test]
    fn get_dynamic_property_returns_value_for_create() {
        let mut data = BTreeMap::new();
        data.insert("myField".to_string(), Value::Text("hello".to_string()));
        let transition = make_create_transition(data);

        let result = transition.get_dynamic_property("myField");
        assert_eq!(result, Some(&Value::Text("hello".to_string())));
    }

    #[test]
    fn get_dynamic_property_returns_value_for_replace() {
        let mut data = BTreeMap::new();
        data.insert("count".to_string(), Value::U64(42));
        let transition = make_replace_transition(data);

        let result = transition.get_dynamic_property("count");
        assert_eq!(result, Some(&Value::U64(42)));
    }

    #[test]
    fn get_dynamic_property_returns_none_for_missing_key_on_create() {
        let transition = make_create_transition(BTreeMap::new());
        assert!(transition.get_dynamic_property("nonexistent").is_none());
    }

    #[test]
    fn get_dynamic_property_returns_none_for_delete() {
        let transition = make_delete_transition();
        assert!(transition.get_dynamic_property("anything").is_none());
    }

    #[test]
    fn get_dynamic_property_returns_none_for_transfer() {
        let transition = make_transfer_transition();
        assert!(transition.get_dynamic_property("anything").is_none());
    }

    #[test]
    fn get_dynamic_property_returns_none_for_update_price() {
        let transition = make_update_price_transition();
        assert!(transition.get_dynamic_property("anything").is_none());
    }

    #[test]
    fn get_dynamic_property_returns_none_for_purchase() {
        let transition = make_purchase_transition();
        assert!(transition.get_dynamic_property("anything").is_none());
    }

    // -----------------------------------------------------------------------
    // entropy
    // -----------------------------------------------------------------------

    #[test]
    fn entropy_returns_some_for_create() {
        let transition = make_create_transition(BTreeMap::new());
        let entropy = transition.entropy();
        assert!(entropy.is_some());
        assert_eq!(entropy.unwrap().len(), 32);
    }

    #[test]
    fn entropy_returns_none_for_replace() {
        let transition = make_replace_transition(BTreeMap::new());
        assert!(transition.entropy().is_none());
    }

    #[test]
    fn entropy_returns_none_for_delete() {
        let transition = make_delete_transition();
        assert!(transition.entropy().is_none());
    }

    #[test]
    fn entropy_returns_none_for_transfer() {
        let transition = make_transfer_transition();
        assert!(transition.entropy().is_none());
    }

    #[test]
    fn entropy_returns_none_for_update_price() {
        let transition = make_update_price_transition();
        assert!(transition.entropy().is_none());
    }

    #[test]
    fn entropy_returns_none_for_purchase() {
        let transition = make_purchase_transition();
        assert!(transition.entropy().is_none());
    }

    // -----------------------------------------------------------------------
    // data
    // -----------------------------------------------------------------------

    #[test]
    fn data_returns_some_for_create() {
        let mut d = BTreeMap::new();
        d.insert("key".to_string(), Value::Bool(true));
        let transition = make_create_transition(d.clone());
        assert_eq!(transition.data(), Some(&d));
    }

    #[test]
    fn data_returns_some_for_replace() {
        let mut d = BTreeMap::new();
        d.insert("key2".to_string(), Value::U64(99));
        let transition = make_replace_transition(d.clone());
        assert_eq!(transition.data(), Some(&d));
    }

    #[test]
    fn data_returns_none_for_delete() {
        assert!(make_delete_transition().data().is_none());
    }

    #[test]
    fn data_returns_none_for_transfer() {
        assert!(make_transfer_transition().data().is_none());
    }

    #[test]
    fn data_returns_none_for_update_price() {
        assert!(make_update_price_transition().data().is_none());
    }

    #[test]
    fn data_returns_none_for_purchase() {
        assert!(make_purchase_transition().data().is_none());
    }

    // -----------------------------------------------------------------------
    // revision
    // -----------------------------------------------------------------------

    #[test]
    fn revision_returns_1_for_create() {
        let transition = make_create_transition(BTreeMap::new());
        assert_eq!(transition.revision(), Some(1));
    }

    #[test]
    fn revision_returns_value_for_replace() {
        let transition = make_replace_transition(BTreeMap::new());
        assert_eq!(transition.revision(), Some(2));
    }

    #[test]
    fn revision_returns_none_for_delete() {
        assert!(make_delete_transition().revision().is_none());
    }

    #[test]
    fn revision_returns_value_for_transfer() {
        let transition = make_transfer_transition();
        assert_eq!(transition.revision(), Some(3));
    }

    #[test]
    fn revision_returns_value_for_update_price() {
        let transition = make_update_price_transition();
        assert_eq!(transition.revision(), Some(4));
    }

    #[test]
    fn revision_returns_value_for_purchase() {
        let transition = make_purchase_transition();
        assert_eq!(transition.revision(), Some(5));
    }

    // -----------------------------------------------------------------------
    // insert_dynamic_property (cfg(test) only)
    // -----------------------------------------------------------------------

    #[test]
    fn insert_dynamic_property_works_on_create() {
        let mut transition = make_create_transition(BTreeMap::new());
        transition.insert_dynamic_property("added".to_string(), Value::Bool(true));
        assert_eq!(
            transition.get_dynamic_property("added"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn insert_dynamic_property_works_on_replace() {
        let mut transition = make_replace_transition(BTreeMap::new());
        transition.insert_dynamic_property("added".to_string(), Value::U64(7));
        assert_eq!(
            transition.get_dynamic_property("added"),
            Some(&Value::U64(7))
        );
    }

    #[test]
    fn insert_dynamic_property_is_noop_on_delete() {
        let mut transition = make_delete_transition();
        transition.insert_dynamic_property("added".to_string(), Value::Bool(true));
        // Should still return None because delete has no data
        assert!(transition.get_dynamic_property("added").is_none());
    }

    // -----------------------------------------------------------------------
    // data_mut
    // -----------------------------------------------------------------------

    #[test]
    fn data_mut_returns_some_for_create_and_replace() {
        let mut create = make_create_transition(BTreeMap::new());
        assert!(create.data_mut().is_some());

        let mut replace = make_replace_transition(BTreeMap::new());
        assert!(replace.data_mut().is_some());
    }

    #[test]
    fn data_mut_returns_none_for_other_variants() {
        let mut delete = make_delete_transition();
        assert!(delete.data_mut().is_none());

        let mut transfer = make_transfer_transition();
        assert!(transfer.data_mut().is_none());

        let mut update_price = make_update_price_transition();
        assert!(update_price.data_mut().is_none());

        let mut purchase = make_purchase_transition();
        assert!(purchase.data_mut().is_none());
    }
}
