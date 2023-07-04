use crate::document::Document;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl Document {
    /// Returns a reference to the unique document ID.
    pub fn id(&self) -> Identifier {
        match self {
            Document::V0(v0) => v0.id,
        }
    }

    /// Sets the unique document ID.
    pub fn set_id(&mut self, id: Identifier) {
        match self {
            Document::V0(v0) => v0.id = id,
        }
    }

    /// Returns a reference to the ID of the document's owner.
    pub fn owner_id(&self) -> Identifier {
        match self {
            Document::V0(v0) => v0.owner_id,
        }
    }

    /// Sets the ID of the document's owner.
    pub fn set_owner_id(&mut self, owner_id: Identifier) {
        match self {
            Document::V0(v0) => v0.owner_id = owner_id,
        }
    }

    /// Returns a reference to the document's properties (data).
    pub fn properties(&self) -> &BTreeMap<String, Value> {
        match self {
            Document::V0(v0) => &v0.properties,
        }
    }

    /// Sets the document's properties (data).
    pub fn set_properties(&mut self, properties: BTreeMap<String, Value>) {
        match self {
            Document::V0(v0) => v0.properties = properties,
        }
    }

    /// Returns a reference to the document revision.
    pub fn revision(&self) -> Option<Revision> {
        match self {
            Document::V0(v0) => v0.revision,
        }
    }

    /// Sets the document revision.
    pub fn set_revision(&mut self, revision: Option<Revision>) {
        match self {
            Document::V0(v0) => v0.revision = revision,
        }
    }

    /// Returns a reference to the document creation time in milliseconds.
    pub fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            Document::V0(v0) => v0.created_at,
        }
    }

    /// Sets the document creation time in milliseconds.
    pub fn set_created_at(&mut self, created_at: Option<TimestampMillis>) {
        match self {
            Document::V0(v0) => v0.created_at = created_at,
        }
    }

    /// Returns a reference to the document update time in milliseconds.
    pub fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            Document::V0(v0) => v0.updated_at,
        }
    }

    /// Sets the document update time in milliseconds.
    pub fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        match self {
            Document::V0(v0) => v0.updated_at = updated_at,
        }
    }

    /// Set the value under the given path.
    /// The path supports syntax from the `lodash` JS library. Example: "root.people[0].name".
    /// If parents are not present, they will be automatically created.
    pub fn set(&mut self, path: &str, value: Value) {
        self.properties().insert(path.to_string(), value);
    }

    /// Retrieves the field specified by the path.
    /// Returns `None` if the path is empty or if the field is not present.
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.properties().get_optional_at_path(path).ok().flatten()
    }

    /// Sets a `u8` value for the specified property name.
    pub fn set_u8(&mut self, property_name: &str, value: u8) {
        self.properties()
            .insert(property_name.to_string(), Value::U8(value));
    }

    /// Sets an `i8` value for the specified property name.
    pub fn set_i8(&mut self, property_name: &str, value: i8) {
        self.properties()
            .insert(property_name.to_string(), Value::I8(value));
    }

    /// Sets a `u16` value for the specified property name.
    pub fn set_u16(&mut self, property_name: &str, value: u16) {
        self.properties()
            .insert(property_name.to_string(), Value::U16(value));
    }

    /// Sets an `i16` value for the specified property name.
    pub fn set_i16(&mut self, property_name: &str, value: i16) {
        self.properties()
            .insert(property_name.to_string(), Value::I16(value));
    }

    /// Sets a `u32` value for the specified property name.
    pub fn set_u32(&mut self, property_name: &str, value: u32) {
        self.properties()
            .insert(property_name.to_string(), Value::U32(value));
    }

    /// Sets an `i32` value for the specified property name.
    pub fn set_i32(&mut self, property_name: &str, value: i32) {
        self.properties()
            .insert(property_name.to_string(), Value::I32(value));
    }

    /// Sets a `u64` value for the specified property name.
    pub fn set_u64(&mut self, property_name: &str, value: u64) {
        self.properties()
            .insert(property_name.to_string(), Value::U64(value));
    }

    /// Sets an `i64` value for the specified property name.
    pub fn set_i64(&mut self, property_name: &str, value: i64) {
        self.properties()
            .insert(property_name.to_string(), Value::I64(value));
    }

    /// Sets a `Vec<u8>` (byte array) value for the specified property name.
    pub fn set_bytes(&mut self, property_name: &str, value: Vec<u8>) {
        self.properties()
            .insert(property_name.to_string(), Value::Bytes(value));
    }
}
