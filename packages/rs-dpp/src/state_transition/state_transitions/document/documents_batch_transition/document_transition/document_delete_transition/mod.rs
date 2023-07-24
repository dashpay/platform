mod v0;

use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionMethodsV0;
use bincode::{Decode, Encode};
use derive_more::Display;
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Display)]
pub enum DocumentDeleteTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentDeleteTransitionV0),
}

impl DocumentTransitionMethodsV0 for DocumentDeleteTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentDeleteTransition::V0(v0) => &v0.base,
        }
    }
}
