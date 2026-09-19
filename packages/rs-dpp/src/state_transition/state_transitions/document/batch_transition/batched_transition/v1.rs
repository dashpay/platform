use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::{Display, From};
use platform_value::{Identifier, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::prelude::{IdentityNonce, Revision};
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::{
    BatchedTransition, DocumentCreateTransition, DocumentDeleteTransition,
    DocumentEraseTransition, DocumentIndexOnlyDeleteTransition, DocumentPurchaseTransition,
    DocumentReplaceTransition, DocumentTransferTransition, DocumentTransition,
    DocumentUpdatePriceTransition, TokenTransition,
};
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::v0::v0_methods::DocumentIndexOnlyDeleteTransitionV0Methods;
use crate::state_transition::batch_transition::resolvers::v0::BatchTransitionResolversV0;
use crate::state_transition::batch_transition::{
    TokenBurnTransition, TokenClaimTransition, TokenConfigUpdateTransition,
    TokenDestroyFrozenFundsTransition, TokenDirectPurchaseTransition,
    TokenEmergencyActionTransition, TokenFreezeTransition, TokenMintTransition,
    TokenSetPriceForDirectPurchaseTransition, TokenTransferTransition, TokenUnfreezeTransition,
};

/// The document transition shell carried by batch transition format 2.
///
/// Batch formats 0 and 1 carry [`DocumentTransition`], whose wire discriminants
/// end at the indexOnly delete kind. Format 2 owns this shell so that the erase
/// kind can exist without changing what a format 0 or 1 batch can decode:
/// software that only knows the older formats keeps rejecting an erase at
/// decode time, exactly as software that predates the kind does. The concrete
/// per-kind payloads are shared with [`DocumentTransition`]; only the two shell
/// enums differ.
#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    // The same `$action` discriminator and variant names as
    // `DocumentTransition`, so a format 2 batch has the same external shape
    // for every kind the older formats already carried.
    serde(tag = "$action", rename_all = "camelCase")
)]
pub enum DocumentTransitionV1 {
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

    #[display("IndexOnlyDeleteDocumentTransition({})", "_0")]
    IndexOnlyDelete(DocumentIndexOnlyDeleteTransition),

    /// The erase kind, which purges the retained revisions of a deleted
    /// keep-history document. It only exists in this shell.
    #[display("EraseDocumentTransition({})", "_0")]
    Erase(DocumentEraseTransition),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentTransitionV1 {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentTransitionV1 {}

impl From<DocumentTransition> for DocumentTransitionV1 {
    fn from(value: DocumentTransition) -> Self {
        match value {
            DocumentTransition::Create(transition) => Self::Create(transition),
            DocumentTransition::Replace(transition) => Self::Replace(transition),
            DocumentTransition::Delete(transition) => Self::Delete(transition),
            DocumentTransition::Transfer(transition) => Self::Transfer(transition),
            DocumentTransition::UpdatePrice(transition) => Self::UpdatePrice(transition),
            DocumentTransition::Purchase(transition) => Self::Purchase(transition),
            DocumentTransition::IndexOnlyDelete(transition) => Self::IndexOnlyDelete(transition),
        }
    }
}

impl<'a> From<&'a DocumentTransition> for DocumentTransitionRefV1<'a> {
    fn from(value: &'a DocumentTransition) -> Self {
        match value {
            DocumentTransition::Create(transition) => Self::Create(transition),
            DocumentTransition::Replace(transition) => Self::Replace(transition),
            DocumentTransition::Delete(transition) => Self::Delete(transition),
            DocumentTransition::Transfer(transition) => Self::Transfer(transition),
            DocumentTransition::UpdatePrice(transition) => Self::UpdatePrice(transition),
            DocumentTransition::Purchase(transition) => Self::Purchase(transition),
            DocumentTransition::IndexOnlyDelete(transition) => Self::IndexOnlyDelete(transition),
        }
    }
}

impl<'a> From<&'a mut DocumentTransition> for DocumentTransitionMutRefV1<'a> {
    fn from(value: &'a mut DocumentTransition) -> Self {
        match value {
            DocumentTransition::Create(transition) => Self::Create(transition),
            DocumentTransition::Replace(transition) => Self::Replace(transition),
            DocumentTransition::Delete(transition) => Self::Delete(transition),
            DocumentTransition::Transfer(transition) => Self::Transfer(transition),
            DocumentTransition::UpdatePrice(transition) => Self::UpdatePrice(transition),
            DocumentTransition::Purchase(transition) => Self::Purchase(transition),
            DocumentTransition::IndexOnlyDelete(transition) => Self::IndexOnlyDelete(transition),
        }
    }
}

