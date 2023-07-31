use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

#[cfg(feature = "state-transition-transformers")]
pub mod transformer;
mod v0;

use crate::data_contract::DataContract;
pub use v0::*;

#[derive(Debug, Clone, From)]
pub enum DocumentBaseTransitionAction<'a> {
    V0(DocumentBaseTransitionActionV0<'a>),
}

impl<'a> DocumentBaseTransitionActionAccessorsV0 for DocumentBaseTransitionAction<'a> {
    fn id(&self) -> Identifier {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.id,
        }
    }

    fn document_type_name(&self) -> &String {
        match self {
            DocumentBaseTransitionAction::V0(v0) => &v0.document_type_name,
        }
    }

    fn document_type_name_owned(self) -> String {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.document_type_name,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.data_contract_id,
        }
    }

    fn data_contract(&self) -> &DataContract {
        match self {
            DocumentBaseTransitionAction::V0(v0) => &v0.data_contract,
        }
    }
}
