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
}