impl TryFrom<DocumentTransitionV1> for DocumentTransition {
    type Error = ProtocolError;

    /// Fails for an erase: no batch format before 2 can carry one.
    fn try_from(value: DocumentTransitionV1) -> Result<Self, Self::Error> {
        Ok(match value {
            DocumentTransitionV1::Create(transition) => Self::Create(transition),
            DocumentTransitionV1::Replace(transition) => Self::Replace(transition),
            DocumentTransitionV1::Delete(transition) => Self::Delete(transition),
            DocumentTransitionV1::Transfer(transition) => Self::Transfer(transition),
            DocumentTransitionV1::UpdatePrice(transition) => Self::UpdatePrice(transition),
            DocumentTransitionV1::Purchase(transition) => Self::Purchase(transition),
            DocumentTransitionV1::IndexOnlyDelete(transition) => Self::IndexOnlyDelete(transition),
            DocumentTransitionV1::Erase(_) => {
                return Err(ProtocolError::Generic(
                    "an erase transition only exists in batch transition format 2".to_string(),
                ))
            }
        })
    }
}

/// The batched transition shell carried by batch transition format 2.
///
/// It differs from [`BatchedTransition`] only in the document shell it wraps,
/// which is the one that knows the erase kind.
#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    // The same `$transition` discriminator as `BatchedTransition`, so the
    // external shape of a format 2 batch matches the older formats.
    serde(tag = "$transition", rename_all = "camelCase")
)]
pub enum BatchedTransitionV1 {
    #[display("DocumentTransition({})", "_0")]
    Document(DocumentTransitionV1),
    #[display("TokenTransition({})", "_0")]
    Token(TokenTransition),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for BatchedTransitionV1 {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for BatchedTransitionV1 {}

impl From<BatchedTransition> for BatchedTransitionV1 {
    fn from(value: BatchedTransition) -> Self {
        match value {
            BatchedTransition::Document(transition) => Self::Document(transition.into()),
            BatchedTransition::Token(transition) => Self::Token(transition),
        }
    }
}

impl<'a> From<&'a BatchedTransition> for BatchedTransitionRefV1<'a> {
    fn from(value: &'a BatchedTransition) -> Self {
        match value {
            BatchedTransition::Document(transition) => Self::Document(transition.into()),
            BatchedTransition::Token(transition) => Self::Token(transition),
        }
    }
}

impl<'a> From<&'a mut BatchedTransition> for BatchedTransitionMutRefV1<'a> {
    fn from(value: &'a mut BatchedTransition) -> Self {
        match value {
            BatchedTransition::Document(transition) => Self::Document(transition.into()),
            BatchedTransition::Token(transition) => Self::Token(transition),
        }
    }
}

impl TryFrom<BatchedTransitionV1> for BatchedTransition {
    type Error = ProtocolError;

    /// Fails for an erase: no batch format before 2 can carry one.
    fn try_from(value: BatchedTransitionV1) -> Result<Self, Self::Error> {
        Ok(match value {
            BatchedTransitionV1::Document(transition) => Self::Document(transition.try_into()?),
            BatchedTransitionV1::Token(transition) => Self::Token(transition),
        })
    }
}

