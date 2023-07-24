#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DocumentDeleteTransitionActionV0 {
    pub base: DocumentBaseTransitionAction,
}

pub trait DocumentDeleteTransitionActionAccessorsV0 {
    fn base(&self) -> &DocumentBaseTransitionAction;
    fn base_owned(self) -> DocumentBaseTransitionAction;
}
