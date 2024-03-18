use platform_value::btreemap_extensions::{
    BTreeValueMapInsertionPathHelper, BTreeValueMapPathHelper,
};
use platform_value::Value;
use std::collections::BTreeMap;

use crate::identity::TimestampMillis;
use crate::prelude::Identifier;
use crate::prelude::Revision;

pub trait DocumentV0Getters {
    /// Returns the unique document ID.
    fn id(&self) -> Identifier;

    /// Returns the ID of the document's owner.
    fn owner_id(&self) -> Identifier;

    /// Returns the document's properties (data).
    fn properties(&self) -> &BTreeMap<String, Value>;

    /// Returns a mutable reference to the document's properties (data).
    fn properties_mut(&mut self) -> &mut BTreeMap<String, Value>;

    /// Returns the document revision.
    fn revision(&self) -> Option<Revision>;

    /// Returns the time in milliseconds that the document was created.
    fn created_at(&self) -> Option<TimestampMillis>;

    /// Returns the time in milliseconds that the document was last updated.
    fn updated_at(&self) -> Option<TimestampMillis>;

    /// Retrieves the field specified by the path.
    /// Returns `None` if the path is empty or if the field is not present.
    fn get(&self, path: &str) -> Option<&Value> {
        self.properties().get_optional_at_path(path).ok().flatten()
    }
    fn id_ref(&self) -> &Identifier;
    fn owner_id_ref(&self) -> &Identifier;
    fn properties_consumed(self) -> BTreeMap<String, Value>;
    fn created_at_block_height(&self) -> Option<u64>;
    fn updated_at_block_height(&self) -> Option<u64>;
    fn created_at_core_block_height(&self) -> Option<u32>;
    fn updated_at_core_block_height(&self) -> Option<u32>;
}

pub trait DocumentV0Setters: DocumentV0Getters {
    /// Sets the unique document ID.
    fn set_id(&mut self, id: Identifier);

    /// Sets the ID of the document's owner.
    fn set_owner_id(&mut self, owner_id: Identifier);

    /// Sets the document's properties (data).
    fn set_properties(&mut self, properties: BTreeMap<String, Value>);

    /// Sets the document revision.
    fn set_revision(&mut self, revision: Option<Revision>);

    /// Sets the time in milliseconds that the document was created.
    fn set_created_at(&mut self, created_at: Option<TimestampMillis>);

    /// Sets the time in milliseconds that the document was last updated.
    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>);

    /// Set the value under the given path.
    /// The path supports syntax from the `lodash` JS library. Example: "root.people[0].name".
    /// If parents are not present, they will be automatically created.
    fn set(&mut self, path: &str, value: Value) {
        if !path.is_empty() {
            self.properties_mut()
                .insert_at_path(path, value)
                .expect("path should not be empty, we checked");
        }
    }

    /// Sets a `u8` value for the specified property name.
    fn set_u8(&mut self, property_name: &str, value: u8) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::U8(value));
    }

    /// Sets an `i8` value for the specified property name.
    fn set_i8(&mut self, property_name: &str, value: i8) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::I8(value));
    }

    /// Sets a `u16` value for the specified property name.
    fn set_u16(&mut self, property_name: &str, value: u16) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::U16(value));
    }

    /// Sets an `i16` value for the specified property name.
    fn set_i16(&mut self, property_name: &str, value: i16) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::I16(value));
    }

    /// Sets a `u32` value for the specified property name.
    fn set_u32(&mut self, property_name: &str, value: u32) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::U32(value));
    }

    /// Sets an `i32` value for the specified property name.
    fn set_i32(&mut self, property_name: &str, value: i32) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::I32(value));
    }

    /// Sets a `u64` value for the specified property name.
    fn set_u64(&mut self, property_name: &str, value: u64) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::U64(value));
    }

    /// Sets an `i64` value for the specified property name.
    fn set_i64(&mut self, property_name: &str, value: i64) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::I64(value));
    }

    /// Sets a `Vec<u8>` (byte array) value for the specified property name.
    fn set_bytes(&mut self, property_name: &str, value: Vec<u8>) {
        self.properties_mut()
            .insert(property_name.to_string(), Value::Bytes(value));
    }
    fn set_created_at_block_height(&mut self, created_at_block_height: Option<u64>);
    fn set_updated_at_block_height(&mut self, updated_at_block_height: Option<u64>);
    fn set_created_at_core_block_height(&mut self, created_at_core_block_height: Option<u32>);
    fn set_updated_at_core_block_height(&mut self, updated_at_core_block_height: Option<u32>);
}