/// A borrowed view of one document transition of any batch format.
///
/// Every batch format can be viewed through it: formats 0 and 1 convert their
/// [`DocumentTransition`] items, format 2 borrows its own shell.
#[derive(Debug, Clone, Copy, PartialEq, Display)]
pub enum DocumentTransitionRefV1<'a> {
    #[display("CreateDocumentTransition({})", "_0")]
    Create(&'a DocumentCreateTransition),
    #[display("ReplaceDocumentTransition({})", "_0")]
    Replace(&'a DocumentReplaceTransition),
    #[display("DeleteDocumentTransition({})", "_0")]
    Delete(&'a DocumentDeleteTransition),
    #[display("TransferDocumentTransition({})", "_0")]
    Transfer(&'a DocumentTransferTransition),
    #[display("UpdatePriceDocumentTransition({})", "_0")]
    UpdatePrice(&'a DocumentUpdatePriceTransition),
    #[display("PurchaseDocumentTransition({})", "_0")]
    Purchase(&'a DocumentPurchaseTransition),
    #[display("IndexOnlyDeleteDocumentTransition({})", "_0")]
    IndexOnlyDelete(&'a DocumentIndexOnlyDeleteTransition),
    #[display("EraseDocumentTransition({})", "_0")]
    Erase(&'a DocumentEraseTransition),
}

/// A mutable view of one document transition of any batch format.
#[derive(Debug, PartialEq, Display)]
pub enum DocumentTransitionMutRefV1<'a> {
    #[display("CreateDocumentTransition({})", "_0")]
    Create(&'a mut DocumentCreateTransition),
    #[display("ReplaceDocumentTransition({})", "_0")]
    Replace(&'a mut DocumentReplaceTransition),
    #[display("DeleteDocumentTransition({})", "_0")]
    Delete(&'a mut DocumentDeleteTransition),
    #[display("TransferDocumentTransition({})", "_0")]
    Transfer(&'a mut DocumentTransferTransition),
    #[display("UpdatePriceDocumentTransition({})", "_0")]
    UpdatePrice(&'a mut DocumentUpdatePriceTransition),
    #[display("PurchaseDocumentTransition({})", "_0")]
    Purchase(&'a mut DocumentPurchaseTransition),
    #[display("IndexOnlyDeleteDocumentTransition({})", "_0")]
    IndexOnlyDelete(&'a mut DocumentIndexOnlyDeleteTransition),
    #[display("EraseDocumentTransition({})", "_0")]
    Erase(&'a mut DocumentEraseTransition),
}

impl DocumentTransitionV1 {
    pub fn borrow_as_ref(&self) -> DocumentTransitionRefV1<'_> {
        match self {
            Self::Create(transition) => DocumentTransitionRefV1::Create(transition),
            Self::Replace(transition) => DocumentTransitionRefV1::Replace(transition),
            Self::Delete(transition) => DocumentTransitionRefV1::Delete(transition),
            Self::Transfer(transition) => DocumentTransitionRefV1::Transfer(transition),
            Self::UpdatePrice(transition) => DocumentTransitionRefV1::UpdatePrice(transition),
            Self::Purchase(transition) => DocumentTransitionRefV1::Purchase(transition),
            Self::IndexOnlyDelete(transition) => {
                DocumentTransitionRefV1::IndexOnlyDelete(transition)
            }
            Self::Erase(transition) => DocumentTransitionRefV1::Erase(transition),
        }
    }

    pub fn borrow_as_mut(&mut self) -> DocumentTransitionMutRefV1<'_> {
        match self {
            Self::Create(transition) => DocumentTransitionMutRefV1::Create(transition),
            Self::Replace(transition) => DocumentTransitionMutRefV1::Replace(transition),
            Self::Delete(transition) => DocumentTransitionMutRefV1::Delete(transition),
            Self::Transfer(transition) => DocumentTransitionMutRefV1::Transfer(transition),
            Self::UpdatePrice(transition) => DocumentTransitionMutRefV1::UpdatePrice(transition),
            Self::Purchase(transition) => DocumentTransitionMutRefV1::Purchase(transition),
            Self::IndexOnlyDelete(transition) => {
                DocumentTransitionMutRefV1::IndexOnlyDelete(transition)
            }
            Self::Erase(transition) => DocumentTransitionMutRefV1::Erase(transition),
        }
    }

    /// The erase transition, when this is one.
    pub fn as_transition_erase(&self) -> Option<&DocumentEraseTransition> {
        match self {
            Self::Erase(transition) => Some(transition),
            _ => None,
        }
    }
}

/// A borrowed view of one batched transition of any batch format.
#[derive(Debug, Clone, Copy, PartialEq, Display)]
pub enum BatchedTransitionRefV1<'a> {
    #[display("DocumentTransition({})", "_0")]
    Document(DocumentTransitionRefV1<'a>),
    #[display("TokenTransition({})", "_0")]
    Token(&'a TokenTransition),
}

/// A mutable view of one batched transition of any batch format.
#[derive(Debug, PartialEq, Display)]
pub enum BatchedTransitionMutRefV1<'a> {
    #[display("DocumentTransition({})", "_0")]
    Document(DocumentTransitionMutRefV1<'a>),
    #[display("TokenTransition({})", "_0")]
    Token(&'a mut TokenTransition),
}

impl BatchedTransitionV1 {
    pub fn borrow_as_ref(&self) -> BatchedTransitionRefV1<'_> {
        match self {
            Self::Document(transition) => {
                BatchedTransitionRefV1::Document(transition.borrow_as_ref())
            }
            Self::Token(transition) => BatchedTransitionRefV1::Token(transition),
        }
    }

    pub fn borrow_as_mut(&mut self) -> BatchedTransitionMutRefV1<'_> {
        match self {
            Self::Document(transition) => {
                BatchedTransitionMutRefV1::Document(transition.borrow_as_mut())
            }
            Self::Token(transition) => BatchedTransitionMutRefV1::Token(transition),
        }
    }

    pub fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        match self {
            Self::Document(transition) => {
                transition.set_identity_contract_nonce(identity_contract_nonce)
            }
            Self::Token(transition) => {
                transition.set_identity_contract_nonce(identity_contract_nonce)
            }
        }
    }
}

impl<'a> BatchedTransitionRefV1<'a> {
    pub fn to_owned_transition(&self) -> BatchedTransitionV1 {
        match self {
            Self::Document(transition) => {
                BatchedTransitionV1::Document(transition.to_owned_transition())
            }
            Self::Token(transition) => BatchedTransitionV1::Token((*transition).clone()),
        }
    }

    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            Self::Document(transition) => transition.identity_contract_nonce(),
            Self::Token(transition) => transition.identity_contract_nonce(),
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        match self {
            Self::Document(transition) => transition.data_contract_id(),
            Self::Token(transition) => transition.data_contract_id(),
        }
    }

    /// The erase transition, when this is one.
    pub fn as_transition_erase(&self) -> Option<&'a DocumentEraseTransition> {
        match self {
            Self::Document(DocumentTransitionRefV1::Erase(transition)) => Some(transition),
            _ => None,
        }
    }
}

impl<'a> DocumentTransitionRefV1<'a> {
    pub fn to_owned_transition(&self) -> DocumentTransitionV1 {
        match self {
            Self::Create(transition) => DocumentTransitionV1::Create((*transition).clone()),
            Self::Replace(transition) => DocumentTransitionV1::Replace((*transition).clone()),
            Self::Delete(transition) => DocumentTransitionV1::Delete((*transition).clone()),
            Self::Transfer(transition) => DocumentTransitionV1::Transfer((*transition).clone()),
            Self::UpdatePrice(transition) => {
                DocumentTransitionV1::UpdatePrice((*transition).clone())
            }
            Self::Purchase(transition) => DocumentTransitionV1::Purchase((*transition).clone()),
            Self::IndexOnlyDelete(transition) => {
                DocumentTransitionV1::IndexOnlyDelete((*transition).clone())
            }
            Self::Erase(transition) => DocumentTransitionV1::Erase((*transition).clone()),
        }
    }

    pub fn base(&self) -> &'a DocumentBaseTransition {
        match self {
            Self::Create(transition) => transition.base(),
            Self::Replace(transition) => transition.base(),
            Self::Delete(transition) => transition.base(),
            Self::Transfer(transition) => transition.base(),
            Self::UpdatePrice(transition) => transition.base(),
            Self::Purchase(transition) => transition.base(),
            Self::IndexOnlyDelete(transition) => transition.base(),
            Self::Erase(transition) => transition.base(),
        }
    }

    pub fn get_id(&self) -> Identifier {
        self.base().id()
    }

    pub fn document_type_name(&self) -> &'a String {
        self.base().document_type_name()
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }

    pub fn entropy(&self) -> Option<Vec<u8>> {
        match self {
            Self::Create(transition) => Some(Vec::from(transition.entropy())),
            _ => None,
        }
    }

    /// The document data the transition carries, when its kind carries any.
    pub fn data(&self) -> Option<&'a BTreeMap<String, Value>> {
        match self {
            Self::Create(transition) => Some(transition.data()),
            Self::Replace(transition) => Some(transition.data()),
            Self::IndexOnlyDelete(transition) => Some(transition.data()),
            Self::Delete(_)
            | Self::Transfer(_)
            | Self::UpdatePrice(_)
            | Self::Purchase(_)
            | Self::Erase(_) => None,
        }
    }

    pub fn get_dynamic_property(&self, path: &str) -> Option<&'a Value> {
        self.data()?.get(path)
    }

    pub fn first_data_depth_exceeding(&self, max_depth: usize) -> Option<usize> {
        self.data()?
            .values()
            .find_map(|value| value.first_depth_exceeding(max_depth))
    }

    pub fn revision(&self) -> Option<Revision> {
        match self {
            Self::Create(_) => Some(1),
            Self::Replace(transition) => Some(transition.revision()),
            Self::Transfer(transition) => Some(transition.revision()),
            Self::UpdatePrice(transition) => Some(transition.revision()),
            Self::Purchase(transition) => Some(transition.revision()),
            Self::Delete(_) | Self::IndexOnlyDelete(_) | Self::Erase(_) => None,
        }
    }

    pub fn as_transition_create(&self) -> Option<&'a DocumentCreateTransition> {
        match self {
            Self::Create(transition) => Some(transition),
            _ => None,
        }
    }

    pub fn as_transition_replace(&self) -> Option<&'a DocumentReplaceTransition> {
        match self {
            Self::Replace(transition) => Some(transition),
            _ => None,
        }
    }

    pub fn as_transition_delete(&self) -> Option<&'a DocumentDeleteTransition> {
        match self {
            Self::Delete(transition) => Some(transition),
            _ => None,
        }
    }

    pub fn as_transition_transfer(&self) -> Option<&'a DocumentTransferTransition> {
        match self {
            Self::Transfer(transition) => Some(transition),
            _ => None,
        }
    }

    pub fn as_transition_purchase(&self) -> Option<&'a DocumentPurchaseTransition> {
        match self {
            Self::Purchase(transition) => Some(transition),
            _ => None,
        }
    }

    pub fn as_transition_erase(&self) -> Option<&'a DocumentEraseTransition> {
        match self {
            Self::Erase(transition) => Some(transition),
            _ => None,
        }
    }
}

