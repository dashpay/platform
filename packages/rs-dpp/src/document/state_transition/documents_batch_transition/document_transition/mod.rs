use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

pub use document_base_transition::*;
pub use document_create_transition::*;
pub use document_delete_transition::*;
pub use document_replace_transition::*;

use crate::{data_contract::DataContract, util::json_value::JsonValueExt, ProtocolError};

mod document_base_transition;
mod document_create_transition;
mod document_delete_transition;
mod document_replace_transition;

pub const PROPERTY_ACTION: &str = "$action";

pub trait DocumentTransitionExt {
    /// returns the creation timestamp (in milliseconds) if it exists for given type of document transition
    fn get_created_at(&self) -> Option<i64>;
    /// returns the update timestamp  (in milliseconds) if it exists for given type of document transition
    fn get_updated_at(&self) -> Option<i64>;
    /// returns the value of dynamic property. The dynamic property is a property that is not specified in protocol
    /// the `path` supports dot-syntax: i.e: property.internal_property
    fn get_dynamic_property(&self, path: &str) -> Option<&JsonValue>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentTransition {
    Create(DocumentCreateTransition),
    Replace(DocumentReplaceTransition),
    Delete(DocumentDeleteTransition),
}

impl AsRef<Self> for DocumentTransition {
    fn as_ref(&self) -> &Self {
        self
    }
}

macro_rules! call_method {
    ($state_transition:expr, $method:ident, $args:tt ) => {
        match $state_transition {
            DocumentTransition::Create(st) => st.$method($args),
            DocumentTransition::Replace(st) => st.$method($args),
            DocumentTransition::Delete(st) => st.$method($args),
        }
    };
    ($state_transition:expr, $method:ident ) => {
        match $state_transition {
            DocumentTransition::Create(st) => st.$method(),
            DocumentTransition::Replace(st) => st.$method(),
            DocumentTransition::Delete(st) => st.$method(),
        }
    };
}

impl DocumentTransitionObjectLike for DocumentTransition {
    fn from_json_str(_json_str: &str, _data_contract: DataContract) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn from_raw_document(
        raw_transition: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let action: Action = TryFrom::try_from(raw_transition.get_u64(PROPERTY_ACTION)? as u8)?;
        Ok(match action {
            Action::Create => DocumentTransition::Create(
                DocumentCreateTransition::from_raw_document(raw_transition, data_contract)?,
            ),
            Action::Replace => DocumentTransition::Replace(
                DocumentReplaceTransition::from_raw_document(raw_transition, data_contract)?,
            ),
            Action::Delete => DocumentTransition::Delete(
                DocumentDeleteTransition::from_raw_document(raw_transition, data_contract)?,
            ),
        })
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        call_method!(self, to_json)
    }

    fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        call_method!(self, to_object)
    }
}

impl DocumentTransition {
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

impl DocumentTransitionExt for DocumentTransition {
    fn get_updated_at(&self) -> Option<i64> {
        match self {
            DocumentTransition::Create(t) => t.updated_at,
            DocumentTransition::Replace(t) => t.updated_at,
            DocumentTransition::Delete(_) => None,
        }
    }

    fn get_created_at(&self) -> Option<i64> {
        match self {
            DocumentTransition::Create(t) => t.created_at,
            DocumentTransition::Replace(_) => None,
            DocumentTransition::Delete(_) => None,
        }
    }

    fn get_dynamic_property(&self, path: &str) -> Option<&JsonValue> {
        match self {
            DocumentTransition::Create(t) => {
                if let Some(ref data) = t.data {
                    data.get_value(path).ok()
                } else {
                    None
                }
            }
            DocumentTransition::Replace(t) => {
                if let Some(ref data) = t.data {
                    data.get_value(path).ok()
                } else {
                    None
                }
            }
            DocumentTransition::Delete(_) => None,
        }
    }
}
