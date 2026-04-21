//! Documents.
//!
//! This module defines the `Document` struct and implements its functions.
//!

mod accessors;
#[cfg(feature = "document-cbor-conversion")]
pub(super) mod cbor_conversion;
#[cfg(feature = "json-conversion")]
pub(super) mod json_conversion;
#[cfg(feature = "value-conversion")]
mod platform_value_conversion;
pub mod serialize;

use chrono::DateTime;
use std::collections::BTreeMap;
use std::fmt;

use platform_value::Value;

use crate::document::document_methods::{
    DocumentGetRawForContractV0, DocumentGetRawForDocumentTypeV0, DocumentHashV0Method,
    DocumentIsEqualIgnoringTimestampsV0,
};

use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::prelude::{BlockHeight, CoreBlockHeight, Identifier};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;

/// Documents contain the data that goes into data contracts.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(
    any(feature = "serde-conversion", feature = "serde-conversion"),
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct DocumentV0 {
    /// The unique document ID.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$id")
    )]
    pub id: Identifier,
    /// The ID of the document's owner.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$ownerId")
    )]
    pub owner_id: Identifier,
    /// The document's properties (data).
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(flatten)
    )]
    pub properties: BTreeMap<String, Value>,
    /// The document revision, if the document is mutable.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$revision", default)
    )]
    pub revision: Option<Revision>,
    /// The time in milliseconds that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$createdAt", default)
    )]
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$updatedAt", default)
    )]
    pub updated_at: Option<TimestampMillis>,
    /// The time in milliseconds that the document was last transferred, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$transferredAt", default)
    )]
    pub transferred_at: Option<TimestampMillis>,
    /// The block that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$createdAtBlockHeight", default)
    )]
    pub created_at_block_height: Option<BlockHeight>,
    /// The block that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$updatedAtBlockHeight", default)
    )]
    pub updated_at_block_height: Option<BlockHeight>,
    /// The block that the document was last transferred to a new identity, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$transferredAtBlockHeight", default)
    )]
    pub transferred_at_block_height: Option<BlockHeight>,
    /// The core block that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$createdAtCoreBlockHeight", default)
    )]
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$updatedAtCoreBlockHeight", default)
    )]
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block that the document was last transferred to a new identity, if it is set as required by the document type schema.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$transferredAtCoreBlockHeight", default)
    )]
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,
    /// The creator id.
    #[cfg_attr(
        any(feature = "serde-conversion", feature = "serde-conversion"),
        serde(rename = "$creatorId", default)
    )]
    pub creator_id: Option<Identifier>,
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
        if let Some(transferred_at) = self.transferred_at {
            let datetime =
                DateTime::from_timestamp_millis(transferred_at as i64).unwrap_or_default();
            write!(
                f,
                "transferred_at:{} ",
                datetime.format("%Y-%m-%d %H:%M:%S")
            )?;
        }

        if let Some(created_at_block_height) = self.created_at_block_height {
            write!(f, "created_at_block_height:{} ", created_at_block_height)?;
        }
        if let Some(updated_at_block_height) = self.updated_at_block_height {
            write!(f, "updated_at_block_height:{} ", updated_at_block_height)?;
        }
        if let Some(transferred_at_block_height) = self.transferred_at_block_height {
            write!(
                f,
                "transferred_at_block_height:{} ",
                transferred_at_block_height
            )?;
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
        if let Some(transferred_at_core_block_height) = self.transferred_at_core_block_height {
            write!(
                f,
                "transferred_at_core_block_height:{} ",
                transferred_at_core_block_height
            )?;
        }

        if let Some(creator_id) = self.creator_id {
            write!(f, "creator_id:{} ", creator_id)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentV0Getters, DocumentV0Setters};
    use platform_value::Identifier;

    fn minimal_doc() -> DocumentV0 {
        DocumentV0 {
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties: BTreeMap::new(),
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
    }

    // ================================================================
    //  Display impl: exercise each optional-field branch
    // ================================================================

    #[test]
    fn display_minimal_document_has_no_properties_marker() {
        let doc = minimal_doc();
        let s = format!("{}", doc);
        assert!(s.contains("id:"), "should contain id");
        assert!(s.contains("owner_id:"), "should contain owner_id");
        assert!(
            s.contains("no properties"),
            "empty properties should render as 'no properties', got: {s}"
        );
    }

    #[test]
    fn display_with_properties_formats_key_value_pairs() {
        let mut doc = minimal_doc();
        doc.properties
            .insert("name".to_string(), Value::Text("Bob".to_string()));
        let s = format!("{}", doc);
        assert!(!s.contains("no properties"));
        assert!(s.contains("name:"), "should contain property key");
    }

    #[test]
    fn display_formats_all_optional_timestamp_fields() {
        let mut doc = minimal_doc();
        // Set every optional field to exercise each branch of Display
        doc.created_at = Some(1_700_000_000_000);
        doc.updated_at = Some(1_700_000_100_000);
        doc.transferred_at = Some(1_700_000_200_000);
        doc.created_at_block_height = Some(10);
        doc.updated_at_block_height = Some(20);
        doc.transferred_at_block_height = Some(30);
        doc.created_at_core_block_height = Some(1);
        doc.updated_at_core_block_height = Some(2);
        doc.transferred_at_core_block_height = Some(3);
        doc.creator_id = Some(Identifier::new([9u8; 32]));

        let s = format!("{}", doc);
        // Each branch should emit its labeled prefix
        assert!(s.contains("created_at:"), "missing created_at: {s}");
        assert!(s.contains("updated_at:"), "missing updated_at: {s}");
        assert!(s.contains("transferred_at:"), "missing transferred_at: {s}");
        assert!(
            s.contains("created_at_block_height:10"),
            "missing created_at_block_height: {s}"
        );
        assert!(
            s.contains("updated_at_block_height:20"),
            "missing updated_at_block_height: {s}"
        );
        assert!(
            s.contains("transferred_at_block_height:30"),
            "missing transferred_at_block_height: {s}"
        );
        assert!(
            s.contains("created_at_core_block_height:1"),
            "missing created_at_core_block_height: {s}"
        );
        assert!(
            s.contains("updated_at_core_block_height:2"),
            "missing updated_at_core_block_height: {s}"
        );
        assert!(
            s.contains("transferred_at_core_block_height:3"),
            "missing transferred_at_core_block_height: {s}"
        );
        assert!(s.contains("creator_id:"), "missing creator_id: {s}");
    }

    #[test]
    fn display_invalid_timestamp_uses_default_formatter() {
        // Timestamps that overflow DateTime should use `.unwrap_or_default()`.
        // This ensures the "unwrap_or_default()" branch of Display is hit.
        let mut doc = minimal_doc();
        // u64::MAX casts to -1i64, which IS inside chrono's range (1 ms before
        // epoch). Use i64::MAX instead — it exceeds chrono's supported ms
        // range (~262,000 years) so `from_timestamp_millis` returns None and
        // the `.unwrap_or_default()` branch is actually exercised.
        doc.created_at = Some(i64::MAX as u64);
        let s = format!("{}", doc);
        // Must not panic and must contain the created_at prefix
        assert!(s.contains("created_at:"));
    }

    // ================================================================
    //  bump_revision: saturating behavior and None pass-through
    // ================================================================

    #[test]
    fn bump_revision_increments_when_some() {
        let mut doc = minimal_doc();
        doc.set_revision(Some(5));
        doc.bump_revision();
        assert_eq!(doc.revision(), Some(6));
    }

    #[test]
    fn bump_revision_is_noop_when_none() {
        let mut doc = minimal_doc();
        assert_eq!(doc.revision(), None);
        doc.bump_revision();
        // None -> None; no panic, no change.
        assert_eq!(doc.revision(), None);
    }

    #[test]
    fn bump_revision_saturates_at_max() {
        let mut doc = minimal_doc();
        doc.set_revision(Some(Revision::MAX));
        doc.bump_revision();
        // saturating_add should cap at MAX, not wrap
        assert_eq!(doc.revision(), Some(Revision::MAX));
    }

    // ================================================================
    //  Default impl
    // ================================================================

    #[test]
    fn default_document_has_zero_identifiers_and_none_fields() {
        let doc = DocumentV0::default();
        assert_eq!(doc.id, Identifier::new([0u8; 32]));
        assert_eq!(doc.owner_id, Identifier::new([0u8; 32]));
        assert!(doc.properties.is_empty());
        assert_eq!(doc.revision, None);
        assert_eq!(doc.created_at, None);
        assert_eq!(doc.updated_at, None);
        assert_eq!(doc.transferred_at, None);
        assert_eq!(doc.creator_id, None);
    }

    // ================================================================
    //  PartialEq semantics
    // ================================================================

    #[test]
    fn documents_with_different_creator_id_are_not_equal() {
        let a = minimal_doc();
        let mut b = minimal_doc();
        b.creator_id = Some(Identifier::new([7u8; 32]));
        assert_ne!(a, b);
    }

    #[test]
    fn documents_with_equal_fields_are_equal() {
        let a = minimal_doc();
        let b = minimal_doc();
        assert_eq!(a, b);
    }

    #[test]
    fn clone_produces_equal_document() {
        let mut doc = minimal_doc();
        doc.properties.insert("k".to_string(), Value::U64(42));
        doc.revision = Some(3);
        let cloned = doc.clone();
        assert_eq!(doc, cloned);
    }
}
