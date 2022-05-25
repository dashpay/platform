use std::convert::TryFrom;

use serde::{Serialize, Deserialize};
use anyhow::{anyhow};

mod document_base_transition;
pub use document_base_transition::*;

mod document_create_transition;
pub use document_create_transition::*;

mod document_delete_transition;
pub use document_delete_transition::*;

mod document_replace_transition;
pub use document_replace_transition::*;

use crate::{data_contract::DataContract, ProtocolError, util::json_value::JsonValueExt};

pub const PROPERTY_ACTION : &str = "$action";


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentTransition {
    Create(DocumentCreateTransition),
    Replace(DocumentReplaceTransition),
    Delete(DocumentDeleteTransition),
}



impl DocumentTransition {
    pub fn from_raw_document(raw_transition : JsonValue, data_contract : DataContract ) ->  Result<Self, ProtocolError> {
        let action : Action = TryFrom::try_from(raw_transition.get_u64(PROPERTY_ACTION)? as u8)?;
        Ok(match action  {
            Action::Create => DocumentTransition::Create(DocumentCreateTransition::from_raw_document(raw_transition, data_contract)?),
            Action::Replace => DocumentTransition::Replace(DocumentReplaceTransition::from_raw_document(raw_transition, data_contract)?),
            Action::Delete => DocumentTransition::Delete(DocumentDeleteTransition::from_raw_document(raw_transition, data_contract)?),
        })
    }

    pub fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentTransition::Create(d) => &d.base,
            DocumentTransition::Delete(d) => &d.base,
            DocumentTransition::Replace(d) => &d.base,
        }
    }

    pub fn as_transition_create(&self) -> Option<&DocumentCreateTransition> {
        if let Self::Create(ref t) = self {
            Some(t)
        } else {
            None
        }
    }
    pub fn as_transition_replace(&self) -> Option<&DocumentReplaceTransition> {
        if let Self::Replace(ref t) = self {
            Some(t)
        } else {
            None
        }
    }

    pub fn as_transition_delete(&self) -> Option<&DocumentDeleteTransition> {
        if let Self::Delete(ref t) = self {
            Some(t)
        } else {
            None
        }
    }
}
