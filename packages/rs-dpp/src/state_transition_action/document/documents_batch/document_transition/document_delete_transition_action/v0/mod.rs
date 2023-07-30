#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DocumentDeleteTransitionActionV0<'a> {
    pub base: DocumentBaseTransitionAction<'a>,
}

pub trait DocumentDeleteTransitionActionAccessorsV0<'a> {
    fn base(&self) -> &DocumentBaseTransitionAction<'a>;
    fn base_owned(self) -> DocumentBaseTransitionAction<'a>;
}
