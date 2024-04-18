pub mod transformer;

use dpp::document::{Document, DocumentV0};
use dpp::identity::TimestampMillis;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::{BlockHeight, CoreBlockHeight, Revision};
use dpp::ProtocolError;

use std::collections::BTreeMap;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::state_transition_action::document::documents_batch::document_transition::document_transfer_transition_action::DocumentTransferTransitionAction;

/// document transfer transition action v0
#[derive(Debug, Clone)]
pub struct DocumentTransferTransitionActionV0 {
    /// Document Base Transition
    pub base: DocumentBaseTransitionAction,
    /// The new document to be inserted
    pub document: Document,
}

/// document transfer transition action accessors v0
pub trait DocumentTransferTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
    /// created at
    fn document(&self) -> &Document;
    fn document_owned(self) -> Document;
}