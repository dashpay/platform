mod v0;

use derive_more::From;
use dpp::data_contract::document_type::property_constraints::{
    AggregateRead, StoredSystemValues,
};
use crate::state_transition_action::batch::batched_transition::document_transition::property_constraint_reads::PropertyConstraintReads;
use dpp::document::Document;
use std::collections::BTreeMap;

use dpp::platform_value::Identifier;
use dpp::ProtocolError;
pub use v0::*;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use dpp::version::PlatformVersion;

/// transformer
pub mod transformer;

/// action
#[derive(Debug, Clone, From)]
pub enum DocumentUpdatePriceTransitionAction {
    /// v0
    V0(DocumentUpdatePriceTransitionActionV0),
}

impl DocumentUpdatePriceTransitionActionAccessorsV0 for DocumentUpdatePriceTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => v0.base,
        }
    }

    fn document(&self) -> &Document {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => &v0.document,
        }
    }

    fn document_owned(self) -> Document {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => v0.document,
        }
    }

    fn property_constraint_aggregates(&self) -> &BTreeMap<AggregateRead, i128> {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => {
                PropertyConstraintReads::aggregates_of(&v0.property_constraint_reads)
            }
        }
    }

    fn set_property_constraint_aggregates(&mut self, aggregates: BTreeMap<AggregateRead, i128>) {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => PropertyConstraintReads::set_aggregates(
                &mut v0.property_constraint_reads,
                aggregates,
            ),
        }
    }

    fn stored_system_values(&self) -> Option<&StoredSystemValues> {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => {
                PropertyConstraintReads::stored_system_values_of(&v0.property_constraint_reads)
            }
        }
    }

    fn set_stored_system_values(&mut self, values: StoredSystemValues) {
        match self {
            DocumentUpdatePriceTransitionAction::V0(v0) => {
                PropertyConstraintReads::set_stored_system_values(
                    &mut v0.property_constraint_reads,
                    values,
                )
            }
        }
    }
}

/// document from update price transition
pub trait DocumentFromUpdatePriceTransitionAction {
    /// Attempts to create a new `Document` from the given `DocumentUpdatePriceTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentUpdatePriceTransitionAction` containing information about the document being update_priced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_update_price_transition_action(
        document_update_price_transition_action: &DocumentUpdatePriceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentUpdatePriceTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentUpdatePriceTransitionAction` instance containing information about the document being update_priced.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_update_price_transition_action(
        document_update_price_transition_action: DocumentUpdatePriceTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
