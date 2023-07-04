use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use anyhow::Context;
use bincode::{Decode, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract, prelude::Identifier, util::json_value::JsonValueExt, ProtocolError,
};
use document_base_transition::DocumentBaseTransition;

mod action;
pub mod document_base_transition;
mod document_base_transition_action;
pub mod document_create_transition;
mod document_create_transition_action;
pub mod document_delete_transition;
mod document_delete_transition_action;
pub mod document_replace_transition;
mod document_replace_transition_action;

use crate::identity::TimestampMillis;
use crate::prelude::Revision;
pub use document_base_transition::DocumentTransitionObjectLike;
pub use document_create_transition::DocumentCreateTransition;
pub use document_delete_transition::DocumentDeleteTransition;
pub use document_replace_transition::DocumentReplaceTransition;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;

pub use document_base_transition_action::DocumentBaseTransitionAction;
pub use document_create_transition_action::DocumentCreateTransitionAction;
pub use document_delete_transition_action::DocumentDeleteTransitionAction;
pub use document_replace_transition_action::DocumentReplaceTransitionAction;

pub use action::{DocumentTransitionAction, DOCUMENT_TRANSITION_ACTION_VERSION};

pub const PROPERTY_ACTION: &str = "$action";

pub trait DocumentTransitionExt {
    /// returns the creation timestamp (in milliseconds) if it exists for given type of document transition
    fn get_created_at(&self) -> Option<TimestampMillis>;
    /// returns the update timestamp  (in milliseconds) if it exists for given type of document transition
    fn get_updated_at(&self) -> Option<TimestampMillis>;
    /// set the created_at (in milliseconds) if it exists
    fn set_created_at(&mut self, timestamp_millis: Option<TimestampMillis>);
    /// set the updated_at (in milliseconds) if it exists
    fn set_updated_at(&mut self, timestamp_millis: Option<TimestampMillis>);
    /// returns the value of dynamic property. The dynamic property is a property that is not specified in protocol
    /// the `path` supports dot-syntax: i.e: property.internal_property
    fn get_dynamic_property(&self, path: &str) -> Option<&Value>;
    ///  get the id
    fn get_id(&self) -> &Identifier;
    /// get the document type
    fn get_document_type(&self) -> &String;
    /// get the transition action
    fn get_action(&self) -> Action;
    /// get the data contract
    fn get_data_contract(&self) -> &DataContract;
    /// get the data contract id
    fn get_data_contract_id(&self) -> &Identifier;
    /// get the data of the transition if exits
    fn get_data(&self) -> Option<&BTreeMap<String, Value>>;
    /// get the revision of transition if exits
    fn get_revision(&self) -> Option<Revision>;
    #[cfg(test)]
    /// Inserts the dynamic property into the document
    fn insert_dynamic_property(&mut self, property_name: String, value: Value);
    /// set data contract's ID
    fn set_data_contract_id(&mut self, id: Identifier);
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, From, PartialEq)]
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

#[derive(Deserialize)]
struct TransitionWithAction {
    action: Action,
}

impl DocumentTransitionObjectLike for DocumentTransition {
    #[cfg(feature = "json-object")]
    fn from_json_object(
        json_value: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let action: Action = TryFrom::try_from(json_value.get_u64(PROPERTY_ACTION)? as u8)
            .context("invalid document transition action")?;

        Ok(match action {
            Action::Create => DocumentTransition::Create(
                DocumentCreateTransition::from_json_object(json_value, data_contract)?,
            ),
            Action::Replace => DocumentTransition::Replace(
                DocumentReplaceTransition::from_json_object(json_value, data_contract)?,
            ),
            Action::Delete => DocumentTransition::Delete(
                DocumentDeleteTransition::from_json_object(json_value, data_contract)?,
            ),
        })
    }

    #[cfg(feature = "platform-value")]
    fn from_raw_object(
        raw_transition: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let map = raw_transition
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Self::from_value_map(map, data_contract)
    }

