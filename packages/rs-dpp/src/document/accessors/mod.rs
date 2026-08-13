pub(crate) mod v0;

pub use v0::*;

use crate::document::Document;
use crate::identity::TimestampMillis;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentV0Getters for Document {
    fn id(&self) -> Identifier {
        match self {
            Document::V0(v0) => v0.id,
        }
    }

    fn id_ref(&self) -> &Identifier {
        match self {
            Document::V0(v0) => &v0.id,
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            Document::V0(v0) => v0.owner_id,
        }
    }

    fn owner_id_ref(&self) -> &Identifier {
        match self {
            Document::V0(v0) => &v0.owner_id,
        }
    }

    fn properties(&self) -> &BTreeMap<String, Value> {
        match self {
            Document::V0(v0) => &v0.properties,
        }
    }

    fn properties_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            Document::V0(v0) => &mut v0.properties,
        }
    }

    fn properties_consumed(self) -> BTreeMap<String, Value> {
        match self {
            Document::V0(v0) => v0.properties,
        }
    }

    fn revision(&self) -> Option<Revision> {
        match self {
            Document::V0(v0) => v0.revision,
        }
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            Document::V0(v0) => v0.created_at,
        }
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            Document::V0(v0) => v0.updated_at,
        }
    }

    fn transferred_at(&self) -> Option<TimestampMillis> {
        match self {
            Document::V0(v0) => v0.transferred_at,
        }
    }

    fn created_at_block_height(&self) -> Option<BlockHeight> {
        match self {
            Document::V0(v0) => v0.created_at_block_height,
        }
    }

    fn updated_at_block_height(&self) -> Option<BlockHeight> {
        match self {
            Document::V0(v0) => v0.updated_at_block_height,
        }
    }

    fn transferred_at_block_height(&self) -> Option<BlockHeight> {
        match self {
            Document::V0(v0) => v0.transferred_at_block_height,
        }
    }

    fn created_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        match self {
            Document::V0(v0) => v0.created_at_core_block_height,
        }
    }

    fn updated_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        match self {
            Document::V0(v0) => v0.updated_at_core_block_height,
        }
    }

    fn transferred_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        match self {
            Document::V0(v0) => v0.transferred_at_core_block_height,
        }
    }

    fn creator_id(&self) -> Option<Identifier> {
        match self {
            Document::V0(v0) => v0.creator_id,
        }
    }
}

impl DocumentV0Setters for Document {
    fn set_id(&mut self, id: Identifier) {
        match self {
            Document::V0(v0) => v0.id = id,
        }
    }

    fn set_owner_id(&mut self, owner_id: Identifier) {
        match self {
            Document::V0(v0) => v0.owner_id = owner_id,
        }
    }

    fn set_properties(&mut self, properties: BTreeMap<String, Value>) {
        match self {
            Document::V0(v0) => v0.properties = properties,
        }
    }

    fn set_revision(&mut self, revision: Option<Revision>) {
        match self {
            Document::V0(v0) => v0.revision = revision,
        }
    }

    fn bump_revision(&mut self) {
        match self {
            Document::V0(v0) => v0.bump_revision(),
        }
    }

