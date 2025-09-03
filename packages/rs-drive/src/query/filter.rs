use std::collections::BTreeMap;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use crate::query::{DriveDocumentQuery, InternalClauses};

#[cfg(any(feature = "server", feature = "verify"))]
/// Drive query struct
#[derive(Debug, PartialEq, Clone)]
pub struct DriveDocumentQueryFilter<'a> {
    ///DataContract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
    /// Internal clauses
    pub internal_clauses: InternalClauses,
}

impl From<DriveDocumentQueryFilter> for DriveDocumentQuery {
    fn from(value: DriveDocumentQueryFilter) -> Self {
        todo!()
    }
}

impl From<DriveDocumentQuery> for DriveDocumentQueryFilter {
    fn from(value: DriveDocumentQuery) -> Self {
        todo!()
    }
}

impl<'a> DriveDocumentQueryFilter<'a> {
    /// Figures out if a document matches the query
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_state_transition(&self, state_transition: &StateTransition) -> bool {
        match state_transition {
            StateTransition::Batch(batch) => {
                for transition in batch.transitions_iter() {
                    if let BatchedTransitionRef::Document(document_transition) = transition {
                        if self.matches_document_state_transition(batch.owner_id(), document_transition) {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document_state_transition(&self, owner_id: Identifier, document_transition: &DocumentTransition) -> bool {
        match document_transition {
            DocumentTransition::Create(create) => {
                self.matches_document(owner_id, create.base(), create.data())
            }
            DocumentTransition::Replace(replace) => {
                self.matches_document(owner_id, replace.base(), replace.data())
            }
            DocumentTransition::Delete(_) => {}
            DocumentTransition::Transfer(_) => {}
            DocumentTransition::UpdatePrice(_) => {}
            DocumentTransition::Purchase(_) => {}
        }
    }
    /// Figures out if a document matches the query
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn matches_document(&self, owner_id: Identifier, document_base_transition: &DocumentBaseTransition, document_data: &BTreeMap<String, Value>) -> bool {
        if document_base_transition.data_contract_id() != self.contract.id() {
            return false;
        }
        if document_base_transition.document_type_name() != self.document_type.name() {
            return false;
        }

        if let Some(primary_key_in_clause) = &self.internal_clauses.primary_key_in_clause {
            if document_base_transition.id() in primary_key_in_clause.value
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn validate(&self) -> bool {

    }
}