pub mod transformer;

use dpp::data_contract::document_type::property_constraints::{
    AggregateRead, StoredSystemValues,
};
use crate::state_transition_action::batch::batched_transition::document_transition::property_constraint_reads::PropertyConstraintReads;
use dpp::document::Document;
use std::collections::BTreeMap;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

/// document transfer transition action v0
#[derive(Debug, Clone)]
pub struct DocumentUpdatePriceTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The new document to be updated
    pub document: Document,
    /// What the document type's `propertyConstraints` rules read beyond the document,
    /// attached once the action is built ([`PropertyConstraintReads`]): the `countOf` and
    /// `sumOf` totals, each as it will be once this write is done, and the system values of
    /// the stored version it replaces, which a rule reads through `$old.`.
    pub property_constraint_reads: Option<Box<PropertyConstraintReads>>,
}

/// document transfer transition action accessors v0
pub trait DocumentUpdatePriceTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// the document with updated price to be reinserted as a ref
    fn document(&self) -> &Document;
    /// the document with updated price to be reinserted as owned
    fn document_owned(self) -> Document;

    /// The `countOf` and `sumOf` totals the rules judging this write read, each as it will
    /// be once the write is done
    fn property_constraint_aggregates(&self) -> &BTreeMap<AggregateRead, i128>;

    /// Sets the totals the rules judging this write read, once they are read from state
    fn set_property_constraint_aggregates(&mut self, aggregates: BTreeMap<AggregateRead, i128>);

    /// The owner, times, heights and revision of the stored version this write replaces,
    /// which a rule reads through `$old.`, when anything the document type declares reads them
    fn stored_system_values(&self) -> Option<&StoredSystemValues>;

    /// Sets the system values of the stored version this write replaces, once it is read
    fn set_stored_system_values(&mut self, values: StoredSystemValues);
}