impl DocumentTransitionV0Methods for DocumentTransitionV1 {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            Self::Create(transition) => transition.base(),
            Self::Replace(transition) => transition.base(),
            Self::Delete(transition) => transition.base(),
            Self::Transfer(transition) => transition.base(),
            Self::UpdatePrice(transition) => transition.base(),
            Self::Purchase(transition) => transition.base(),
            Self::IndexOnlyDelete(transition) => transition.base(),
            Self::Erase(transition) => transition.base(),
        }
    }

    fn get_dynamic_property(&self, path: &str) -> Option<&Value> {
        self.data()?.get(path)
    }

    fn get_id(&self) -> Identifier {
        self.base().id()
    }

    fn entropy(&self) -> Option<Vec<u8>> {
        match self {
            Self::Create(transition) => Some(Vec::from(transition.entropy())),
            _ => None,
        }
    }

    fn document_type_name(&self) -> &String {
        self.base().document_type_name()
    }

    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    fn data(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Create(transition) => Some(transition.data()),
            Self::Replace(transition) => Some(transition.data()),
            Self::IndexOnlyDelete(transition) => Some(transition.data()),
            Self::Delete(_)
            | Self::Transfer(_)
            | Self::UpdatePrice(_)
            | Self::Purchase(_)
            | Self::Erase(_) => None,
        }
    }

    fn first_data_depth_exceeding(&self, max_depth: usize) -> Option<usize> {
        self.borrow_as_ref().first_data_depth_exceeding(max_depth)
    }

    fn revision(&self) -> Option<Revision> {
        self.borrow_as_ref().revision()
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }

    #[cfg(test)]
    fn insert_dynamic_property(&mut self, property_name: String, value: Value) {
        if let Some(data) = self.data_mut() {
            data.insert(property_name, value);
        }
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        self.base_mut().set_data_contract_id(id)
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            Self::Create(transition) => transition.base_mut(),
            Self::Replace(transition) => transition.base_mut(),
            Self::Delete(transition) => transition.base_mut(),
            Self::Transfer(transition) => transition.base_mut(),
            Self::UpdatePrice(transition) => transition.base_mut(),
            Self::Purchase(transition) => transition.base_mut(),
            Self::IndexOnlyDelete(transition) => transition.base_mut(),
            Self::Erase(transition) => transition.base_mut(),
        }
    }

    fn data_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        match self {
            Self::Create(transition) => Some(transition.data_mut()),
            Self::Replace(transition) => Some(transition.data_mut()),
            Self::IndexOnlyDelete(transition) => Some(transition.data_mut()),
            Self::Delete(_)
            | Self::Transfer(_)
            | Self::UpdatePrice(_)
            | Self::Purchase(_)
            | Self::Erase(_) => None,
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            Self::Replace(transition) => transition.set_revision(revision),
            Self::Transfer(transition) => transition.set_revision(revision),
            Self::UpdatePrice(transition) => transition.set_revision(revision),
            Self::Purchase(transition) => transition.set_revision(revision),
            Self::Create(_) | Self::Delete(_) | Self::IndexOnlyDelete(_) | Self::Erase(_) => {}
        }
    }

    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        self.base_mut().set_identity_contract_nonce(nonce)
    }
}