    #[cfg(feature = "json-object")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        call_method!(self, to_json)
    }

    #[cfg(feature = "platform-value")]
    fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        call_method!(self, to_value_map)
    }

    #[cfg(feature = "platform-value")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        call_method!(self, to_object)
    }

    #[cfg(feature = "platform-value")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        call_method!(self, to_cleaned_object)
    }

    #[cfg(feature = "platform-value")]
    fn from_value_map(
        map: BTreeMap<String, Value>,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let action: Action = map.get_integer::<u8>(PROPERTY_ACTION)?.try_into()?;
        Ok(match action {
            Action::Create => DocumentTransition::Create(DocumentCreateTransition::from_value_map(
                map,
                data_contract,
            )?),
            Action::Replace => DocumentTransition::Replace(
                DocumentReplaceTransition::from_value_map(map, data_contract)?,
            ),
            Action::Delete => DocumentTransition::Delete(DocumentDeleteTransition::from_value_map(
                map,
                data_contract,
            )?),
        })
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

    pub fn action(&self) -> Action {
        self.base().action
    }
}

impl DocumentTransitionExt for DocumentTransition {
    fn get_id(&self) -> &Identifier {
        &self.base().id
    }

    fn get_document_type(&self) -> &String {
        &self.base().document_type_name
    }

    fn get_action(&self) -> Action {
        self.base().action
    }

    fn get_data_contract(&self) -> &DataContract {
        &self.base().data_contract
    }

    fn get_data_contract_id(&self) -> &Identifier {
        &self.base().data_contract_id
    }

    fn get_updated_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentTransition::Create(t) => t.updated_at,
            DocumentTransition::Replace(t) => t.updated_at,
            DocumentTransition::Delete(_) => None,
        }
    }

    fn set_updated_at(&mut self, timestamp_millis: Option<TimestampMillis>) {
        match self {
            DocumentTransition::Create(ref mut t) => t.updated_at = timestamp_millis,
            DocumentTransition::Replace(ref mut t) => t.updated_at = timestamp_millis,
            DocumentTransition::Delete(_) => {}
        }
    }

    fn get_created_at(&self) -> Option<TimestampMillis> {
        match self {
            DocumentTransition::Create(t) => t.created_at,
            DocumentTransition::Replace(_) => None,
            DocumentTransition::Delete(_) => None,
        }
    }

    fn set_created_at(&mut self, timestamp_millis: Option<TimestampMillis>) {
        match self {
            DocumentTransition::Create(ref mut t) => t.created_at = timestamp_millis,
            DocumentTransition::Replace(_) => {}
            DocumentTransition::Delete(_) => {}
        }
    }

    fn get_dynamic_property(&self, path: &str) -> Option<&Value> {
        match self {
            DocumentTransition::Create(t) => {
                if let Some(ref data) = t.data {
                    data.get(path)
                } else {
                    None
                }
            }
            DocumentTransition::Replace(t) => {
                if let Some(ref data) = t.data {
                    data.get(path)
                } else {
                    None
                }
            }
            DocumentTransition::Delete(_) => None,
        }
    }

    fn get_data(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            DocumentTransition::Create(t) => t.data.as_ref(),
            DocumentTransition::Replace(t) => t.data.as_ref(),
            DocumentTransition::Delete(_) => None,
        }
    }

    fn get_revision(&self) -> Option<Revision> {
        match self {
            DocumentTransition::Create(t) => t.get_revision(),
            DocumentTransition::Replace(t) => Some(t.revision),
            DocumentTransition::Delete(_) => None,
        }
    }

    #[cfg(test)]
    fn insert_dynamic_property(&mut self, property_name: String, value: Value) {
        match self {
            DocumentTransition::Create(ref mut t) => {
                if let Some(ref mut data) = t.data {
                    let _ = data.insert(property_name, value);
                }
            }
            DocumentTransition::Replace(ref mut t) => {
                if let Some(ref mut data) = t.data {
                    let _ = data.insert(property_name, value);
                }
            }
            DocumentTransition::Delete(_) => {}
        }
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        match self {
            DocumentTransition::Create(ref mut t) => {
                t.base.data_contract_id = id;
            }
            DocumentTransition::Replace(ref mut t) => {
                t.base.data_contract_id = id;
            }
            DocumentTransition::Delete(ref mut t) => {
                t.base.data_contract_id = id;
            }
        }
    }
}
