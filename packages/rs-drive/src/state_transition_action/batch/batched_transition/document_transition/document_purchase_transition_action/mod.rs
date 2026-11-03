mod v0;

use derive_more::From;
use dpp::data_contract::document_type::property_constraints::AggregateRead;
use dpp::document::Document;
use dpp::fee::Credits;
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
pub enum DocumentPurchaseTransitionAction {
    /// v0
    V0(DocumentPurchaseTransitionActionV0),
}

/// What `property_constraint_aggregates` returns for an action holding none.
static NO_PROPERTY_CONSTRAINT_AGGREGATES: BTreeMap<AggregateRead, i128> = BTreeMap::new();

impl DocumentPurchaseTransitionActionAccessorsV0 for DocumentPurchaseTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => v0.base,
        }
    }

    fn document(&self) -> &Document {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => &v0.document,
        }
    }

    fn document_owned(self) -> Document {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => v0.document,
        }
    }

    fn original_owner_id(&self) -> Identifier {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => v0.original_owner_id,
        }
    }

    fn price(&self) -> Credits {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => v0.price,
        }
    }

    fn property_constraint_aggregates(&self) -> &BTreeMap<AggregateRead, i128> {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => v0
                .property_constraint_aggregates
                .as_deref()
                .unwrap_or(&NO_PROPERTY_CONSTRAINT_AGGREGATES),
        }
    }

    fn set_property_constraint_aggregates(&mut self, aggregates: BTreeMap<AggregateRead, i128>) {
        match self {
            DocumentPurchaseTransitionAction::V0(v0) => {
                v0.property_constraint_aggregates =
                    (!aggregates.is_empty()).then(|| Box::new(aggregates))
            }
        }
    }
}

/// document from purchase transition
pub trait DocumentFromPurchaseTransitionAction {
    /// Attempts to create a new `Document` from the given `DocumentPurchaseTransitionAction` reference and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the `DocumentPurchaseTransitionAction` containing information about the document being purchased.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_purchase_transition_action(
        document_purchase_transition_action: &DocumentPurchaseTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentPurchaseTransitionAction` instance and `owner_id`.
    ///
    /// # Arguments
    ///
    /// * `value` - A `DocumentPurchaseTransitionAction` instance containing information about the document being purchased.
    /// * `owner_id` - The `Identifier` of the document's owner.
    ///
    /// # Returns
    ///
    /// * `Result<Self, ProtocolError>` - A new `Document` object if successful, otherwise a `ProtocolError`.
    fn try_from_owned_purchase_transition_action(
        document_purchase_transition_action: DocumentPurchaseTransitionAction,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
