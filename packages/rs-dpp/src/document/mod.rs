use std::convert::TryInto;

use ciborium::value::Value as CborValue;
use integer_encoding::VarInt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub use state_transition::documents_batch_transition::document_transition;
pub use state_transition::documents_batch_transition::validation;
pub use state_transition::documents_batch_transition::DocumentsBatchTransition;

use crate::data_contract::DataContract;
use crate::errors::ProtocolError;
use crate::identifier::Identifier;
use crate::metadata::Metadata;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::cbor_value::FieldType;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::util::hash::hash;
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::{cbor_value, deserializer};

mod document;
pub mod document_factory;
pub mod document_validator;
pub mod errors;
pub mod fetch_and_validate_data_contract;
pub mod generate_document_id;
pub mod serialize;
pub mod state_transition;
pub use document::Document;
pub use state_transition::documents_batch_transition::document_transition::document_in_state_transition::DocumentInStateTransition;
pub use state_transition::documents_batch_transition::document_transition::document_in_state_transition::property_names as document_in_state_transition_property_names;
pub use state_transition::documents_batch_transition::document_transition::document_in_state_transition::IDENTIFIER_FIELDS as DOCUMENT_IN_STATE_TRANSITION_IDENTIFIER_FIELDS;