    fn set_created_at(&mut self, created_at: Option<TimestampMillis>) {
        match self {
            Document::V0(v0) => v0.created_at = created_at,
        }
    }

    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        match self {
            Document::V0(v0) => v0.updated_at = updated_at,
        }
    }

    fn set_transferred_at(&mut self, transferred_at: Option<TimestampMillis>) {
        match self {
            Document::V0(v0) => v0.transferred_at = transferred_at,
        }
    }

    fn set_created_at_block_height(&mut self, created_at_block_height: Option<u64>) {
        match self {
            Document::V0(v0) => v0.created_at_block_height = created_at_block_height,
        }
    }

    fn set_updated_at_block_height(&mut self, updated_at_block_height: Option<u64>) {
        match self {
            Document::V0(v0) => v0.updated_at_block_height = updated_at_block_height,
        }
    }

    fn set_transferred_at_block_height(&mut self, transferred_at_block_height: Option<u64>) {
        match self {
            Document::V0(v0) => v0.transferred_at_block_height = transferred_at_block_height,
        }
    }

    fn set_created_at_core_block_height(&mut self, created_at_core_block_height: Option<u32>) {
        match self {
            Document::V0(v0) => v0.created_at_core_block_height = created_at_core_block_height,
        }
    }

    fn set_updated_at_core_block_height(&mut self, updated_at_core_block_height: Option<u32>) {
        match self {
            Document::V0(v0) => v0.updated_at_core_block_height = updated_at_core_block_height,
        }
    }

    fn set_transferred_at_core_block_height(
        &mut self,
        transferred_at_core_block_height: Option<u32>,
    ) {
        match self {
            Document::V0(v0) => {
                v0.transferred_at_core_block_height = transferred_at_core_block_height
            }
        }
    }

    fn set_creator_id(&mut self, creator_id: Option<Identifier>) {
        match self {
            Document::V0(v0) => v0.creator_id = creator_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::v0::DocumentV0;
    use crate::prelude::Revision;

    fn make_doc() -> Document {
        Document::V0(DocumentV0 {
            contract_version: None,
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties: BTreeMap::new(),
            revision: Some(1),
            created_at: Some(10),
            updated_at: Some(20),
            transferred_at: Some(30),
            created_at_block_height: Some(100),
            updated_at_block_height: Some(200),
            transferred_at_block_height: Some(300),
            created_at_core_block_height: Some(1),
            updated_at_core_block_height: Some(2),
            transferred_at_core_block_height: Some(3),
            creator_id: Some(Identifier::new([9u8; 32])),
        })
    }

    // ================================================================
    //  Document-enum getters forward to the V0 variant.
    // ================================================================

    #[test]
    fn getters_forward_all_fields_to_v0() {
        let doc = make_doc();
        assert_eq!(doc.id(), Identifier::new([1u8; 32]));
        assert_eq!(doc.owner_id(), Identifier::new([2u8; 32]));
        assert_eq!(doc.id_ref(), &Identifier::new([1u8; 32]));
        assert_eq!(doc.owner_id_ref(), &Identifier::new([2u8; 32]));
        assert_eq!(doc.revision(), Some(1));
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
        assert!(doc.properties().is_empty());
    }

    // ================================================================
    //  Document-enum properties_consumed returns the inner BTreeMap.
    // ================================================================

    #[test]
    fn properties_consumed_forwards_to_v0() {
        let Document::V0(mut v0) = make_doc();
        v0.properties.insert("k".into(), Value::U64(77));
        let doc = Document::V0(v0);
        let map = doc.properties_consumed();
        assert_eq!(map.get("k"), Some(&Value::U64(77)));
    }

    // ================================================================
    //  Document-enum properties_mut allows mutation.
    // ================================================================

    #[test]
    fn properties_mut_allows_mutation_via_enum() {
        let mut doc = make_doc();
        doc.properties_mut().insert("x".into(), Value::U64(5));
        assert_eq!(doc.properties().get("x"), Some(&Value::U64(5)));
    }

    // ================================================================
    //  Document-enum setters forward to the V0 variant.
    // ================================================================

    #[test]
    fn setters_forward_all_fields_to_v0() {
        let mut doc = make_doc();
        doc.set_id(Identifier::new([42u8; 32]));
        doc.set_owner_id(Identifier::new([43u8; 32]));
        let mut p = BTreeMap::new();
        p.insert("z".into(), Value::Bool(true));
        doc.set_properties(p.clone());
        doc.set_revision(Some(99));
        doc.set_created_at(Some(1_000_000));
        doc.set_updated_at(Some(2_000_000));
        doc.set_transferred_at(Some(3_000_000));
        doc.set_created_at_block_height(Some(111));
        doc.set_updated_at_block_height(Some(222));
        doc.set_transferred_at_block_height(Some(333));
        doc.set_created_at_core_block_height(Some(4));
        doc.set_updated_at_core_block_height(Some(5));
        doc.set_transferred_at_core_block_height(Some(6));
        doc.set_creator_id(Some(Identifier::new([7u8; 32])));

        assert_eq!(doc.id(), Identifier::new([42u8; 32]));
        assert_eq!(doc.owner_id(), Identifier::new([43u8; 32]));
        assert_eq!(doc.properties(), &p);
        assert_eq!(doc.revision(), Some(99));
        assert_eq!(doc.created_at(), Some(1_000_000));
        assert_eq!(doc.updated_at(), Some(2_000_000));
        assert_eq!(doc.transferred_at(), Some(3_000_000));
        assert_eq!(doc.created_at_block_height(), Some(111));
        assert_eq!(doc.updated_at_block_height(), Some(222));
        assert_eq!(doc.transferred_at_block_height(), Some(333));
        assert_eq!(doc.created_at_core_block_height(), Some(4));
        assert_eq!(doc.updated_at_core_block_height(), Some(5));
        assert_eq!(doc.transferred_at_core_block_height(), Some(6));
        assert_eq!(doc.creator_id(), Some(Identifier::new([7u8; 32])));
    }

    // ================================================================
    //  Document-enum bump_revision forwards to the V0 saturating_add.
    //  Unlike increment_revision, bump_revision never errors and
    //  saturates at Revision::MAX.
    // ================================================================

    #[test]
    fn bump_revision_via_enum_saturates_at_max() {
        let mut doc = make_doc();
        doc.set_revision(Some(Revision::MAX));
        doc.bump_revision();
        assert_eq!(doc.revision(), Some(Revision::MAX));
    }

    #[test]
    fn bump_revision_via_enum_is_noop_for_none() {
        let mut doc = make_doc();
        doc.set_revision(None);
        doc.bump_revision();
        assert_eq!(doc.revision(), None);
    }

    // ================================================================
    //  Document::get (default impl on the trait) returns a property
    //  at the given path or None.
    // ================================================================

    #[test]
    fn get_returns_property_by_path() {
        let mut doc = make_doc();
        doc.properties_mut().insert("a".into(), Value::U64(1));
        assert_eq!(doc.get("a"), Some(&Value::U64(1)));
        assert_eq!(doc.get("missing"), None);
    }
}
