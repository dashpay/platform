use crate::document::{DocumentV0, DocumentV0Getters, DocumentV0Setters};
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentV0Getters for DocumentV0 {
    fn id(&self) -> Identifier {
        self.id
    }

    fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    fn properties(&self) -> &BTreeMap<String, Value> {
        &self.properties
    }

    fn properties_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.properties
    }

    fn revision(&self) -> Option<Revision> {
        self.revision
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        self.created_at
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        self.updated_at
    }

    fn id_ref(&self) -> &Identifier {
        &self.id
    }

    fn owner_id_ref(&self) -> &Identifier {
        &self.owner_id
    }

    fn properties_consumed(self) -> BTreeMap<String, Value> {
        self.properties
    }
}

impl DocumentV0Setters for DocumentV0 {
    fn set_id(&mut self, id: Identifier) {
        self.id = id;
    }

    fn set_owner_id(&mut self, owner_id: Identifier) {
        self.owner_id = owner_id;
    }

    fn set_properties(&mut self, properties: BTreeMap<String, Value>) {
        self.properties = properties;
    }

    fn set_revision(&mut self, revision: Option<Revision>) {
        self.revision = revision;
    }

    fn set_created_at(&mut self, created_at: Option<TimestampMillis>) {
        self.created_at = created_at;
    }

    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        self.updated_at = updated_at;
    }
}
