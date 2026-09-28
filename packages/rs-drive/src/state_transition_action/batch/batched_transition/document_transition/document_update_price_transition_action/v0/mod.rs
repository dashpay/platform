pub mod transformer;

use dpp::data_contract::document_type::property_constraints::AggregateRead;
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
    /// The `countOf` and `sumOf` totals the document type's `propertyConstraints` rules
    /// read, each as it will be once this write is done, read from state when the action is
    /// built; `None` when the rules judging the write read none, and boxed, since only
    /// such a write holds any and the action is one variant of a large enum.
    pub property_constraint_aggregates: Option<Box<BTreeMap<AggregateRead, i128>>>,
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
}
