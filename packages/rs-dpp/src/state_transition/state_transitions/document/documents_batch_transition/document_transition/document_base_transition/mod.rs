mod fields;
pub mod v0;
mod v0_methods;

use crate::data_contract::DataContract;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
use bincode::{Decode, Encode};
use derive_more::Display;
pub use fields::*;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, PartialEq, Display)]
pub enum DocumentBaseTransition {
    #[display(fmt = "V0({})", "_0")]
    V0(DocumentBaseTransitionV0),
}

impl Default for DocumentBaseTransition {
    fn default() -> Self {
        DocumentBaseTransition::V0(DocumentBaseTransitionV0::default()) // since only v0
    }
}
