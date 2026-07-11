//! Documents.
//!
//! This module defines the `Document` struct and implements its functions.
//!

mod accessors;
#[cfg(feature = "document-cbor-conversion")]
pub(super) mod cbor_conversion;
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
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct DocumentV0 {
    /// The unique document ID.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$id"))]
    pub id: Identifier,
    /// The ID of the document's owner.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$ownerId"))]
    pub owner_id: Identifier,
    /// The document's properties (data).
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub properties: BTreeMap<String, Value>,
    /// The document revision, if the document is mutable.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$revision", default))]
    pub revision: Option<Revision>,
    /// The time in milliseconds that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$createdAt", default))]
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$updatedAt", default))]
    pub updated_at: Option<TimestampMillis>,
    /// The time in milliseconds that the document was last transferred, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$transferredAt", default)
    )]
    pub transferred_at: Option<TimestampMillis>,
    /// The block that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$createdAtBlockHeight", default)
    )]
    pub created_at_block_height: Option<BlockHeight>,
    /// The block that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$updatedAtBlockHeight", default)
    )]
    pub updated_at_block_height: Option<BlockHeight>,
    /// The block that the document was last transferred to a new identity, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$transferredAtBlockHeight", default)
    )]
    pub transferred_at_block_height: Option<BlockHeight>,
    /// The core block that the document was created, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$createdAtCoreBlockHeight", default)
    )]
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block that the document was last updated, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$updatedAtCoreBlockHeight", default)
    )]
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    /// The core block that the document was last transferred to a new identity, if it is set as required by the document type schema.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$transferredAtCoreBlockHeight", default)
    )]
    pub transferred_at_core_block_height: Option<CoreBlockHeight>,
    /// The creator id.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$creatorId", default))]
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
    use crate::data_contract::accessors::v0::DataContractV0Getters;
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

    // ================================================================
    //  Display impl: properties ordering and mixed fields
    // ================================================================

    #[test]
    fn display_writes_properties_in_btreemap_sorted_order() {
        // BTreeMap iterates in sorted key order. Verify the Display impl
        // (which delegates to self.properties.iter()) emits the keys in that
        // order. This exercises the properties-iteration branch of Display
        // with more than one property.
        let mut doc = minimal_doc();
        doc.properties
            .insert("zebra".to_string(), Value::Text("z".into()));
        doc.properties
            .insert("apple".to_string(), Value::Text("a".into()));
        doc.properties
            .insert("mango".to_string(), Value::Text("m".into()));

        let s = format!("{}", doc);
        let apple_idx = s.find("apple:").expect("apple missing");
        let mango_idx = s.find("mango:").expect("mango missing");
        let zebra_idx = s.find("zebra:").expect("zebra missing");
        assert!(
            apple_idx < mango_idx && mango_idx < zebra_idx,
            "properties should appear in sorted (BTreeMap) order: {s}"
        );
    }

    #[test]
    fn display_mixes_system_fields_and_user_properties() {
        // Exercise Display with only some optional system fields set,
        // plus a property. Different combo than prior tests so we hit
        // the transition from "system optional Some arm" to "properties
        // iteration arm".
        let mut doc = minimal_doc();
        doc.revision = Some(42);
        doc.created_at_block_height = Some(7);
        doc.properties
            .insert("greeting".to_string(), Value::Text("hi".into()));

        let s = format!("{}", doc);
        assert!(s.contains("created_at_block_height:7"));
        assert!(s.contains("greeting:"));
        // revision is NOT rendered by Display (only system timestamps +
        // properties are). Verify Display does not add spurious revision text.
        assert!(!s.contains("revision"));
    }

    // ================================================================
    //  Hash method: from the DocumentHashV0Method trait, which is the
    //  empty impl on DocumentV0 that forwards to hash_v0. Exercises a
    //  code path not covered by accessor-only tests.
    // ================================================================

    #[test]
    fn hash_v0_produces_deterministic_output_for_identical_documents() {
        use crate::document::document_methods::DocumentHashV0Method;
        use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        use crate::tests::json_document::json_document_to_contract;
        use platform_version::version::PlatformVersion;

        // hash_v0 is the default-method impl on DocumentV0 (via empty impl
        // block). It requires a contract + document type to hash through.
        let platform_version = PlatformVersion::first();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load family contract");
        let doc_type = contract
            .document_type_for_name("person")
            .expect("expected person type");

        // Build a document that can be serialized under this type.
        use crate::data_contract::document_type::random_document::CreateRandomDocument;
        let document = doc_type
            .random_document(Some(7), platform_version)
            .expect("random document");
        let doc_v0 = match &document {
            crate::document::Document::V0(d) => d.clone(),
        };

        // Determinism: hashing the same document twice must produce equal bytes.
        let h1 = doc_v0
            .hash_v0(&contract, doc_type, platform_version)
            .expect("hash succeeds");
        let h2 = doc_v0
            .hash_v0(&contract, doc_type, platform_version)
            .expect("hash succeeds");
        assert_eq!(h1, h2);
        // The double-SHA256 result is 32 bytes.
        assert_eq!(h1.len(), 32);

        // And sanity: the hash must differ from the plain serialized bytes
        // — i.e. the impl actually hashes, it doesn't just forward serialize().
        let serialized = doc_v0
            .serialize(doc_type, &contract, platform_version)
            .expect("serialize");
        assert_ne!(h1, serialized);
    }

    #[test]
    fn hash_v0_differs_between_different_documents() {
        use crate::document::document_methods::DocumentHashV0Method;
        use crate::tests::json_document::json_document_to_contract;
        use platform_version::version::PlatformVersion;

        let platform_version = PlatformVersion::first();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            false,
            platform_version,
        )
        .expect("family contract");
        let doc_type = contract
            .document_type_for_name("person")
            .expect("person type");

        use crate::data_contract::document_type::random_document::CreateRandomDocument;
        let crate::document::Document::V0(doc_a) = doc_type
            .random_document(Some(1), platform_version)
            .expect("random a");
        let crate::document::Document::V0(doc_b) = doc_type
            .random_document(Some(2), platform_version)
            .expect("random b");

        let h_a = doc_a
            .hash_v0(&contract, doc_type, platform_version)
            .expect("hash a");
        let h_b = doc_b
            .hash_v0(&contract, doc_type, platform_version)
            .expect("hash b");
        assert_ne!(h_a, h_b);
    }

    // ================================================================
    //  PartialEq: individually flip each field and assert inequality.
    //  Exercises the derived PartialEq arm comparisons field-by-field.
    // ================================================================

    #[test]
    fn not_equal_when_revision_differs() {
        let a = minimal_doc();
        let mut b = minimal_doc();
        b.revision = Some(1);
        assert_ne!(a, b);
    }

    #[test]
    fn not_equal_when_each_timestamp_differs() {
        let a = minimal_doc();

        let mut b = minimal_doc();
        b.created_at = Some(1);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.updated_at = Some(2);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.transferred_at = Some(3);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.created_at_block_height = Some(4);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.updated_at_block_height = Some(5);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.transferred_at_block_height = Some(6);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.created_at_core_block_height = Some(7);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.updated_at_core_block_height = Some(8);
        assert_ne!(a, b);

        let mut b = minimal_doc();
        b.transferred_at_core_block_height = Some(9);
        assert_ne!(a, b);
    }

    #[test]
    fn not_equal_when_properties_differ() {
        let a = minimal_doc();
        let mut b = minimal_doc();
        b.properties.insert("foo".to_string(), Value::U64(1));
        assert_ne!(a, b);
    }

    #[test]
    fn not_equal_when_id_differs() {
        let a = minimal_doc();
        let mut b = minimal_doc();
        b.id = Identifier::new([99u8; 32]);
        assert_ne!(a, b);
    }

    #[test]
    fn not_equal_when_owner_id_differs() {
        let a = minimal_doc();
        let mut b = minimal_doc();
        b.owner_id = Identifier::new([98u8; 32]);
        assert_ne!(a, b);
    }

    // ================================================================
    //  bump_revision: additional edge cases — starting at 0, and at
    //  MAX-1 → MAX → MAX (saturating).
    // ================================================================

    #[test]
    fn bump_revision_from_zero_increments_to_one() {
        let mut doc = minimal_doc();
        doc.set_revision(Some(0));
        doc.bump_revision();
        assert_eq!(doc.revision(), Some(1));
    }

    #[test]
    fn bump_revision_from_max_minus_one_reaches_max_then_saturates() {
        let mut doc = minimal_doc();
        doc.set_revision(Some(Revision::MAX - 1));
        doc.bump_revision();
        assert_eq!(doc.revision(), Some(Revision::MAX));
        doc.bump_revision();
        assert_eq!(doc.revision(), Some(Revision::MAX));
        // one more to make absolutely sure saturating_add really did saturate.
        doc.bump_revision();
        assert_eq!(doc.revision(), Some(Revision::MAX));
    }

    // ================================================================
    //  Default + setters: mutate each setter and ensure the getter round-trips.
    //  Exercises Setter::set_* arms that might otherwise not be executed.
    // ================================================================

    #[test]
    fn setters_round_trip_every_field() {
        use crate::document::{DocumentV0Getters, DocumentV0Setters};
        let mut doc = DocumentV0::default();
        doc.set_id(Identifier::new([1u8; 32]));
        doc.set_owner_id(Identifier::new([2u8; 32]));
        let mut props = BTreeMap::new();
        props.insert("a".to_string(), Value::U64(99));
        doc.set_properties(props.clone());
        doc.set_revision(Some(4));
        doc.set_created_at(Some(10));
        doc.set_updated_at(Some(20));
        doc.set_transferred_at(Some(30));
        doc.set_created_at_block_height(Some(100));
        doc.set_updated_at_block_height(Some(200));
        doc.set_transferred_at_block_height(Some(300));
        doc.set_created_at_core_block_height(Some(1));
        doc.set_updated_at_core_block_height(Some(2));
        doc.set_transferred_at_core_block_height(Some(3));
        doc.set_creator_id(Some(Identifier::new([9u8; 32])));

        assert_eq!(doc.id(), Identifier::new([1u8; 32]));
        assert_eq!(doc.owner_id(), Identifier::new([2u8; 32]));
        assert_eq!(doc.properties(), &props);
        assert_eq!(doc.revision(), Some(4));
        assert_eq!(doc.created_at(), Some(10));
        assert_eq!(doc.updated_at(), Some(20));
        assert_eq!(doc.transferred_at(), Some(30));
        assert_eq!(doc.created_at_block_height(), Some(100));
        assert_eq!(doc.updated_at_block_height(), Some(200));
        assert_eq!(doc.transferred_at_block_height(), Some(300));
        assert_eq!(doc.created_at_core_block_height(), Some(1));
        assert_eq!(doc.updated_at_core_block_height(), Some(2));
        assert_eq!(doc.transferred_at_core_block_height(), Some(3));
        assert_eq!(doc.creator_id(), Some(Identifier::new([9u8; 32])));

        // id_ref, owner_id_ref and properties_consumed exercise separate
        // methods on DocumentV0Getters.
        assert_eq!(doc.id_ref(), &Identifier::new([1u8; 32]));
        assert_eq!(doc.owner_id_ref(), &Identifier::new([2u8; 32]));
        assert_eq!(doc.clone().properties_consumed(), props);
    }

    // ================================================================
    //  properties_mut actually allows mutation (exercises the &mut accessor
    //  arm, not just the immutable getter).
    // ================================================================

    #[test]
    fn properties_mut_allows_inserting_new_key() {
        use crate::document::DocumentV0Getters;
        let mut doc = minimal_doc();
        doc.properties_mut().insert("k".into(), Value::U64(7));
        assert_eq!(doc.properties().get("k"), Some(&Value::U64(7)));
    }

    // ================================================================
    //  Debug impl: should include field names so tracing messages print
    //  reasonable output (covers the auto-derived Debug arm without
    //  duplicating other checks).
    // ================================================================

    #[test]
    fn debug_format_contains_field_names() {
        let doc = minimal_doc();
        let dbg = format!("{:?}", doc);
        assert!(dbg.contains("DocumentV0"), "expected struct name in Debug");
        assert!(dbg.contains("id"));
        assert!(dbg.contains("owner_id"));
    }

    // ================================================================
    //  Display with transferred_at_core_block_height and creator_id set
    //  but other transferred fields None: exercises the "Some(creator_id)
    //  AFTER several optional system fields that ARE None" path.
    // ================================================================

    #[test]
    fn display_with_only_creator_id_and_no_timestamps() {
        let mut doc = minimal_doc();
        doc.creator_id = Some(Identifier::new([7u8; 32]));
        let s = format!("{}", doc);
        assert!(s.contains("creator_id:"));
        // No timestamp prefix should be rendered.
        assert!(!s.contains("created_at:"));
        assert!(!s.contains("updated_at:"));
        assert!(!s.contains("transferred_at:"));
        // With empty properties, the "no properties" trailer kicks in.
        assert!(s.contains("no properties"));
    }
}
