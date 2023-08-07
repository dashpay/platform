//! Documents.
//!
//! This module defines the `Document` struct and implements its functions.
//!

mod accessors;
#[cfg(feature = "cbor")]
pub(super) mod cbor_conversion;
#[cfg(feature = "json-object")]
pub(super) mod json_conversion;
#[cfg(feature = "platform-value")]
mod platform_value_conversion;
pub mod serialize;

use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::{BTreeMap, HashSet};
use std::fmt;

use serde_json::{json, Value as JsonValue};

use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::DataContract;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};

use crate::data_contract::document_type::v0::v0_methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::errors::DataContractError;
use crate::document::document_methods::{
    DocumentGetRawForContractV0, DocumentGetRawForDocumentTypeV0, DocumentHashV0Method,
    DocumentMethodsV0,
};

use crate::document::errors::DocumentError;

use crate::identity::TimestampMillis;
use crate::prelude::Identifier;
use crate::prelude::Revision;

use crate::util::hash::hash_to_vec;
use crate::ProtocolError;

/// Documents contain the data that goes into data contracts.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct DocumentV0 {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: Identifier,
    /// The ID of the document's owner.
    #[serde(rename = "$ownerId")]
    pub owner_id: Identifier,
    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,
    /// The document revision.
    #[serde(rename = "$revision", default)]
    pub revision: Option<Revision>,
    /// The time in milliseconds that the document was created
    #[serde(rename = "$createdAt", default)]
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the document was last updated
    #[serde(rename = "$updatedAt", default)]
    pub updated_at: Option<TimestampMillis>,
}

impl DocumentGetRawForContractV0 for DocumentV0 {
    //automatically done
}

impl DocumentGetRawForDocumentTypeV0 for DocumentV0 {
    //automatically done
}

impl DocumentHashV0Method for DocumentV0 {
    //automatically done
}

impl fmt::Display for DocumentV0 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id:{} ", self.id)?;
        write!(f, "owner_id:{} ", self.owner_id)?;
        if let Some(created_at) = self.created_at {
            let naive = NaiveDateTime::from_timestamp_millis(created_at as i64).unwrap_or_default();
            let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
            write!(f, "created_at:{} ", datetime.format("%Y-%m-%d %H:%M:%S"))?;
        }
        if let Some(updated_at) = self.updated_at {
            let naive = NaiveDateTime::from_timestamp_millis(updated_at as i64).unwrap_or_default();
            let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
            write!(f, "updated_at:{} ", datetime.format("%Y-%m-%d %H:%M:%S"))?;
        }

        if self.properties.is_empty() {
            write!(f, "no properties")?;
        } else {
            for (key, value) in self.properties.iter() {
                write!(f, "{}:{} ", key, value)?
            }
        }
        Ok(())
    }
}
