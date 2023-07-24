mod v0;
mod v0_methods;

use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use bincode::{Decode, Encode};
use derive_more::Display;
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Display)]
pub enum DocumentDeleteTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentDeleteTransitionV0),
}
