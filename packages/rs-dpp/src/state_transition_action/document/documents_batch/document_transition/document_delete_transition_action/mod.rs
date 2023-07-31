use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::{DocumentDeleteTransitionActionAccessorsV0, DocumentDeleteTransitionActionV0};

#[cfg(feature = "state-transition-transformers")]
pub mod transformer;
pub mod v0;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

#[derive(Debug, Clone, From)]
pub enum DocumentDeleteTransitionAction<'a> {
    V0(DocumentDeleteTransitionActionV0<'a>),
}

impl<'a> DocumentDeleteTransitionActionAccessorsV0<'a> for DocumentDeleteTransitionAction<'a> {
    fn base(&self) -> &DocumentBaseTransitionAction<'a> {
        match self {
            DocumentDeleteTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction<'a> {
        match self {
            DocumentDeleteTransitionAction::V0(v0) => v0.base,
        }
    }
}
