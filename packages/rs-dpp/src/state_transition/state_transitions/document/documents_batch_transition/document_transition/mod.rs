use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use derive_more::From;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::prelude::{Identifier, IdentityNonce};
use document_base_transition::DocumentBaseTransition;

pub mod action_type;
pub mod document_base_transition;
pub mod document_create_transition;
pub mod document_delete_transition;
pub mod document_replace_transition;

use crate::prelude::Revision;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use derive_more::Display;
pub use document_create_transition::DocumentCreateTransition;
pub use document_delete_transition::DocumentDeleteTransition;
pub use document_replace_transition::DocumentReplaceTransition;
use platform_value::Value;

use crate::state_transition::state_transitions::document::documents_batch_transition::document_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::state_transition::state_transitions::document::documents_batch_transition::document_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use crate::state_transition::state_transitions::document::documents_batch_transition::document_transition::document_delete_transition::v0::v0_methods::DocumentDeleteTransitionV0Methods;

pub const PROPERTY_ACTION: &str = "$action";

pub trait DocumentTransitionV0Methods {
    fn base(&self) -> &DocumentBaseTransition;
    /// returns the value of dynamic property. The dynamic property is a property that is not specified in protocol
    /// the `path` supports dot-syntax: i.e: property.internal_property
    fn get_dynamic_property(&self, path: &str) -> Option<&Value>;
    ///  get the id
    fn get_id(&self) -> Identifier;
    /// get the document type
    fn document_type_name(&self) -> &String;
    /// get the data contract id
    fn data_contract_id(&self) -> Identifier;
    /// get the data of the transition if exits
    fn data(&self) -> Option<&BTreeMap<String, Value>>;
    /// get the revision of transition if exits
    fn revision(&self) -> Option<Revision>;

    /// get the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;
    #[cfg(test)]
    /// Inserts the dynamic property into the document
    fn insert_dynamic_property(&mut self, property_name: String, value: Value);
    /// set data contract's ID
    fn set_data_contract_id(&mut self, id: Identifier);
    fn base_mut(&mut self) -> &mut DocumentBaseTransition;
    fn data_mut(&mut self) -> Option<&mut BTreeMap<String, Value>>;

    // sets revision of the transition
    fn set_revision(&mut self, revision: Revision);

    // sets identity contract nonce
    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce);
}

#[derive(Debug, Clone, Encode, Decode, From, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum DocumentTransition {
    #[display(fmt = "CreateDocumentTransition({})", "_0")]
    Create(DocumentCreateTransition),

    #[display(fmt = "ReplaceDocumentTransition({})", "_0")]
    Replace(DocumentReplaceTransition),

    #[display(fmt = "DeleteDocumentTransition({})", "_0")]
    Delete(DocumentDeleteTransition),
}