impl BatchTransitionResolversV0 for BatchedTransitionV1 {
    fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        match self {
            Self::Document(DocumentTransitionV1::Create(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        match self {
            Self::Document(DocumentTransitionV1::Replace(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        match self {
            Self::Document(DocumentTransitionV1::Delete(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_transfer(&self) -> Option<&DocumentTransferTransition> {
        match self {
            Self::Document(DocumentTransitionV1::Transfer(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_purchase(&self) -> Option<&DocumentPurchaseTransition> {
        match self {
            Self::Document(DocumentTransitionV1::Purchase(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_token_burn(&self) -> Option<&TokenBurnTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_burn(),
        }
    }

    fn as_transition_token_mint(&self) -> Option<&TokenMintTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_mint(),
        }
    }

    fn as_transition_token_transfer(&self) -> Option<&TokenTransferTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_transfer(),
        }
    }

    fn as_transition_token_freeze(&self) -> Option<&TokenFreezeTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_freeze(),
        }
    }

    fn as_transition_token_unfreeze(&self) -> Option<&TokenUnfreezeTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_unfreeze(),
        }
    }

    fn as_transition_token_destroy_frozen_funds(
        &self,
    ) -> Option<&TokenDestroyFrozenFundsTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_destroy_frozen_funds(),
        }
    }

    fn as_transition_token_claim(&self) -> Option<&TokenClaimTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_claim(),
        }
    }

    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_emergency_action(),
        }
    }

    fn as_transition_token_config_update(&self) -> Option<&TokenConfigUpdateTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_config_update(),
        }
    }

    fn as_transition_token_direct_purchase(&self) -> Option<&TokenDirectPurchaseTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_direct_purchase(),
        }
    }

    fn as_transition_token_set_price_for_direct_purchase(
        &self,
    ) -> Option<&TokenSetPriceForDirectPurchaseTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_set_price_for_direct_purchase(),
        }
    }
}

