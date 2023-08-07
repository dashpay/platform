mod from_document;
pub(crate) mod v0;
mod v0_methods;

use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use bincode::{Decode, Encode};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
pub use v0::*;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Display, From)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum DocumentDeleteTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentDeleteTransitionV0),
}
