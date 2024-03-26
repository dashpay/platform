//! Documents.
//!
//! This module defines the `Document` struct and implements its functions.
//!

mod accessors;
#[cfg(feature = "document-cbor-conversion")]
pub(super) mod cbor_conversion;
#[cfg(feature = "document-json-conversion")]
pub(super) mod json_conversion;
#[cfg(feature = "document-value-conversion")]
mod platform_value_conversion;
pub mod serialize;

use chrono::DateTime;
use std::collections::BTreeMap;
use std::fmt;

use platform_value::Value;
use serde::{Deserialize, Serialize};

use crate::document::document_methods::{
    DocumentGetRawForContractV0, DocumentGetRawForDocumentTypeV0, DocumentHashV0Method,
    DocumentIsEqualIgnoringTimestampsV0,
};

use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::prelude::{BlockHeight, CoreBlockHeight, Identifier};

/// Documents contain the data that goes into data contracts.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "document-serde-conversion", derive(Serialize, Deserialize))]
pub struct DocumentV0 {
    /// The unique document ID.
    #[cfg_attr(feature = "document-serde-conversion", serde(rename = "$id"))]
    pub id: Identifier,
    /// The ID of the document's owner.
    #[cfg_attr(feature = "document-serde-conversion", serde(rename = "$ownerId"))]
    pub owner_id: Identifier,
    /// The document's properties (data).
    #[cfg_attr(feature = "document-serde-conversion", serde(flatten))]
    pub properties: BTreeMap<String, Value>,
    /// The document revision, if the document is mutable.
    #[cfg_attr(
        feature = "document-serde-conversion",
        serde(rename = "$revision", default)
    )]
    pub revision: Option<Revision>,
    /// The time in milliseconds that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "document-serde-conversion",
        serde(rename = "$createdAt", default)
    )]
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "document-serde-conversion",
        serde(rename = "$updatedAt", default)
    )]
    pub updated_at: Option<TimestampMillis>,
    /// The block that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "document-serde-conversion",
        serde(rename = "$createdAtBlockHeight", default)
    )]
    pub created_at_block_height: Option<BlockHeight>,
    /// The block that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "document-serde-conversion",
        serde(rename = "$updatedAtBlockHeight", default)
    )]
    pub updated_at_block_height: Option<BlockHeight>,
    /// The core block that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "document-serde-conversion",
        serde(rename = "$createdAtCoreBlockHeight", default)
    )]
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "document-serde-conversion",
        serde(rename = "$updatedAtCoreBlockHeight", default)
    )]
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
}

impl DocumentGetRawForContractV0 for DocumentV0 {
    //automatically done
}

impl DocumentIsEqualIgnoringTimestampsV0 for DocumentV0 {
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
            let datetime = DateTime::from_timestamp_millis(created_at as i64).unwrap_or_default();
            write!(f, "created_at:{} ", datetime.format("%Y-%m-%d %H:%M:%S"))?;
        }
        if let Some(updated_at) = self.updated_at {
            let datetime = DateTime::from_timestamp_millis(updated_at as i64).unwrap_or_default();
            write!(f, "updated_at:{} ", datetime.format("%Y-%m-%d %H:%M:%S"))?;
        }

        if let Some(created_at_block_height) = self.created_at_block_height {
            write!(f, "created_at_block_height:{} ", created_at_block_height)?;
        }
        if let Some(updated_at_block_height) = self.updated_at_block_height {
            write!(f, "updated_at_block_height:{} ", updated_at_block_height)?;
        }
        if let Some(created_at_core_block_height) = self.created_at_core_block_height {
            write!(
                f,
                "created_at_core_block_height:{} ",
                created_at_core_block_height
            )?;
        }
        if let Some(updated_at_core_block_height) = self.updated_at_core_block_height {
            write!(
                f,
                "updated_at_core_block_height:{} ",
                updated_at_core_block_height
            )?;
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