impl BatchTransitionResolversV0 for BatchedTransitionRefV1<'_> {
    fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        match self {
            Self::Document(DocumentTransitionRefV1::Create(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        match self {
            Self::Document(DocumentTransitionRefV1::Replace(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        match self {
            Self::Document(DocumentTransitionRefV1::Delete(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_transfer(&self) -> Option<&DocumentTransferTransition> {
        match self {
            Self::Document(DocumentTransitionRefV1::Transfer(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_purchase(&self) -> Option<&DocumentPurchaseTransition> {
        match self {
            Self::Document(DocumentTransitionRefV1::Purchase(transition)) => Some(transition),
            _ => None,
        }
    }

    fn as_transition_token_burn(&self) -> Option<&TokenBurnTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_burn(),
        }
    }

    fn as_transition_token_mint(&self) -> Option<&TokenMintTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_mint(),
        }
    }

    fn as_transition_token_transfer(&self) -> Option<&TokenTransferTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_transfer(),
        }
    }

    fn as_transition_token_freeze(&self) -> Option<&TokenFreezeTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_freeze(),
        }
    }

    fn as_transition_token_unfreeze(&self) -> Option<&TokenUnfreezeTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_unfreeze(),
        }
    }

    fn as_transition_token_destroy_frozen_funds(
        &self,
    ) -> Option<&TokenDestroyFrozenFundsTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_destroy_frozen_funds(),
        }
    }

    fn as_transition_token_claim(&self) -> Option<&TokenClaimTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_claim(),
        }
    }

    fn as_transition_token_emergency_action(&self) -> Option<&TokenEmergencyActionTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_emergency_action(),
        }
    }

    fn as_transition_token_config_update(&self) -> Option<&TokenConfigUpdateTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_config_update(),
        }
    }

    fn as_transition_token_direct_purchase(&self) -> Option<&TokenDirectPurchaseTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_direct_purchase(),
        }
    }

    fn as_transition_token_set_price_for_direct_purchase(
        &self,
    ) -> Option<&TokenSetPriceForDirectPurchaseTransition> {
        match self {
            Self::Document(_) => None,
            Self::Token(token) => token.as_transition_token_set_price_for_direct_purchase(),
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::serialization::{JsonConvertible, ValueConvertible};
    use crate::state_transition::batch_transition::batched_transition::{
        document_create_transition, document_erase_transition,
    };

    fn assert_document_round_trip(transition: DocumentTransitionV1, expected_action: &str) {
        let json = transition.to_json().expect("to_json");
        let json_obj = json.as_object().expect("json object");
        assert_eq!(
            json_obj.get("$action").and_then(|v| v.as_str()),
            Some(expected_action),
            "json `$action` discriminator mismatch"
        );
        let recovered_json = DocumentTransitionV1::from_json(json).expect("from_json");
        assert_eq!(transition, recovered_json);

        let value = transition.to_object().expect("to_object");
        let recovered_value = DocumentTransitionV1::from_object(value).expect("from_object");
        assert_eq!(transition, recovered_value);
    }

    #[test]
    fn should_keep_the_external_shape_of_a_legacy_kind_in_the_format_2_shell() {
        let create = document_create_transition::json_convertible_tests::fixture();
        let legacy = DocumentTransition::Create(create.clone());
        let format_2 = DocumentTransitionV1::Create(create);

        assert_eq!(
            legacy.to_json().expect("legacy to_json"),
            format_2.to_json().expect("format 2 to_json"),
            "a kind shared by every batch format must serialize identically"
        );
        assert_document_round_trip(format_2, "create");
    }

    #[test]
    fn should_round_trip_an_erase_through_json_and_value() {
        let erase = document_erase_transition::json_convertible_tests::fixture();
        assert_document_round_trip(DocumentTransitionV1::Erase(erase), "erase");
    }

    #[test]
    fn should_tag_the_batched_shell_with_the_transition_kind() {
        let erase = document_erase_transition::json_convertible_tests::fixture();
        let batched = BatchedTransitionV1::Document(DocumentTransitionV1::Erase(erase));

        let json = batched.to_json().expect("to_json");
        let json_obj = json.as_object().expect("json object");
        assert_eq!(
            json_obj.get("$transition").and_then(|v| v.as_str()),
            Some("document")
        );
        assert_eq!(
            json_obj.get("$action").and_then(|v| v.as_str()),
            Some("erase")
        );
        let recovered = BatchedTransitionV1::from_json(json).expect("from_json");
        assert_eq!(batched, recovered);
    }

    #[test]
    fn should_refuse_an_erase_in_the_shell_of_the_older_batch_formats() {
        let erase = document_erase_transition::json_convertible_tests::fixture();
        let json = DocumentTransitionV1::Erase(erase)
            .to_json()
            .expect("to_json");

        assert!(
            DocumentTransition::from_json(json).is_err(),
            "the shell used by batch formats 0 and 1 has no erase kind"
        );
    }
}