//
// impl AsRef<Self> for DocumentTransition {
//     fn as_ref(&self) -> &Self {
//         self
//     }
// }
//
// macro_rules! call_method {
//     ($state_transition:expr, $method:ident, $args:tt ) => {
//         match $state_transition {
//             DocumentTransition::Create(st) => st.$method($args),
//             DocumentTransition::Replace(st) => st.$method($args),
//             DocumentTransition::Delete(st) => st.$method($args),
//         }
//     };
//     ($state_transition:expr, $method:ident ) => {
//         match $state_transition {
//             DocumentTransition::Create(st) => st.$method(),
//             DocumentTransition::Replace(st) => st.$method(),
//             DocumentTransition::Delete(st) => st.$method(),
//         }
//     };
// }
//
// impl DocumentTransitionObjectLike for DocumentTransition {
//     #[cfg(feature = "state-transition-json-conversion")]
//     fn from_json_object(
//         json_value: JsonValue,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError>
//     where
//         Self: Sized,
//     {
//         let action: Action = TryFrom::try_from(json_value.get_u64(PROPERTY_ACTION)? as u8)
//             .context("invalid document transition action")?;
//
//         Ok(match action {
//             Action::Create => DocumentTransition::Create(
//                 DocumentCreateTransition::from_json_object(json_value, data_contract)?,
//             ),
//             Action::Replace => DocumentTransition::Replace(
//                 DocumentReplaceTransitionV0::from_json_object(json_value, data_contract)?,
//             ),
//             Action::Delete => DocumentTransition::Delete(
//                 DocumentDeleteTransition::from_json_object(json_value, data_contract)?,
//             ),
//         })
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn from_object(
//         raw_transition: Value,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError>
//     where
//         Self: Sized,
//     {
//         let map = raw_transition
//             .into_btree_string_map()
//             .map_err(ProtocolError::ValueError)?;
//         Self::from_value_map(map, data_contract)
//     }
//
//     #[cfg(feature = "state-transition-json-conversion")]
//     fn to_json(&self) -> Result<JsonValue, ProtocolError> {
//         call_method!(self, to_json)
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_value_map(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
//         call_method!(self, to_value_map)
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_object(&self) -> Result<Value, ProtocolError> {
//         call_method!(self, to_object)
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
//         call_method!(self, to_cleaned_object)
//     }
//
//     #[cfg(feature = "state-transition-value-conversion")]
//     fn from_value_map(
//         map: BTreeMap<String, Value>,
//         data_contract: DataContract,
//     ) -> Result<Self, ProtocolError>
//     where
//         Self: Sized,
//     {
//         let action: Action = map.get_integer::<u8>(PROPERTY_ACTION)?.try_into()?;
//         Ok(match action {
//             Action::Create => DocumentTransition::Create(DocumentCreateTransition::from_value_map(
//                 map,
//                 data_contract,
//             )?),
//             Action::Replace => DocumentTransition::Replace(
//                 DocumentReplaceTransitionV0::from_value_map(map, data_contract)?,
//             ),
//             Action::Delete => DocumentTransition::Delete(DocumentDeleteTransition::from_value_map(
//                 map,
//                 data_contract,
//             )?),
//         })
//     }
// }

impl DocumentTransition {
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

impl DocumentTransitionV0Methods for DocumentTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentTransition::Create(t) => t.base(),
            DocumentTransition::Replace(t) => t.base(),
            DocumentTransition::Delete(t) => t.base(),
        }
    }

    fn get_dynamic_property(&self, path: &str) -> Option<&Value> {
        match self {
            DocumentTransition::Create(t) => t.data().get(path),
            DocumentTransition::Replace(t) => t.data().get(path),
            DocumentTransition::Delete(_) => None,
        }
    }

    fn get_id(&self) -> Identifier {
        self.base().id()
    }

    fn document_type_name(&self) -> &String {
        self.base().document_type_name()
    }

    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    fn data(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            DocumentTransition::Create(t) => Some(t.data()),
            DocumentTransition::Replace(t) => Some(t.data()),
            DocumentTransition::Delete(_) => None,
        }
    }

    fn revision(&self) -> Option<Revision> {
        match self {
            DocumentTransition::Create(_) => Some(1),
            DocumentTransition::Replace(t) => Some(t.revision()),
            DocumentTransition::Delete(_) => None,
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DocumentTransition::Create(t) => t.base().identity_contract_nonce(),
            DocumentTransition::Replace(t) => t.base().identity_contract_nonce(),
            DocumentTransition::Delete(t) => t.base().identity_contract_nonce(),
        }
    }

    #[cfg(test)]
    fn insert_dynamic_property(&mut self, property_name: String, value: Value) {
        match self {
            DocumentTransition::Create(document_create_transition) => {
                document_create_transition
                    .data_mut()
                    .insert(property_name, value);
            }
            DocumentTransition::Replace(document_replace_transition) => {
                document_replace_transition
                    .data_mut()
                    .insert(property_name, value);
            }
            DocumentTransition::Delete(_) => {}
        }
    }

    fn set_data_contract_id(&mut self, id: Identifier) {
        self.base_mut().set_data_contract_id(id)
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentTransition::Create(t) => t.base_mut(),
            DocumentTransition::Replace(t) => t.base_mut(),
            DocumentTransition::Delete(t) => t.base_mut(),
        }
    }

    fn data_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        match self {
            DocumentTransition::Create(t) => Some(t.data_mut()),
            DocumentTransition::Replace(t) => Some(t.data_mut()),
            DocumentTransition::Delete(_) => None,
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            DocumentTransition::Create(_) => {}
            DocumentTransition::Replace(ref mut t) => t.set_revision(revision),
            DocumentTransition::Delete(_) => {}
        }
    }

    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            DocumentTransition::Create(t) => t.base_mut().set_identity_contract_nonce(nonce),
            DocumentTransition::Replace(t) => t.base_mut().set_identity_contract_nonce(nonce),
            DocumentTransition::Delete(t) => t.base_mut().set_identity_contract_nonce(nonce),
        }
    }
}
