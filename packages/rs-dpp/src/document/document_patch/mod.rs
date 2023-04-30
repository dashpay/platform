use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Documents contain the data that goes into data contracts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct DocumentPatch {
    /// The unique document ID.
    #[serde(rename = "$id")]
    pub id: Identifier,
    /// The document's properties (data).
    #[serde(flatten)]
    pub properties: BTreeMap<String, Value>,
    /// The document revision.
    #[serde(rename = "$revision")]
    pub revision: Option<Revision>,
    #[serde(rename = "$updatedAt")]
    pub updated_at: Option<TimestampMillis>,
}
